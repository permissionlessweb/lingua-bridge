# LinguaBridge Voice Inference Pipeline

Technical documentation for the real-time voice translation system.

## Architecture Overview

```
Discord Voice Channel
        │
        │ Opus (20ms frames)
        ▼
┌──────────────────────────────────────────────────────────────┐
│  Rust Bot (Songbird)                                         │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────────┐    │
│  │   Voice     │──▶│   Audio     │──▶│   WebSocket     │    │
│  │   Handler   │   │   Buffer    │   │   Client        │    │
│  │             │   │   Manager   │   │                 │    │
│  └─────────────┘   └─────────────┘   └────────┬────────┘    │
│        ▲                                       │             │
│        │ SSRC → User ID mapping                │             │
└────────┼───────────────────────────────────────┼─────────────┘
         │                                       │
         │                                       │ JSON/WebSocket
         │                                       ▼
┌────────┴───────────────────────────────────────────────────────┐
│  Python Voice Inference Service                                 │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────┐ │
│  │ Distil-Whisper  │─▶│  TranslateGemma  │─▶│   CosyVoice    │ │
│  │ (STT)           │  │  (Translation)   │  │   (TTS)        │ │
│  └─────────────────┘  └──────────────────┘  └────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  Voice Bridge (src/voice/bridge.rs)                             │
│  Routes results to multiple destinations                        │
└────────────────────────────┬────────────────────────────────────┘
                             │
         +-------------------+-------------------+
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────┐
│ Voice Web View  │ │ Discord Thread  │ │ TTS Playback (optional) │
│ /voice/{g}/{c}  │ │ Transcripts     │ │ (voice channel audio)   │
│ Real-time feed  │ │ Per-language    │ │                         │
│ + TTS toggle    │ │ threads         │ │                         │
└─────────────────┘ └─────────────────┘ └─────────────────────────┘
```

---

## User Identification & Tracking

### How the Bot Knows Who Is Speaking

Discord voice uses **SSRC (Synchronization Source)** identifiers for each audio stream. The bot maps these to Discord user IDs through the following mechanism:

#### 1. Speaking State Updates

When a user starts speaking, Discord sends a `SpeakingStateUpdate` event containing:

| Field | Description |
|-------|-------------|
| `ssrc` | Unique 32-bit audio stream identifier |
| `user_id` | Discord user ID (`u64`) |
| `speaking` | Boolean flags (microphone, soundshare, priority) |

**Source:** `src/voice/handler.rs:73-95`

```rust
EventContext::SpeakingStateUpdate(Speaking { speaking, ssrc, user_id, .. }) => {
    if let Some(user_id) = user_id {
        let user_id_u64: u64 = user_id.0;
        // Register mapping in buffer manager
        self.buffer_manager.register_speaker(*ssrc, user_id_u64, username).await;
    }
}
```

#### 2. SSRC → User Mapping Storage

The `AudioBufferManager` maintains two concurrent hash maps:

```rust
// src/voice/buffer.rs:157-166
pub struct AudioBufferManager {
    /// SSRC -> user buffer mapping
    buffers: Arc<RwLock<HashMap<Ssrc, UserBuffer>>>,
    /// SSRC -> (User ID, Username) mapping
    ssrc_map: Arc<RwLock<HashMap<Ssrc, (u64, String)>>>,
    guild_id: u64,
    channel_id: u64,
}
```

When audio packets arrive via `VoiceTick`, the SSRC is resolved to a user ID:

```rust
// src/voice/buffer.rs:205-208
pub async fn push_audio(&self, packet: AudioPacket) -> Option<AudioSegment> {
    let ssrc_map = self.ssrc_map.read().await;
    let (user_id, username) = ssrc_map.get(&packet.ssrc)?.clone();
    // Audio is now attributed to this user
}
```

---

## Audio Capture & Buffering

### Discord Audio Format

| Parameter | Value |
|-----------|-------|
| Sample Rate | 48,000 Hz |
| Bit Depth | 16-bit signed integer |
| Channels | Stereo (converted to mono) |
| Frame Duration | 20ms (Opus codec) |
| Samples per Frame | 960 samples (at 48kHz) |

**Source:** `src/voice/types.rs:10-17`

### Voice Activity Detection (VAD)

The system uses **energy-based VAD** to detect speech boundaries:

```rust
// src/voice/buffer.rs:144-154
fn detect_speech(samples: &[i16]) -> bool {
    // Calculate RMS energy
    let sum_squares: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
    let rms = (sum_squares / samples.len() as f64).sqrt() / 32768.0;
    rms > VAD_ENERGY_THRESHOLD  // Default: 0.01
}
```

**VAD Parameters:**

| Parameter | Value | Purpose |
|-----------|-------|---------|
| `MIN_SPEECH_DURATION_MS` | 500ms | Minimum utterance length to process |
| `MAX_SPEECH_DURATION_SECS` | 30s | Force flush to prevent memory buildup |
| `SILENCE_TIMEOUT_MS` | 800ms | Silence duration to end utterance |
| `VAD_ENERGY_THRESHOLD` | 0.01 | RMS energy threshold for speech |

### Per-User Audio Buffers

Each user has an independent buffer that accumulates audio:

```rust
// src/voice/buffer.rs:23-41
struct UserBuffer {
    user_id: u64,
    username: String,
    guild_id: u64,
    channel_id: u64,
    samples: Vec<i16>,           // Accumulated PCM samples
    speech_start: Option<Instant>, // When this utterance began
    last_audio_time: Instant,    // Last received audio
    is_speaking: bool,           // Currently speaking?
}
```

**Flush Conditions:**
1. Silence timeout reached (800ms) AND minimum duration met (500ms)
2. Maximum duration reached (30 seconds)
3. User disconnects from voice channel

---

## WebSocket Protocol

### Rust Bot → Python Service

Audio segments are sent as JSON over WebSocket:

```json
{
    "type": "Audio",
    "guild_id": "123456789",
    "channel_id": "987654321",
    "user_id": "111222333",
    "username": "JohnDoe",
    "audio_base64": "<PCM i16 LE, 48kHz, mono>",
    "sample_rate": 48000,
    "target_language": "en",
    "generate_tts": false
}
```

**Source:** `src/voice/types.rs:146-176`

### Python Service → Rust Bot

Results are returned as JSON:

```json
{
    "type": "Result",
    "guild_id": "123456789",
    "channel_id": "987654321",
    "user_id": "111222333",
    "username": "JohnDoe",
    "original_text": "Hola, como estas?",
    "translated_text": "Hello, how are you?",
    "source_language": "es",
    "target_language": "en",
    "tts_audio": "<base64 WAV, 24kHz>",
    "latency_ms": 450
}
```

**Source:** `src/voice/types.rs:178-221`

### Connection Management

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `reconnect_delay` | 2s × attempt | Exponential backoff |
| `max_reconnect_attempts` | 10 | Give up threshold |
| `request_timeout` | 30s | Per-request timeout |
| `ping_interval` | 30s | Keep-alive interval |

**Source:** `src/voice/client.rs:36-46`

---

## Speech-to-Text (STT)

### Model: Distil-Whisper Large V3

The STT engine uses `faster-whisper` with CTranslate2 backend for efficient inference.

**Configuration:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `model_size` | `distil-large-v3` | Whisper model variant |
| `device` | `cuda` | Compute device |
| `compute_type` | `float16` | Precision (CUDA) / `int8` (CPU) |
| `beam_size` | 5 | Beam search width |
| `vad_filter` | `true` | Built-in VAD filtering |

**Source:** `inference/voice/stt.py:50-76`

### Audio Preprocessing

1. **Resampling**: 48kHz → 16kHz (Whisper's expected rate)
2. **Normalization**: i16 → float32 normalized to [-1.0, 1.0]
3. **VAD Filtering**: Remove silence with 500ms minimum duration

```python
# inference/voice/stt.py:148-159
if sample_rate != SAMPLE_RATE:
    audio = self._resample(audio, sample_rate, SAMPLE_RATE)

if audio.dtype != np.float32:
    audio = audio.astype(np.float32)

if audio.max() > 1.0:
    audio = audio / 32768.0
```

### Language Detection

Whisper automatically detects the source language and returns:

| Field | Description |
|-------|-------------|
| `language` | ISO 639-1 code (e.g., "es", "ja") |
| `language_probability` | Confidence score (0.0-1.0) |

### Optional: Speaker Diarization

When enabled, pyannote.audio v3.1 assigns speaker labels:

```python
# inference/voice/stt.py:99-119
self.diarization_pipeline = Pipeline.from_pretrained(
    "pyannote/speaker-diarization-3.1",
    use_auth_token=self.hf_token,
)
```

**Note:** Requires HuggingFace token with pyannote license acceptance.

---

## Translation

### Model: TranslateGemma 4B

Uses the existing text inference service (shared with text channel translation).

**Configuration:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `model_id` | `google/translategemma-4b-it` | Model identifier |
| `device` | `cuda` | Compute device |
| `torch_dtype` | `bfloat16` | Tensor precision |

**Source:** `inference/voice_service.py:97-106`

### Translation Flow

```python
# inference/voice_service.py:276-289
if source_language != target_language and translator is not None:
    translated_text = translator.translate(
        original_text,
        source_lang=source_language,
        target_lang=target_language,
    )
```

**Fallback:** If translation fails, the original text is returned unchanged.

---

## Text-to-Speech (TTS)

### Primary: CosyVoice 2-0.5B

Low-latency multilingual synthesis with optional voice cloning.

**Configuration:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `model_name` | `CosyVoice2-0.5B` | Model variant |
| `device` | `cuda` | Compute device |
| Output Sample Rate | 24,000 Hz | Fixed output rate |

**Source:** `inference/voice/tts.py:37-51`

### Language → Voice Mapping

```python
# inference/voice/tts.py:137-148
voice_map = {
    "en": "English",
    "zh": "Chinese",
    "ja": "Japanese",
    "ko": "Korean",
    "es": "Spanish",
    "fr": "French",
    "de": "German",
    "it": "Italian",
    "pt": "Portuguese",
    "ru": "Russian",
}
```

### Fallback: edge-tts

If CosyVoice is unavailable, Microsoft Edge neural voices are used:

```python
# inference/voice/tts.py:179-191
voice_map = {
    "en": "en-US-AriaNeural",
    "es": "es-ES-ElviraNeural",
    "fr": "fr-FR-DeniseNeural",
    "de": "de-DE-KatjaNeural",
    "ja": "ja-JP-NanamiNeural",
    # ...
}
```

### Audio Encoding

TTS audio is base64-encoded WAV for WebSocket transmission:

```python
# inference/voice/tts.py:227-247
def audio_to_base64(audio: np.ndarray, sample_rate: int) -> str:
    audio_int16 = (audio * 32767).astype(np.int16)
    buffer = io.BytesIO()
    with wave.open(buffer, 'wb') as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)  # 16-bit
        wav.setframerate(sample_rate)
        wav.writeframes(audio_int16.tobytes())
    return base64.b64encode(buffer.read()).decode('utf-8')
```

---

## Configuration Reference

### Voice Config (`src/config.rs:81-126`)

```toml
[voice]
url = "ws://voice-inference:8001/voice"  # Inference service URL
enable_tts_playback = false              # Play TTS in Discord VC
buffer_ms = 500                          # Audio buffer size
vad_threshold = 0.5                      # VAD sensitivity (0.0-1.0)
default_target_language = "en"           # Default translation target
```

### Environment Variables

```bash
# Inference service
DEVICE=cuda                    # cuda | cpu
TORCH_DTYPE=bfloat16          # bfloat16 | float16 | float32

# STT
STT_MODEL=distil-large-v3     # Whisper model
ENABLE_DIARIZATION=false      # Speaker diarization
HF_TOKEN=hf_...               # HuggingFace token (for diarization)

# TTS
TTS_MODEL=CosyVoice2-0.5B     # TTS model
ENABLE_TTS=true               # Enable TTS synthesis

# Translation
TRANSLATEGEMMA_MODEL=google/translategemma-4b-it
```

---

## Data Flow Summary

### Complete Pipeline (per utterance)

```
1. User speaks in Discord voice channel
         │
         ▼
2. Opus audio → Songbird decodes → PCM i16 stereo
         │
         ▼
3. Stereo → Mono conversion (average channels)
         │
         ▼
4. Per-user buffer accumulates samples
         │  VAD monitors for speech boundaries
         ▼
5. Silence detected (800ms) → Buffer flushed
         │
         ▼
6. AudioSegment created with user attribution
         │  • user_id, username
         │  • guild_id, channel_id
         │  • PCM samples (i16, 48kHz, mono)
         ▼
7. Base64 encode → WebSocket → Python service
         │
         ▼
8. Resample 48kHz → 16kHz
         │
         ▼
9. Distil-Whisper transcription
         │  • Language auto-detected
         │  • Segments with timestamps
         ▼
10. TranslateGemma translation (if source != target)
         │
         ▼
11. CosyVoice TTS synthesis (if enabled)
         │
         ▼
12. Result JSON → WebSocket → Rust bot
         │
         ▼
13. Broadcast to:
    • Web client (live transcription feed)
    • Discord text channel (optional)
    • Voice channel TTS playback (optional)
```

---

## Latency Breakdown (Target <500ms)

| Stage | Target | Notes |
|-------|--------|-------|
| Audio capture | 20ms | Single Opus frame |
| VAD + buffering | 50-100ms | Variable based on speech |
| Network (Rust→Python) | 5-10ms | Local container network |
| Resampling | 5ms | 48→16kHz |
| STT (Distil-Whisper) | 200-300ms | Depends on utterance length |
| Translation | 50-100ms | Cached for common phrases |
| TTS (if enabled) | 100-150ms | Streaming chunks |
| Network (Python→Rust) | 5-10ms | JSON response |
| **Total** | **~450-700ms** | Without TTS playback |

---

## Database Schema

Voice channel settings are persisted per-channel:

```sql
CREATE TABLE voice_channel_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    voice_channel_id TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    target_language TEXT NOT NULL DEFAULT 'en',
    enable_tts BOOLEAN NOT NULL DEFAULT false,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(guild_id, voice_channel_id)
);
```

**Source:** `src/db/queries.rs`

---

## Discord Permissions Required

| Permission | Purpose |
|------------|---------|
| Connect | Join voice channels |
| Speak | Play TTS audio |
| Use Voice Activity | Receive audio streams |

---

## Known Limitations

1. **CUDA Required**: GPU inference for real-time performance
2. **Single GPU**: All models share one GPU (no multi-GPU distribution)
3. **No Streaming Decode**: Full utterance processed (not progressive)
4. **Speaker Diarization**: Requires HuggingFace token with license acceptance
5. **Voice Cloning**: CosyVoice cross-lingual cloning requires speaker embedding extraction (not yet implemented)
6. **TTS Playback**: Placeholder in Rust (`src/voice/playback.rs`) - audio mixing not implemented

---

## File Reference

| File | Purpose |
|------|---------|
| `src/voice/handler.rs` | Songbird event handler, SSRC mapping |
| `src/voice/buffer.rs` | Per-user audio buffers, VAD |
| `src/voice/client.rs` | WebSocket client to inference |
| `src/voice/bridge.rs` | Routes results to web clients + Discord threads |
| `src/voice/types.rs` | Shared type definitions |
| `src/voice/playback.rs` | TTS playback (placeholder) |
| `src/web/voice_routes.rs` | Voice web view HTML + WebSocket handlers |
| `src/web/broadcast.rs` | WebSocket broadcast to connected clients |
| `src/db/models.rs` | VoiceTranscriptSettings model |
| `src/db/queries.rs` | VoiceTranscriptRepo CRUD operations |
| `inference/voice/stt.py` | Distil-Whisper engine |
| `inference/voice/tts.py` | CosyVoice/edge-tts engine |
| `inference/voice_service.py` | WebSocket server |
| `src/config.rs` | VoiceConfig definition |

---

## Voice Web View

The voice web view provides a browser-based interface for viewing real-time transcriptions.

### Routes

| Route | Purpose |
|-------|---------|
| `GET /voice/{guild_id}/{channel_id}` | HTML page with transcription UI |
| `GET /voice/{guild_id}/{channel_id}/ws` | WebSocket for real-time updates |

### Features

- **Relative timestamps** - "5 seconds ago", "1 minute ago" (updates every 10s)
- **TTS audio toggle** - Play/pause button with audio queue
- **Volume control** - Slider for TTS playback level
- **Speaker colors** - Consistent colors per user
- **Language badges** - Shows source → target language
- **Auto-reconnection** - Exponential backoff on disconnect

### WebSocket Message Format

Messages sent to web clients:

```json
{
    "type": "voice_transcription",
    "guild_id": "123456789",
    "channel_id": "987654321",
    "user_id": "111222333",
    "username": "JohnDoe",
    "original_text": "Hola, como estas?",
    "translated_text": "Hello, how are you?",
    "source_language": "es",
    "target_language": "en",
    "tts_audio": "<base64 WAV>",
    "timestamp": "2024-01-15T10:30:00Z"
}
```

---

## Discord Thread Transcripts

The bot can post voice transcripts to Discord threads, creating a searchable archive.

### Database Schema

```sql
CREATE TABLE voice_transcript_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    voice_channel_id TEXT NOT NULL,
    text_channel_id TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    languages TEXT NOT NULL DEFAULT '["en"]',
    thread_ids TEXT NOT NULL DEFAULT '{}',
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    UNIQUE(guild_id, voice_channel_id)
);
```

### Thread Management

1. User runs `/voice transcript enable:true languages:en,es,fr`
2. Bot creates threads: "Voice Translation - English", etc.
3. Thread IDs stored in `thread_ids` JSON column
4. `VoiceBridge` posts messages to appropriate threads

### Message Format in Threads

```
**Username**
> Original text in source language
Translated text in target language
```

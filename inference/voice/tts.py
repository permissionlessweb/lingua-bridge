"""
Text-to-Speech module using CosyVoice 2.

Provides multilingual speech synthesis with optional voice cloning.
"""

import io
import logging
import time
from dataclasses import dataclass
from typing import Optional, Tuple

import numpy as np

logger = logging.getLogger(__name__)

# Constants
OUTPUT_SAMPLE_RATE = 24000  # CosyVoice outputs 24kHz


@dataclass
class TTSResult:
    """Text-to-speech synthesis result."""
    audio: np.ndarray
    sample_rate: int
    duration: float
    processing_time_ms: int


class TextToSpeech:
    """
    Text-to-speech engine using CosyVoice 2.

    Supports multilingual synthesis and cross-lingual voice cloning.
    """

    def __init__(
        self,
        model_name: str = "CosyVoice2-0.5B",
        device: str = "cuda",
    ):
        """
        Initialize the TTS engine.

        Args:
            model_name: CosyVoice model variant
            device: Device to run on (cuda, cpu)
        """
        self.model_name = model_name
        self.device = device
        self.model = None
        self.loaded = False

    def load(self):
        """Load the TTS model."""
        logger.info(f"Loading TTS model: {self.model_name}")
        start = time.time()

        try:
            # CosyVoice uses a specific import pattern
            from cosyvoice.cli.cosyvoice import CosyVoice2

            self.model = CosyVoice2(self.model_name)
            self.loaded = True

            load_time = time.time() - start
            logger.info(f"TTS model loaded in {load_time:.2f}s")

        except ImportError:
            logger.warning(
                "CosyVoice not installed. "
                "Falling back to edge-tts for basic synthesis."
            )
            self._setup_fallback()

    def _setup_fallback(self):
        """Set up fallback TTS using edge-tts."""
        try:
            import edge_tts
            self.fallback_mode = True
            self.loaded = True
            logger.info("Using edge-tts as fallback")
        except ImportError:
            logger.error("Neither CosyVoice nor edge-tts available")
            self.loaded = False

    async def synthesize(
        self,
        text: str,
        language: str = "en",
        speaker_embedding: Optional[np.ndarray] = None,
        speed: float = 1.0,
    ) -> TTSResult:
        """
        Synthesize speech from text.

        Args:
            text: Text to synthesize
            language: Target language code
            speaker_embedding: Optional speaker embedding for voice cloning
            speed: Speech speed multiplier (0.5-2.0)

        Returns:
            TTSResult with audio samples
        """
        if not self.loaded:
            raise RuntimeError("TTS model not loaded. Call load() first.")

        start_time = time.time()

        if hasattr(self, 'fallback_mode') and self.fallback_mode:
            audio, sr = await self._synthesize_fallback(text, language)
        else:
            audio, sr = self._synthesize_cosyvoice(
                text, language, speaker_embedding, speed
            )

        processing_time = int((time.time() - start_time) * 1000)
        duration = len(audio) / sr

        return TTSResult(
            audio=audio,
            sample_rate=sr,
            duration=duration,
            processing_time_ms=processing_time,
        )

    def _synthesize_cosyvoice(
        self,
        text: str,
        language: str,
        speaker_embedding: Optional[np.ndarray],
        speed: float,
    ) -> Tuple[np.ndarray, int]:
        """Synthesize using CosyVoice."""
        # Map language codes to CosyVoice voice names
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

        # Use cross-lingual synthesis if speaker embedding provided
        if speaker_embedding is not None:
            # Cross-lingual voice cloning
            for audio_chunk in self.model.inference_cross_lingual(
                text,
                speaker_embedding,
            ):
                # Collect chunks
                pass
        else:
            # Standard synthesis
            voice = voice_map.get(language, "English")
            for audio_chunk in self.model.inference_sft(text, voice):
                pass

        # CosyVoice outputs 24kHz audio
        return audio_chunk, OUTPUT_SAMPLE_RATE

    async def _synthesize_fallback(
        self,
        text: str,
        language: str,
    ) -> Tuple[np.ndarray, int]:
        """Synthesize using edge-tts fallback."""
        import edge_tts
        import tempfile
        import soundfile as sf

        # Map language to edge-tts voice
        voice_map = {
            "en": "en-US-AriaNeural",
            "es": "es-ES-ElviraNeural",
            "fr": "fr-FR-DeniseNeural",
            "de": "de-DE-KatjaNeural",
            "it": "it-IT-ElsaNeural",
            "pt": "pt-BR-FranciscaNeural",
            "ja": "ja-JP-NanamiNeural",
            "ko": "ko-KR-SunHiNeural",
            "zh": "zh-CN-XiaoxiaoNeural",
            "ru": "ru-RU-SvetlanaNeural",
        }

        voice = voice_map.get(language, "en-US-AriaNeural")

        communicate = edge_tts.Communicate(text, voice)

        # Write to temporary file
        with tempfile.NamedTemporaryFile(suffix=".mp3", delete=True) as f:
            await communicate.save(f.name)

            # Read audio
            audio, sr = sf.read(f.name)

        # Convert to mono if stereo
        if len(audio.shape) > 1:
            audio = audio.mean(axis=1)

        return audio.astype(np.float32), sr

    def unload(self):
        """Unload model to free memory."""
        if self.model is not None:
            del self.model
            self.model = None
        self.loaded = False

        import gc
        gc.collect()

        try:
            import torch
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except ImportError:
            pass


def audio_to_base64(audio: np.ndarray, sample_rate: int) -> str:
    """Convert audio array to base64-encoded WAV."""
    import base64
    import io
    import wave
    import struct

    # Convert to int16
    audio_int16 = (audio * 32767).astype(np.int16)

    # Write to WAV
    buffer = io.BytesIO()
    with wave.open(buffer, 'wb') as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)  # 16-bit
        wav.setframerate(sample_rate)
        wav.writeframes(audio_int16.tobytes())

    # Encode to base64
    buffer.seek(0)
    return base64.b64encode(buffer.read()).decode('utf-8')


def base64_to_audio(b64_string: str) -> Tuple[np.ndarray, int]:
    """Decode base64 WAV to audio array."""
    import base64
    import io
    import wave

    audio_bytes = base64.b64decode(b64_string)
    buffer = io.BytesIO(audio_bytes)

    with wave.open(buffer, 'rb') as wav:
        sample_rate = wav.getframerate()
        audio_bytes = wav.readframes(wav.getnframes())

        # Convert to numpy
        audio = np.frombuffer(audio_bytes, dtype=np.int16)
        audio = audio.astype(np.float32) / 32767.0

    return audio, sample_rate

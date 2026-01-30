"""
LinguaBridge Unified Inference Service

FastAPI server providing both REST and WebSocket endpoints for:
- Text translation and language detection (REST)
- Real-time voice translation (WebSocket)

This service consolidates the separate inference and voice-inference services
into a single service to reduce GPU requirements.
"""

import asyncio
import base64
import json
import logging
import os
import time
from contextlib import asynccontextmanager
from typing import Optional

import numpy as np
from dotenv import load_dotenv
from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field

from voice.stt import SpeechToText
from voice.tts import TextToSpeech, audio_to_base64
from translator import TranslateGemmaTranslator
from detector import LanguageDetector
from voice_protocol import (
    parse_binary_frame,
    parse_text_frame,
    create_result_response,
    create_error_response,
    create_pong_response,
    VoiceProtocolError,
)

# Load environment variables
load_dotenv()

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Configuration
TRANSLATE_MODEL = os.getenv("TRANSLATEGEMMA_MODEL", "google/translategemma-4b-it")
STT_MODEL = os.getenv("STT_MODEL", "distil-large-v3")
TTS_MODEL = os.getenv("TTS_MODEL", "CosyVoice2-0.5B")
DEVICE = os.getenv("DEVICE", "cuda")
TORCH_DTYPE = os.getenv("TORCH_DTYPE", "bfloat16")
ENABLE_STT = os.getenv("ENABLE_STT", "true").lower() == "true"
ENABLE_TTS = os.getenv("ENABLE_TTS", "true").lower() == "true"
ENABLE_DIARIZATION = os.getenv("ENABLE_DIARIZATION", "false").lower() == "true"
HF_TOKEN = os.getenv("HF_TOKEN", "")

# Discord audio format
DISCORD_SAMPLE_RATE = 48000

# Global model instances
translator: Optional[TranslateGemmaTranslator] = None
detector: Optional[LanguageDetector] = None
stt: Optional[SpeechToText] = None
tts: Optional[TextToSpeech] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Initialize models on startup."""
    global translator, detector, stt, tts

    logger.info("Starting Unified Inference Service")
    logger.info(f"Translate Model: {TRANSLATE_MODEL}")
    logger.info(f"STT Model: {STT_MODEL} (enabled: {ENABLE_STT})")
    logger.info(f"TTS Model: {TTS_MODEL} (enabled: {ENABLE_TTS})")
    logger.info(f"Device: {DEVICE}, Dtype: {TORCH_DTYPE}")

    # Load translation model (shared by both REST and voice)
    try:
        translator = TranslateGemmaTranslator(
            model_id=TRANSLATE_MODEL,
            device=DEVICE,
            torch_dtype=TORCH_DTYPE
        )
        logger.info("TranslateGemma model loaded successfully")
    except Exception as e:
        logger.error(f"Failed to load TranslateGemma model: {e}")
        translator = None

    # Load language detector (for REST API)
    try:
        detector = LanguageDetector()
        logger.info("Language detector loaded successfully")
    except Exception as e:
        logger.error(f"Failed to load language detector: {e}")
        detector = None

    # Load STT model (for voice)
    if ENABLE_STT:
        try:
            stt = SpeechToText(
                model_size=STT_MODEL,
                device=DEVICE,
                compute_type="float16" if DEVICE == "cuda" else "int8",
                enable_diarization=ENABLE_DIARIZATION,
                hf_token=HF_TOKEN if HF_TOKEN else None,
            )
            stt.load()
            logger.info("STT model loaded successfully")
        except Exception as e:
            logger.error(f"Failed to load STT model: {e}")
            stt = None
    else:
        logger.info("STT disabled by configuration")

    # Load TTS model (for voice)
    if ENABLE_TTS:
        try:
            tts = TextToSpeech(
                model_name=TTS_MODEL,
                device=DEVICE,
            )
            tts.load()
            logger.info("TTS model loaded successfully")
        except Exception as e:
            logger.error(f"Failed to load TTS model: {e}")
            tts = None
    else:
        logger.info("TTS disabled by configuration")

    yield

    # Cleanup
    logger.info("Shutting down Unified Inference Service")
    if translator:
        del translator
    if detector:
        del detector
    if stt:
        stt.unload()
    if tts:
        tts.unload()


app = FastAPI(
    title="LinguaBridge Unified Inference Service",
    description="Translation, language detection, and voice translation in one service",
    version="0.2.0",
    lifespan=lifespan
)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# ============================================================================
# Request/Response Models for REST API
# ============================================================================

class TranslateRequest(BaseModel):
    text: str = Field(..., description="Text to translate", max_length=2000)
    source_lang: str = Field(..., description="Source language code (ISO 639-1)")
    target_lang: str = Field(..., description="Target language code (ISO 639-1)")


class TranslateResponse(BaseModel):
    translated_text: str
    source_lang: str
    target_lang: str
    confidence: Optional[float] = None


class DetectRequest(BaseModel):
    text: str = Field(..., description="Text to detect language for", max_length=2000)


class DetectResponse(BaseModel):
    language: str
    confidence: float


class HealthResponse(BaseModel):
    status: str
    model: str
    model_loaded: bool
    detector_loaded: bool
    stt_loaded: bool
    tts_loaded: bool
    stt_model: str
    tts_model: str


class LanguageInfo(BaseModel):
    code: str
    name: str


# ============================================================================
# REST Endpoints (from main.py)
# ============================================================================

@app.get("/health", response_model=HealthResponse)
async def health_check():
    """Check service health and model status."""
    all_critical_loaded = translator is not None and detector is not None
    return HealthResponse(
        status="ok" if all_critical_loaded else "degraded",
        model=TRANSLATE_MODEL,
        model_loaded=translator is not None,
        detector_loaded=detector is not None,
        stt_loaded=stt is not None,
        tts_loaded=tts is not None,
        stt_model=STT_MODEL,
        tts_model=TTS_MODEL,
    )


@app.post("/translate", response_model=TranslateResponse)
async def translate(request: TranslateRequest):
    """Translate text from source language to target language."""
    if translator is None:
        raise HTTPException(
            status_code=503,
            detail="Translation model not loaded. Please try again later."
        )

    # Skip if source and target are the same
    if request.source_lang == request.target_lang:
        return TranslateResponse(
            translated_text=request.text,
            source_lang=request.source_lang,
            target_lang=request.target_lang
        )

    try:
        result = translator.translate(
            text=request.text,
            source_lang=request.source_lang,
            target_lang=request.target_lang
        )
        return TranslateResponse(
            translated_text=result,
            source_lang=request.source_lang,
            target_lang=request.target_lang
        )
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        logger.error(f"Translation error: {e}")
        raise HTTPException(status_code=500, detail="Translation failed")


@app.post("/detect", response_model=DetectResponse)
async def detect_language(request: DetectRequest):
    """Detect the language of input text."""
    if detector is None:
        raise HTTPException(
            status_code=503,
            detail="Language detector not loaded. Please try again later."
        )

    try:
        lang, confidence = detector.detect(request.text)
        return DetectResponse(
            language=lang,
            confidence=confidence
        )
    except Exception as e:
        logger.error(f"Language detection error: {e}")
        raise HTTPException(status_code=500, detail="Language detection failed")


@app.get("/languages", response_model=list[LanguageInfo])
async def list_languages():
    """List all supported languages."""
    # TranslateGemma supports 55 languages
    languages = [
        ("ar", "Arabic"), ("bn", "Bengali"), ("bg", "Bulgarian"),
        ("ca", "Catalan"), ("zh", "Chinese"), ("hr", "Croatian"),
        ("cs", "Czech"), ("da", "Danish"), ("nl", "Dutch"),
        ("en", "English"), ("et", "Estonian"), ("fi", "Finnish"),
        ("fr", "French"), ("de", "German"), ("el", "Greek"),
        ("gu", "Gujarati"), ("he", "Hebrew"), ("hi", "Hindi"),
        ("hu", "Hungarian"), ("id", "Indonesian"), ("it", "Italian"),
        ("ja", "Japanese"), ("kn", "Kannada"), ("ko", "Korean"),
        ("lv", "Latvian"), ("lt", "Lithuanian"), ("mk", "Macedonian"),
        ("ms", "Malay"), ("ml", "Malayalam"), ("mr", "Marathi"),
        ("no", "Norwegian"), ("fa", "Persian"), ("pl", "Polish"),
        ("pt", "Portuguese"), ("pa", "Punjabi"), ("ro", "Romanian"),
        ("ru", "Russian"), ("sr", "Serbian"), ("sk", "Slovak"),
        ("sl", "Slovenian"), ("es", "Spanish"), ("sv", "Swedish"),
        ("ta", "Tamil"), ("te", "Telugu"), ("th", "Thai"),
        ("tr", "Turkish"), ("uk", "Ukrainian"), ("ur", "Urdu"),
        ("vi", "Vietnamese"),
    ]
    return [LanguageInfo(code=code, name=name) for code, name in languages]


# ============================================================================
# WebSocket Endpoint (from voice_service.py)
# ============================================================================

@app.websocket("/voice")
async def voice_websocket(websocket: WebSocket):
    """
    WebSocket endpoint for voice translation.

    Supports both binary and text frames:

    Binary frame format (recommended - 33% bandwidth savings):
        [4 bytes: header_length as u32 LE]
        [header_length bytes: JSON header]
        [remaining bytes: raw PCM i16 LE samples]

    Text frame format (JSON, backward compatible):
        {
            "type": "Audio",
            "guild_id": "123",
            "channel_id": "456",
            "user_id": "789",
            "username": "User",
            "audio_base64": "<base64 PCM i16 48kHz mono>",
            "sample_rate": 48000,
            "target_language": "en",
            "generate_tts": false,
            "audio_hash": 12345678901234567890
        }

    Response format (JSON text frame):
        {
            "type": "Result",
            "guild_id": "123",
            "channel_id": "456",
            "user_id": "789",
            "username": "User",
            "original_text": "Hola mundo",
            "translated_text": "Hello world",
            "source_language": "es",
            "target_language": "en",
            "tts_audio": "<base64 WAV>",
            "latency_ms": 450,
            "audio_hash": 12345678901234567890
        }
    """
    await websocket.accept()
    logger.info("Voice WebSocket connection established")

    # Send ready message
    await websocket.send_json({
        "type": "Ready",
        "stt_models": [STT_MODEL] if stt else [],
        "tts_models": [TTS_MODEL] if tts else [],
    })

    try:
        while True:
            # Receive message (can be text or binary)
            raw_message = await websocket.receive()

            # Parse based on message type
            try:
                if "bytes" in raw_message:
                    # Binary frame
                    binary_data = raw_message["bytes"]
                    logger.debug(f"Received binary frame: {len(binary_data)} bytes")

                    header, samples = parse_binary_frame(binary_data)
                    msg_type = header.get("type")

                    if msg_type == "Audio":
                        # Binary audio frame
                        start_time = time.time()

                        try:
                            result = await process_audio_binary(header, samples)
                            result["latency_ms"] = int((time.time() - start_time) * 1000)
                            await websocket.send_text(result)
                        except Exception as e:
                            logger.error(f"Audio processing error: {e}", exc_info=True)
                            error_response = create_error_response(str(e), "PROCESSING_ERROR")
                            await websocket.send_text(error_response)
                    else:
                        logger.warning(f"Unknown binary message type: {msg_type}")

                elif "text" in raw_message:
                    # Text frame (JSON)
                    text_data = raw_message["text"]
                    logger.debug(f"Received text frame: {len(text_data)} bytes")

                    message = parse_text_frame(text_data)
                    msg_type = message.get("type")

                    if msg_type == "Ping":
                        await websocket.send_text(create_pong_response())
                        continue

                    if msg_type == "Configure":
                        # Handle configuration updates
                        logger.info(f"Configuration update: {message}")
                        continue

                    if msg_type == "Audio":
                        # Text audio frame (legacy base64 format)
                        start_time = time.time()

                        try:
                            result = await process_audio_text(message)
                            result["latency_ms"] = int((time.time() - start_time) * 1000)
                            await websocket.send_json(result)
                        except Exception as e:
                            logger.error(f"Audio processing error: {e}", exc_info=True)
                            await websocket.send_json({
                                "type": "Error",
                                "message": str(e),
                                "code": "PROCESSING_ERROR",
                            })
                else:
                    logger.warning(f"Unknown WebSocket message format: {raw_message.keys()}")

            except VoiceProtocolError as e:
                logger.error(f"Protocol error: {e}")
                error_response = create_error_response(str(e), "PROTOCOL_ERROR")
                await websocket.send_text(error_response)

    except WebSocketDisconnect:
        logger.info("Voice WebSocket connection closed")
    except Exception as e:
        logger.error(f"Voice WebSocket error: {e}", exc_info=True)


async def process_audio_binary(header: dict, samples: np.ndarray) -> str:
    """
    Process incoming binary audio frame and return translation result.

    Args:
        header: Parsed JSON header with metadata
        samples: Raw PCM samples (i16)

    Returns:
        JSON string response (use send_text, not send_json)
    """
    guild_id = header["guild_id"]
    channel_id = header["channel_id"]
    user_id = header["user_id"]
    username = header["username"]
    sample_rate = header.get("sample_rate", DISCORD_SAMPLE_RATE)
    target_language = header.get("target_language", "en")
    generate_tts = header.get("generate_tts", False)
    audio_hash = header.get("audio_hash", 0)  # CRITICAL: Must echo back

    # Convert i16 samples to float32 for processing
    audio_float = samples.astype(np.float32) / 32768.0

    logger.info(
        f"Processing binary audio: {len(samples)} samples, "
        f"{len(samples) / sample_rate:.2f}s from {username}, "
        f"hash={audio_hash}"
    )

    # Process audio (transcribe + translate + TTS)
    result = await _process_audio_internal(
        audio_float=audio_float,
        sample_rate=sample_rate,
        guild_id=guild_id,
        channel_id=channel_id,
        user_id=user_id,
        username=username,
        target_language=target_language,
        generate_tts=generate_tts,
    )

    # Create response JSON (echo back audio_hash for cache)
    return create_result_response(
        guild_id=result["guild_id"],
        channel_id=result["channel_id"],
        user_id=result["user_id"],
        username=result["username"],
        original_text=result["original_text"],
        translated_text=result["translated_text"],
        source_language=result["source_language"],
        target_language=result["target_language"],
        tts_audio=result["tts_audio"],
        latency_ms=result.get("latency_ms", 0),
        audio_hash=audio_hash,  # Echo back for cache correlation
    )


async def process_audio_text(message: dict) -> dict:
    """
    Process incoming text audio frame (legacy base64 format).

    Args:
        message: Parsed JSON message with base64 audio

    Returns:
        Dict response (use send_json)
    """
    guild_id = message["guild_id"]
    channel_id = message["channel_id"]
    user_id = message["user_id"]
    username = message["username"]
    audio_base64 = message["audio_base64"]
    sample_rate = message.get("sample_rate", DISCORD_SAMPLE_RATE)
    target_language = message.get("target_language", "en")
    generate_tts = message.get("generate_tts", False)
    audio_hash = message.get("audio_hash", 0)  # Optional for text frames

    # Decode audio
    audio_bytes = base64.b64decode(audio_base64)
    audio = np.frombuffer(audio_bytes, dtype=np.int16)
    audio_float = audio.astype(np.float32) / 32768.0

    logger.info(
        f"Processing text audio: {len(audio)} samples, "
        f"{len(audio) / sample_rate:.2f}s from {username}"
    )

    # Process audio (transcribe + translate + TTS)
    result = await _process_audio_internal(
        audio_float=audio_float,
        sample_rate=sample_rate,
        guild_id=guild_id,
        channel_id=channel_id,
        user_id=user_id,
        username=username,
        target_language=target_language,
        generate_tts=generate_tts,
    )

    # Add audio_hash if provided (for cache correlation)
    if audio_hash:
        result["audio_hash"] = audio_hash

    return result


async def _process_audio_internal(
    audio_float: np.ndarray,
    sample_rate: int,
    guild_id: str,
    channel_id: str,
    user_id: str,
    username: str,
    target_language: str,
    generate_tts: bool,
) -> dict:
    """
    Internal audio processing logic shared by binary and text handlers.

    Args:
        audio_float: Audio samples as float32 [-1.0, 1.0]
        sample_rate: Sample rate in Hz
        guild_id: Discord guild ID
        channel_id: Discord channel ID
        user_id: Discord user ID
        username: Discord username
        target_language: Target language code
        generate_tts: Whether to generate TTS audio

    Returns:
        Dict with transcription/translation results
    """
    # Step 1: Transcribe
    if stt is None:
        raise RuntimeError("STT model not loaded")

    transcription = stt.transcribe(audio_float, sample_rate=sample_rate)
    original_text = transcription.text
    source_language = transcription.language

    if not original_text.strip():
        logger.info("No speech detected in audio")
        return {
            "type": "Result",
            "guild_id": guild_id,
            "channel_id": channel_id,
            "user_id": user_id,
            "username": username,
            "original_text": "",
            "translated_text": "",
            "source_language": source_language,
            "target_language": target_language,
            "tts_audio": None,
        }

    logger.info(f"Transcribed ({source_language}): {original_text}")

    # Step 2: Translate (if needed)
    translated_text = original_text
    if source_language != target_language and translator is not None:
        try:
            translated_text = translator.translate(
                original_text,
                source_lang=source_language,
                target_lang=target_language,
            )
            logger.info(f"Translated ({target_language}): {translated_text}")
        except Exception as e:
            logger.error(f"Translation failed: {e}")
            # Fall back to original text
            translated_text = original_text

    # Step 3: Synthesize TTS (if requested)
    tts_audio = None
    if generate_tts and tts is not None and translated_text.strip():
        try:
            tts_result = await tts.synthesize(
                translated_text,
                language=target_language,
            )
            tts_audio = audio_to_base64(tts_result.audio, tts_result.sample_rate)
            logger.info(f"Generated TTS: {tts_result.duration:.2f}s")
        except Exception as e:
            logger.error(f"TTS failed: {e}")

    return {
        "type": "Result",
        "guild_id": guild_id,
        "channel_id": channel_id,
        "user_id": user_id,
        "username": username,
        "original_text": original_text,
        "translated_text": translated_text,
        "source_language": source_language,
        "target_language": target_language,
        "tts_audio": tts_audio,
    }


if __name__ == "__main__":
    import uvicorn

    host = os.getenv("HOST", "0.0.0.0")
    port = int(os.getenv("PORT", "8000"))

    uvicorn.run(
        "unified_inference:app",
        host=host,
        port=port,
        reload=os.getenv("DEBUG", "false").lower() == "true"
    )

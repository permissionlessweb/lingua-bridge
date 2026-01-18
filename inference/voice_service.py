"""
LinguaBridge Voice Inference Service

WebSocket server for real-time voice translation.
Receives audio from Discord bot, transcribes, translates, and synthesizes speech.
"""

import asyncio
import base64
import json
import logging
import os
import time
from contextlib import asynccontextmanager
from dataclasses import dataclass
from typing import Optional

import numpy as np
from dotenv import load_dotenv
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware

from voice.stt import SpeechToText
from voice.tts import TextToSpeech, audio_to_base64
from translator import TranslateGemmaTranslator

# Load environment variables
load_dotenv()

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Configuration
STT_MODEL = os.getenv("STT_MODEL", "distil-large-v3")
TTS_MODEL = os.getenv("TTS_MODEL", "CosyVoice2-0.5B")
TRANSLATE_MODEL = os.getenv("TRANSLATEGEMMA_MODEL", "google/translategemma-4b-it")
DEVICE = os.getenv("DEVICE", "cuda")
ENABLE_DIARIZATION = os.getenv("ENABLE_DIARIZATION", "false").lower() == "true"
ENABLE_TTS = os.getenv("ENABLE_TTS", "true").lower() == "true"
HF_TOKEN = os.getenv("HF_TOKEN", "")

# Discord audio format
DISCORD_SAMPLE_RATE = 48000

# Global model instances
stt: Optional[SpeechToText] = None
tts: Optional[TextToSpeech] = None
translator: Optional[TranslateGemmaTranslator] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Initialize models on startup."""
    global stt, tts, translator

    logger.info("Starting Voice Inference Service")
    logger.info(f"STT Model: {STT_MODEL}")
    logger.info(f"TTS Model: {TTS_MODEL}")
    logger.info(f"Translate Model: {TRANSLATE_MODEL}")
    logger.info(f"Device: {DEVICE}")

    # Load STT model
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

    # Load TTS model
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

    # Load translation model
    try:
        translator = TranslateGemmaTranslator(
            model_id=TRANSLATE_MODEL,
            device=DEVICE,
            torch_dtype="bfloat16" if DEVICE == "cuda" else "float32",
        )
        logger.info("Translation model loaded successfully")
    except Exception as e:
        logger.error(f"Failed to load translation model: {e}")
        translator = None

    yield

    # Cleanup
    logger.info("Shutting down Voice Inference Service")
    if stt:
        stt.unload()
    if tts:
        tts.unload()


app = FastAPI(
    title="LinguaBridge Voice Inference Service",
    description="Real-time voice translation via WebSocket",
    version="0.1.0",
    lifespan=lifespan,
)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
async def health():
    """Health check endpoint."""
    return {
        "status": "ok" if stt else "degraded",
        "stt_loaded": stt is not None,
        "tts_loaded": tts is not None,
        "translator_loaded": translator is not None,
        "stt_model": STT_MODEL,
        "tts_model": TTS_MODEL,
    }


@app.websocket("/voice")
async def voice_websocket(websocket: WebSocket):
    """
    WebSocket endpoint for voice translation.

    Message format (JSON):
    {
        "type": "Audio",
        "guild_id": "123",
        "channel_id": "456",
        "user_id": "789",
        "username": "User",
        "audio_base64": "<base64 PCM i16 48kHz mono>",
        "sample_rate": 48000,
        "target_language": "en",
        "generate_tts": false
    }

    Response format:
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
        "latency_ms": 450
    }
    """
    await websocket.accept()
    logger.info("Voice WebSocket connection established")

    # Send ready message
    await websocket.send_json({
        "type": "Ready",
        "stt_models": [STT_MODEL],
        "tts_models": [TTS_MODEL] if tts else [],
    })

    try:
        while True:
            # Receive message
            data = await websocket.receive_text()
            message = json.loads(data)

            msg_type = message.get("type")

            if msg_type == "Ping":
                await websocket.send_json({"type": "Pong"})
                continue

            if msg_type == "Configure":
                # Handle configuration updates
                logger.info(f"Configuration update: {message}")
                continue

            if msg_type == "Audio":
                # Process audio
                start_time = time.time()

                try:
                    result = await process_audio(message)
                    result["latency_ms"] = int((time.time() - start_time) * 1000)
                    await websocket.send_json(result)
                except Exception as e:
                    logger.error(f"Audio processing error: {e}")
                    await websocket.send_json({
                        "type": "Error",
                        "message": str(e),
                        "code": "PROCESSING_ERROR",
                    })

    except WebSocketDisconnect:
        logger.info("Voice WebSocket connection closed")
    except Exception as e:
        logger.error(f"Voice WebSocket error: {e}")


async def process_audio(message: dict) -> dict:
    """Process incoming audio and return translation result."""
    guild_id = message["guild_id"]
    channel_id = message["channel_id"]
    user_id = message["user_id"]
    username = message["username"]
    audio_base64 = message["audio_base64"]
    sample_rate = message.get("sample_rate", DISCORD_SAMPLE_RATE)
    target_language = message.get("target_language", "en")
    generate_tts = message.get("generate_tts", False)

    # Decode audio
    audio_bytes = base64.b64decode(audio_base64)
    audio = np.frombuffer(audio_bytes, dtype=np.int16)
    audio_float = audio.astype(np.float32) / 32768.0

    logger.info(
        f"Processing audio: {len(audio)} samples, "
        f"{len(audio) / sample_rate:.2f}s from {username}"
    )

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
    port = int(os.getenv("PORT", "8001"))

    uvicorn.run(
        "voice_service:app",
        host=host,
        port=port,
        reload=os.getenv("DEBUG", "false").lower() == "true",
    )

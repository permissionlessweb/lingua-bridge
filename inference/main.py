"""
LinguaBridge Inference Service

FastAPI server providing translation and language detection endpoints
using TranslateGemma models from Google.
"""

import os
import logging
from contextlib import asynccontextmanager
from typing import Optional

from dotenv import load_dotenv
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field

from translator import TranslateGemmaTranslator
from detector import LanguageDetector

# Load environment variables
load_dotenv()

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Configuration
MODEL_ID = os.getenv("TRANSLATEGEMMA_MODEL", "google/translategemma-4b-it")
DEVICE = os.getenv("DEVICE", "cuda")  # cuda, cpu, or auto
TORCH_DTYPE = os.getenv("TORCH_DTYPE", "bfloat16")  # bfloat16, float16, float32

# Global instances
translator: Optional[TranslateGemmaTranslator] = None
detector: Optional[LanguageDetector] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Initialize models on startup."""
    global translator, detector

    logger.info(f"Loading TranslateGemma model: {MODEL_ID}")
    logger.info(f"Device: {DEVICE}, Dtype: {TORCH_DTYPE}")

    try:
        translator = TranslateGemmaTranslator(
            model_id=MODEL_ID,
            device=DEVICE,
            torch_dtype=TORCH_DTYPE
        )
        logger.info("TranslateGemma model loaded successfully")
    except Exception as e:
        logger.error(f"Failed to load TranslateGemma model: {e}")
        translator = None

    try:
        detector = LanguageDetector()
        logger.info("Language detector loaded successfully")
    except Exception as e:
        logger.error(f"Failed to load language detector: {e}")
        detector = None

    yield

    # Cleanup
    logger.info("Shutting down inference service")
    if translator:
        del translator
    if detector:
        del detector


app = FastAPI(
    title="LinguaBridge Inference Service",
    description="Translation and language detection powered by TranslateGemma",
    version="0.1.0",
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


# Request/Response models
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


class LanguageInfo(BaseModel):
    code: str
    name: str


# Endpoints
@app.get("/health", response_model=HealthResponse)
async def health_check():
    """Check service health and model status."""
    return HealthResponse(
        status="ok" if translator and detector else "degraded",
        model=MODEL_ID,
        model_loaded=translator is not None,
        detector_loaded=detector is not None
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


if __name__ == "__main__":
    import uvicorn

    host = os.getenv("HOST", "0.0.0.0")
    port = int(os.getenv("PORT", "8000"))

    uvicorn.run(
        "main:app",
        host=host,
        port=port,
        reload=os.getenv("DEBUG", "false").lower() == "true"
    )

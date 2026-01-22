"""
Mock Text Inference Service

This module provides a mock for the text inference service,
replacing the FastAPI server during e2e testing.
"""

import json
from pathlib import Path
from typing import Dict, Any

from fastapi import FastAPI
import uvicorn


class MockTextInference:
    """Mock text inference service with exact matching."""

    def __init__(self):
        """Initialize with test scenarios from centralized test data."""
        self.scenarios = self._load_scenarios()
        self.health_status = {"status": "ok", "model_loaded": True, "detector_loaded": True}

    def _load_scenarios(self) -> Dict[str, Any]:
        """Load translation scenarios from centralized test data."""
        try:
            testdata_file = Path(__file__).parent.parent / "fixtures" / "testdata.json"
            with open(testdata_file) as f:
                data = json.load(f)
                return data.get("translations", {}).get("text", {})
        except FileNotFoundError:
            return {}

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        """Mock translation with exact matching."""
        # Find matching scenario
        for scenario in self.scenarios.values():
            if (scenario["input"] == text and
                scenario["source_lang"] == source_lang and
                scenario["target_lang"] == target_lang):
                return scenario["expected"]

        # Fallback mock response
        return f"[MOCK TRANSLATION] {text} ({source_lang} -> {target_lang})"

    def detect_language(self, text: str) -> tuple[str, float]:
        """Mock language detection."""
        # Simple mock - detect based on common words
        if any(word in text.lower() for word in ["hello", "world", "good"]):
            return ("en", 0.95)
        elif any(word in text.lower() for word in ["hola", "¿", "biblioteca"]):
            return ("es", 0.92)
        elif any(word in text for word in ["こんにちは", "世界"]):
            return ("ja", 0.98)
        else:
            return ("en", 0.80)  # Default fallback

    def get_health(self) -> Dict[str, Any]:
        """Get health status."""
        return self.health_status

    def set_health_status(self, status: str, model_loaded: bool = True, detector_loaded: bool = True):
        """Set health status for testing failure scenarios."""
        self.health_status = {
            "status": status,
            "model_loaded": model_loaded,
            "detector_loaded": detector_loaded
        }


# FastAPI mock app for integration testing
mock_app = FastAPI(title="Mock Linguabridge Text Inference")

mock_inference = MockTextInference()


@mock_app.get("/health")
async def health():
    """Mock health endpoint."""
    return mock_inference.get_health()


@mock_app.post("/translate")
async def translate(request: Dict[str, Any]):
    """Mock translate endpoint."""
    result = mock_inference.translate(
        text=request["text"],
        source_lang=request["source_lang"],
        target_lang=request["target_lang"]
    )
    return {
        "translated_text": result,
        "source_lang": request["source_lang"],
        "target_lang": request["target_lang"]
    }


@mock_app.post("/detect")
async def detect(request: Dict[str, Any]):
    """Mock detect endpoint."""
    lang, confidence = mock_inference.detect_language(request["text"])
    return {
        "language": lang,
        "confidence": confidence
    }


@mock_app.get("/languages")
async def languages():
    """Mock languages endpoint."""
    return [
        {"code": "en", "name": "English"},
        {"code": "es", "name": "Spanish"},
        {"code": "fr", "name": "French"},
        {"code": "de", "name": "German"},
        {"code": "ja", "name": "Japanese"},
        {"code": "zh", "name": "Chinese"}
    ]


async def run_mock_text_server(host: str = "localhost", port: int = 8000):
    """Run the mock text inference server asynchronously."""
    config = uvicorn.Config(mock_app, host=host, port=port, log_level="warning")
    server = uvicorn.Server(config)
    await server.serve()


def run_mock_text_server_sync(host: str = "localhost", port: int = 8000):
    """Run the mock text inference server (blocking)."""
    uvicorn.run(mock_app, host=host, port=port, log_level="warning")

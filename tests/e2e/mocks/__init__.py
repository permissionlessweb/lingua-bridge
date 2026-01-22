"""Mock services for E2E testing."""

from .text_inference_mock import (
    MockTextInference,
    mock_app,
    mock_inference,
    run_mock_text_server,
    run_mock_text_server_sync,
)
from .voice_inference_mock import (
    MockVoiceInference,
    mock_voice_inference,
    mock_voice_server,
    create_mock_audio_data,
)
from .web_server_mock import (
    mock_web_app,
    run_mock_web_server,
    run_mock_web_server_sync,
)

__all__ = [
    # Text inference
    "MockTextInference",
    "mock_app",
    "mock_inference",
    "run_mock_text_server",
    "run_mock_text_server_sync",
    # Voice inference
    "MockVoiceInference",
    "mock_voice_inference",
    "mock_voice_server",
    "create_mock_audio_data",
    # Web server
    "mock_web_app",
    "run_mock_web_server",
    "run_mock_web_server_sync",
]

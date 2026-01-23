"""
Mock Voice Inference Service

This module provides a mock for the voice inference service,
replacing the WebSocket server during e2e testing.
"""

import json
import asyncio
import base64
from pathlib import Path
from typing import Dict, Any, List

import websockets
from websockets.exceptions import ConnectionClosed


class MockVoiceInference:
    """Mock voice inference service with WebSocket support."""

    def __init__(self):
        """Initialize with test scenarios from centralized test data."""
        self.scenarios = self._load_scenarios()
        self.connected_clients: List[websockets.WebSocketServerProtocol] = []
        self.audio_samples: Dict[str, bytes] = {}

    def _load_scenarios(self) -> Dict[str, Any]:
        """Load translation scenarios from centralized test data."""
        try:
            testdata_file = Path(__file__).parent.parent / "fixtures" / "testdata.json"
            with open(testdata_file) as f:
                data = json.load(f)
                # Merge text and voice translations for voice mock
                scenarios = {}
                scenarios.update(data.get("translations", {}).get("text", {}))
                scenarios.update(data.get("translations", {}).get("voice", {}))
                return scenarios
        except FileNotFoundError:
            return {}

    def transcribe_audio(self, audio_data: bytes) -> str:
        """Mock STT transcription."""
        # Simple mock based on audio data length (in real scenarios, this would analyze audio)
        if len(audio_data) > 1000:  # "Long" audio
            return "Hello, this is a test message for voice translation."
        else:  # "Short" audio
            return "Hi there!"

    def translate_text(self, text: str, source_lang: str, target_lang: str) -> str:
        """Mock text translation."""
        # Use same logic as text inference mock
        for scenario in self.scenarios.values():
            # Handle both text scenarios (input) and voice scenarios (transcription)
            input_text = scenario.get("input") or scenario.get("transcription", "")
            if (input_text.lower() in text.lower() and
                scenario.get("source_lang") == source_lang and
                scenario.get("target_lang") == target_lang):
                return scenario.get("expected", "")

        return f"[MOCK VOICE TRANSLATION] {text} ({source_lang} -> {target_lang})"

    def synthesize_speech(self, text: str) -> bytes:
        """Mock TTS synthesis."""
        # Return mock audio data
        mock_audio = b"mock_tts_audio_" + text.encode()[:50]
        return mock_audio

    async def handle_connection(self, websocket: websockets.WebSocketServerProtocol):
        """Handle WebSocket connection."""
        self.connected_clients.append(websocket)
        try:
            async for message in websocket:
                data = json.loads(message)

                if data.get("type") == "audio":
                    # Process audio data
                    audio_b64 = data.get("audio", "")
                    audio_data = base64.b64decode(audio_b64)

                    # Mock STT
                    transcription = self.transcribe_audio(audio_data)

                    # Mock translation (assume English to Spanish for voice tests)
                    translation = self.translate_text(transcription, "en", "es")

                    # Mock TTS
                    tts_audio = self.synthesize_speech(translation)
                    tts_b64 = base64.b64encode(tts_audio).decode()

                    # Send response
                    response = {
                        "type": "translation",
                        "transcription": transcription,
                        "translation": translation,
                        "tts_audio": tts_b64,
                        "source_lang": "en",
                        "target_lang": "es"
                    }

                    await websocket.send(json.dumps(response))

                elif data.get("type") == "ping":
                    await websocket.send(json.dumps({"type": "pong"}))

        except ConnectionClosed:
            pass
        finally:
            if websocket in self.connected_clients:
                self.connected_clients.remove(websocket)

    async def broadcast_to_clients(self, message: Dict[str, Any]):
        """Broadcast message to all connected clients."""
        for client in self.connected_clients:
            try:
                await client.send(json.dumps(message))
            except:
                pass

    def get_connected_count(self) -> int:
        """Get number of connected clients."""
        return len(self.connected_clients)


# Global mock instance
mock_voice_inference = MockVoiceInference()


async def mock_voice_server(host: str = "localhost", port: int = 8001):
    """Run the mock voice inference WebSocket server."""
    server = await websockets.serve(
        mock_voice_inference.handle_connection,
        host,
        port
    )
    await server.wait_closed()


def create_mock_audio_data(text: str, duration_ms: int = 1000) -> bytes:
    """Create mock audio data for testing."""
    # In real scenarios, this would be actual audio encoding
    return f"mock_audio_{text}_{duration_ms}ms".encode()

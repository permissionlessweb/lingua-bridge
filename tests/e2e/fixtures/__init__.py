"""
Test fixtures and data.

Central location for all e2e test data. Import from here:

    from fixtures import TestData, TRANSLATIONS, VOICE_SAMPLES
"""

import json
from pathlib import Path
from typing import Dict, Any

FIXTURES_DIR = Path(__file__).parent
TESTDATA_FILE = FIXTURES_DIR / "testdata.json"

# Legacy file (kept for backwards compatibility)
TRANSLATION_SCENARIOS_FILE = FIXTURES_DIR / "translation_scenarios.json"
VOICE_SAMPLES_DIR = FIXTURES_DIR / "voice_samples"


def load_testdata() -> Dict[str, Any]:
    """Load the main test data file."""
    with open(TESTDATA_FILE) as f:
        return json.load(f)


def load_translation_scenarios() -> Dict[str, Any]:
    """Load translation scenarios (legacy format)."""
    with open(TRANSLATION_SCENARIOS_FILE) as f:
        return json.load(f)


class TestData:
    """
    Centralized access to all test data.

    Usage:
        data = TestData()
        scenario = data.translations["text"]["english_to_spanish"]
        langs = data.supported_languages
    """

    _instance = None
    _data = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._data = load_testdata()
        return cls._instance

    @property
    def translations(self) -> Dict[str, Any]:
        """All translation test scenarios (text and voice)."""
        return self._data["translations"]

    @property
    def text_translations(self) -> Dict[str, Any]:
        """Text translation scenarios only."""
        return self._data["translations"]["text"]

    @property
    def voice_translations(self) -> Dict[str, Any]:
        """Voice translation scenarios only."""
        return self._data["translations"]["voice"]

    @property
    def language_detection(self) -> Dict[str, Any]:
        """Language detection test cases."""
        return self._data["language_detection"]

    @property
    def voice(self) -> Dict[str, Any]:
        """Voice-related test data (audio samples, TTS)."""
        return self._data["voice"]

    @property
    def discord(self) -> Dict[str, Any]:
        """Discord-specific test data."""
        return self._data["discord"]

    @property
    def web(self) -> Dict[str, Any]:
        """Web interface test data."""
        return self._data["web"]

    @property
    def mock_responses(self) -> Dict[str, Any]:
        """Expected mock service responses."""
        return self._data["mock_responses"]

    @property
    def supported_languages(self) -> list:
        """List of supported languages."""
        return self._data["mock_responses"]["supported_languages"]

    def get_translation(self, scenario_name: str, category: str = "text") -> Dict[str, Any]:
        """Get a specific translation scenario by name."""
        return self._data["translations"][category].get(scenario_name, {})

    def get_webview_message(self, message_type: str) -> Dict[str, Any]:
        """Get a webview message template."""
        return self._data["web"]["webview_messages"].get(message_type, {})


# Convenience shortcuts
TRANSLATIONS = TestData().text_translations
VOICE_SAMPLES = TestData().voice
DISCORD_DATA = TestData().discord
WEB_DATA = TestData().web

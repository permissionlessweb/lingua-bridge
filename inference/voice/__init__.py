"""
Voice translation module for LinguaBridge.

Provides speech-to-text, translation, and text-to-speech capabilities
for real-time voice channel translation.
"""

from .stt import SpeechToText
from .tts import TextToSpeech

__all__ = ["SpeechToText", "TextToSpeech"]

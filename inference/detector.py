"""
Language Detection Module

Uses FastText's language identification model for detecting
the language of input text.
"""

import logging
import os
import urllib.request
from pathlib import Path
from typing import Tuple

logger = logging.getLogger(__name__)

# FastText lid model URL
FASTTEXT_MODEL_URL = "https://dl.fbaipublicfiles.com/fasttext/supervised-models/lid.176.bin"
MODEL_CACHE_DIR = Path.home() / ".cache" / "linguabridge"
MODEL_PATH = MODEL_CACHE_DIR / "lid.176.bin"


class LanguageDetector:
    """
    Language detection using FastText's language identification model.

    The model supports 176 languages and provides confidence scores.
    """

    def __init__(self, model_path: str = None):
        """
        Initialize the language detector.

        Args:
            model_path: Path to the FastText model file.
                       If None, downloads the default model.
        """
        try:
            import fasttext
        except ImportError:
            raise ImportError(
                "fasttext is required for language detection. "
                "Install with: pip install fasttext-wheel"
            )

        self.fasttext = fasttext

        # Use provided path or download default model
        if model_path:
            self.model_path = Path(model_path)
        else:
            self.model_path = MODEL_PATH
            self._ensure_model_downloaded()

        logger.info(f"Loading FastText model from {self.model_path}")

        # Suppress FastText warnings about loading
        self.model = fasttext.load_model(str(self.model_path))

        logger.info("Language detector initialized successfully")

    def _ensure_model_downloaded(self):
        """Download the FastText model if not present."""
        if self.model_path.exists():
            logger.debug(f"Model already exists at {self.model_path}")
            return

        logger.info(f"Downloading FastText language model to {self.model_path}")
        self.model_path.parent.mkdir(parents=True, exist_ok=True)

        try:
            urllib.request.urlretrieve(FASTTEXT_MODEL_URL, self.model_path)
            logger.info("Model downloaded successfully")
        except Exception as e:
            logger.error(f"Failed to download model: {e}")
            raise RuntimeError(
                f"Could not download FastText model. "
                f"Please download manually from {FASTTEXT_MODEL_URL} "
                f"and place at {self.model_path}"
            )

    def detect(self, text: str) -> Tuple[str, float]:
        """
        Detect the language of the input text.

        Args:
            text: Text to detect language for

        Returns:
            Tuple of (language_code, confidence)
            - language_code: ISO 639-1 two-letter code
            - confidence: Confidence score between 0 and 1
        """
        if not text.strip():
            return ("en", 0.0)  # Default to English for empty text

        # Clean text (FastText doesn't like newlines)
        clean_text = text.replace("\n", " ").strip()

        # Get prediction
        labels, scores = self.model.predict(clean_text, k=1)

        # Extract language code (format is __label__xx)
        label = labels[0]
        lang_code = label.replace("__label__", "")
        confidence = float(scores[0])

        return (lang_code, confidence)

    def detect_multiple(self, text: str, k: int = 3) -> list[Tuple[str, float]]:
        """
        Get top-k language predictions.

        Args:
            text: Text to detect language for
            k: Number of predictions to return

        Returns:
            List of (language_code, confidence) tuples
        """
        if not text.strip():
            return [("en", 0.0)]

        clean_text = text.replace("\n", " ").strip()
        labels, scores = self.model.predict(clean_text, k=k)

        results = []
        for label, score in zip(labels, scores):
            lang_code = label.replace("__label__", "")
            results.append((lang_code, float(score)))

        return results

    @property
    def supported_languages(self) -> int:
        """Return the number of supported languages."""
        return 176  # FastText lid.176 model


class FallbackDetector:
    """
    Fallback language detector using simple heuristics.
    Used when FastText is not available.
    """

    # Character ranges for common language scripts
    SCRIPT_RANGES = {
        "ja": [
            (0x3040, 0x309F),  # Hiragana
            (0x30A0, 0x30FF),  # Katakana
        ],
        "zh": [
            (0x4E00, 0x9FFF),  # CJK Unified Ideographs
        ],
        "ko": [
            (0xAC00, 0xD7AF),  # Hangul Syllables
        ],
        "ar": [
            (0x0600, 0x06FF),  # Arabic
        ],
        "he": [
            (0x0590, 0x05FF),  # Hebrew
        ],
        "th": [
            (0x0E00, 0x0E7F),  # Thai
        ],
        "ru": [
            (0x0400, 0x04FF),  # Cyrillic
        ],
        "el": [
            (0x0370, 0x03FF),  # Greek
        ],
        "hi": [
            (0x0900, 0x097F),  # Devanagari
        ],
    }

    def detect(self, text: str) -> Tuple[str, float]:
        """Simple script-based detection."""
        if not text.strip():
            return ("en", 0.0)

        char_counts = {}
        total = 0

        for char in text:
            code = ord(char)
            for lang, ranges in self.SCRIPT_RANGES.items():
                for start, end in ranges:
                    if start <= code <= end:
                        char_counts[lang] = char_counts.get(lang, 0) + 1
                        total += 1
                        break

        if not char_counts:
            # Default to English for Latin script
            return ("en", 0.5)

        # Return language with most characters
        detected = max(char_counts, key=char_counts.get)
        confidence = char_counts[detected] / max(total, 1)

        return (detected, confidence)


def create_detector() -> LanguageDetector:
    """
    Create the best available language detector.
    Falls back to heuristic detection if FastText unavailable.
    """
    try:
        return LanguageDetector()
    except ImportError:
        logger.warning(
            "FastText not available, using fallback detector. "
            "Install fasttext-wheel for better accuracy."
        )
        return FallbackDetector()


if __name__ == "__main__":
    # Test the detector
    logging.basicConfig(level=logging.INFO)

    detector = create_detector()

    test_texts = [
        "Hello, how are you today?",
        "Bonjour, comment allez-vous?",
        "Hola, ¿cómo estás?",
        "こんにちは、元気ですか?",
        "你好，你好吗？",
        "안녕하세요, 잘 지내세요?",
        "Привет, как дела?",
        "مرحبا، كيف حالك؟",
    ]

    print("Language Detection Tests:")
    print("-" * 50)

    for text in test_texts:
        lang, conf = detector.detect(text)
        print(f"'{text[:30]}...'")
        print(f"  -> {lang} (confidence: {conf:.2%})")
        print()

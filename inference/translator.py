"""
TranslateGemma Translation Module

Wrapper for Google's TranslateGemma models for text translation.
Supports 4B, 12B, and 27B model variants.
"""

import logging
from typing import Optional

import torch
from transformers import AutoModelForImageTextToText, AutoProcessor

logger = logging.getLogger(__name__)

# Supported model variants
SUPPORTED_MODELS = {
    "google/translategemma-4b-it",
    "google/translategemma-12b-it",
    "google/translategemma-27b-it",
}

# Map common dtype strings to torch dtypes
DTYPE_MAP = {
    "bfloat16": torch.bfloat16,
    "float16": torch.float16,
    "float32": torch.float32,
    "auto": "auto",
}


class TranslateGemmaTranslator:
    """
    Translation engine using Google's TranslateGemma models.

    TranslateGemma uses a specific chat template format:
    {
        "role": "user",
        "content": [
            {
                "type": "text",
                "source_lang_code": "<ISO 639-1 code>",
                "target_lang_code": "<ISO 639-1 code>",
                "text": "<text to translate>"
            }
        ]
    }
    """

    def __init__(
        self,
        model_id: str = "google/translategemma-4b-it",
        device: str = "cuda",
        torch_dtype: str = "bfloat16",
        max_new_tokens: int = 512,
    ):
        """
        Initialize the TranslateGemma translator.

        Args:
            model_id: HuggingFace model ID (4b-it, 12b-it, or 27b-it)
            device: Device to run on ('cuda', 'cpu', or 'auto')
            torch_dtype: Data type ('bfloat16', 'float16', 'float32', 'auto')
            max_new_tokens: Maximum tokens to generate
        """
        if model_id not in SUPPORTED_MODELS:
            logger.warning(
                f"Model {model_id} not in known supported models. Proceeding anyway."
            )

        self.model_id = model_id
        self.max_new_tokens = max_new_tokens
        self.device = device

        # Parse dtype
        dtype = DTYPE_MAP.get(torch_dtype, torch.bfloat16)

        logger.info(f"Loading processor for {model_id}")
        self.processor = AutoProcessor.from_pretrained(model_id)

        logger.info(f"Loading model {model_id} on {device} with dtype {torch_dtype}")

        # Configure device mapping
        if device == "auto":
            device_map = "auto"
        elif device == "cuda" and torch.cuda.is_available():
            device_map = "auto"  # Let accelerate handle multi-GPU
        else:
            device_map = "cpu"

        self.model = AutoModelForImageTextToText.from_pretrained(
            model_id,
            device_map=device_map,
            torch_dtype=dtype if dtype != "auto" else None,
            trust_remote_code=True,
        )

        # Set model to eval mode
        self.model.eval()

        logger.info(f"Model loaded successfully. Device: {self.model.device}")

    def translate(
        self,
        text: str,
        source_lang: str,
        target_lang: str,
    ) -> str:
        """
        Translate text from source language to target language.

        Args:
            text: Text to translate
            source_lang: Source language code (ISO 639-1, e.g., 'en', 'es', 'zh')
            target_lang: Target language code (ISO 639-1)

        Returns:
            Translated text
        """
        if not text.strip():
            return ""

        # Skip if same language
        if source_lang == target_lang:
            return text

        # Build the chat message in TranslateGemma format
        messages = [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "source_lang_code": source_lang,
                        "target_lang_code": target_lang,
                        "text": text,
                    }
                ],
            }
        ]

        # Process input
        inputs = self.processor.apply_chat_template(
            messages,
            tokenize=True,
            add_generation_prompt=True,
            return_dict=True,
            return_tensors="pt",
        )

        # Move to model device
        inputs = {
            k: v.to(self.model.device, dtype=self.model.dtype)
            if hasattr(v, "to") else v
            for k, v in inputs.items()
        }

        input_len = inputs["input_ids"].shape[1]

        # Generate translation
        with torch.inference_mode():
            outputs = self.model.generate(
                **inputs,
                max_new_tokens=self.max_new_tokens,
                do_sample=False,
                pad_token_id=self.processor.tokenizer.pad_token_id,
                eos_token_id=self.processor.tokenizer.eos_token_id,
            )

        # Decode only the generated tokens (exclude input)
        generated = outputs[0][input_len:]
        translated = self.processor.decode(generated, skip_special_tokens=True)

        return translated.strip()

    def translate_batch(
        self,
        texts: list[str],
        source_lang: str,
        target_lang: str,
    ) -> list[str]:
        """
        Translate multiple texts (currently processes sequentially).

        Args:
            texts: List of texts to translate
            source_lang: Source language code
            target_lang: Target language code

        Returns:
            List of translated texts
        """
        # For now, process sequentially
        # Future optimization: batch processing
        return [
            self.translate(text, source_lang, target_lang)
            for text in texts
        ]

    @property
    def supported_languages(self) -> list[str]:
        """Return list of supported language codes."""
        return [
            "ar", "bn", "bg", "ca", "zh", "hr", "cs", "da", "nl", "en",
            "et", "fi", "fr", "de", "el", "gu", "he", "hi", "hu", "id",
            "it", "ja", "kn", "ko", "lv", "lt", "mk", "ms", "ml", "mr",
            "no", "fa", "pl", "pt", "pa", "ro", "ru", "sr", "sk", "sl",
            "es", "sv", "ta", "te", "th", "tr", "uk", "ur", "vi",
        ]

    def is_language_supported(self, lang_code: str) -> bool:
        """Check if a language is supported."""
        # Handle regional variants (en_US -> en)
        base_code = lang_code.split("_")[0].split("-")[0].lower()
        return base_code in self.supported_languages


if __name__ == "__main__":
    # Test the translator
    logging.basicConfig(level=logging.INFO)

    print("Loading TranslateGemma translator...")
    translator = TranslateGemmaTranslator(
        model_id="google/translategemma-4b-it",
        device="cuda" if torch.cuda.is_available() else "cpu",
    )

    # Test translation
    test_cases = [
        ("Hello, how are you?", "en", "es"),
        ("Bonjour, comment allez-vous?", "fr", "en"),
        ("こんにちは", "ja", "en"),
    ]

    for text, src, tgt in test_cases:
        result = translator.translate(text, src, tgt)
        print(f"\n{src} -> {tgt}")
        print(f"  Input:  {text}")
        print(f"  Output: {result}")

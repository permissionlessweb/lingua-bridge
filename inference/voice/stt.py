"""
Speech-to-Text module using Distil-Whisper.

Uses faster-whisper for efficient transcription with optional
speaker diarization via pyannote.audio.
"""

import logging
import time
from dataclasses import dataclass
from typing import List, Optional, Tuple

import numpy as np

logger = logging.getLogger(__name__)

# Constants
SAMPLE_RATE = 16000  # Whisper expects 16kHz audio


@dataclass
class TranscriptionSegment:
    """A segment of transcribed speech."""
    text: str
    start: float
    end: float
    speaker: Optional[str] = None
    confidence: Optional[float] = None


@dataclass
class TranscriptionResult:
    """Complete transcription result."""
    text: str
    segments: List[TranscriptionSegment]
    language: str
    language_probability: float
    duration: float
    processing_time_ms: int


class SpeechToText:
    """
    Speech-to-text engine using Distil-Whisper.

    Uses faster-whisper for efficient CTranslate2-based inference.
    Optionally integrates pyannote.audio for speaker diarization.
    """

    def __init__(
        self,
        model_size: str = "distil-large-v3",
        device: str = "cuda",
        compute_type: str = "float16",
        enable_diarization: bool = False,
        hf_token: Optional[str] = None,
    ):
        """
        Initialize the STT engine.

        Args:
            model_size: Whisper model size (distil-large-v3, large-v3, etc.)
            device: Device to run on (cuda, cpu)
            compute_type: Compute type (float16, int8, int8_float16)
            enable_diarization: Whether to enable speaker diarization
            hf_token: HuggingFace token for pyannote models
        """
        self.model_size = model_size
        self.device = device
        self.compute_type = compute_type
        self.enable_diarization = enable_diarization
        self.hf_token = hf_token

        self.model = None
        self.diarization_pipeline = None

    def load(self):
        """Load the models."""
        from faster_whisper import WhisperModel

        logger.info(f"Loading Whisper model: {self.model_size}")
        start = time.time()

        self.model = WhisperModel(
            self.model_size,
            device=self.device,
            compute_type=self.compute_type,
        )

        load_time = time.time() - start
        logger.info(f"Whisper model loaded in {load_time:.2f}s")

        if self.enable_diarization:
            self._load_diarization()

    def _load_diarization(self):
        """Load speaker diarization pipeline."""
        try:
            from pyannote.audio import Pipeline

            logger.info("Loading speaker diarization pipeline")

            if not self.hf_token:
                logger.warning(
                    "No HuggingFace token provided. "
                    "Diarization requires accepting pyannote license."
                )
                return

            self.diarization_pipeline = Pipeline.from_pretrained(
                "pyannote/speaker-diarization-3.1",
                use_auth_token=self.hf_token,
            )

            if self.device == "cuda":
                import torch
                self.diarization_pipeline.to(torch.device("cuda"))

            logger.info("Diarization pipeline loaded")

        except ImportError:
            logger.warning("pyannote.audio not installed, diarization disabled")
        except Exception as e:
            logger.error(f"Failed to load diarization: {e}")

    def transcribe(
        self,
        audio: np.ndarray,
        sample_rate: int = SAMPLE_RATE,
        language: Optional[str] = None,
    ) -> TranscriptionResult:
        """
        Transcribe audio to text.

        Args:
            audio: Audio samples as numpy array (float32, mono)
            sample_rate: Audio sample rate (will resample if not 16kHz)
            language: Optional language code to use (auto-detect if None)

        Returns:
            TranscriptionResult with text and segments
        """
        if self.model is None:
            raise RuntimeError("Model not loaded. Call load() first.")

        start_time = time.time()

        # Resample if needed
        if sample_rate != SAMPLE_RATE:
            audio = self._resample(audio, sample_rate, SAMPLE_RATE)

        # Ensure float32
        if audio.dtype != np.float32:
            audio = audio.astype(np.float32)

        # Normalize
        if audio.max() > 1.0:
            audio = audio / 32768.0

        # Transcribe
        segments, info = self.model.transcribe(
            audio,
            language=language,
            beam_size=5,
            best_of=5,
            vad_filter=True,
            vad_parameters=dict(
                min_silence_duration_ms=500,
                speech_pad_ms=200,
            ),
        )

        # Collect segments
        result_segments = []
        full_text_parts = []

        for segment in segments:
            result_segments.append(TranscriptionSegment(
                text=segment.text.strip(),
                start=segment.start,
                end=segment.end,
                confidence=segment.avg_logprob if hasattr(segment, 'avg_logprob') else None,
            ))
            full_text_parts.append(segment.text.strip())

        full_text = " ".join(full_text_parts)

        # Add speaker labels if diarization enabled
        if self.diarization_pipeline is not None and len(result_segments) > 0:
            result_segments = self._add_speaker_labels(audio, result_segments)

        processing_time = int((time.time() - start_time) * 1000)

        return TranscriptionResult(
            text=full_text,
            segments=result_segments,
            language=info.language,
            language_probability=info.language_probability,
            duration=info.duration,
            processing_time_ms=processing_time,
        )

    def _resample(
        self,
        audio: np.ndarray,
        orig_sr: int,
        target_sr: int
    ) -> np.ndarray:
        """Resample audio to target sample rate."""
        try:
            import librosa
            return librosa.resample(audio, orig_sr=orig_sr, target_sr=target_sr)
        except ImportError:
            # Fallback to scipy
            from scipy import signal
            duration = len(audio) / orig_sr
            target_length = int(duration * target_sr)
            return signal.resample(audio, target_length)

    def _add_speaker_labels(
        self,
        audio: np.ndarray,
        segments: List[TranscriptionSegment],
    ) -> List[TranscriptionSegment]:
        """Add speaker labels to segments using diarization."""
        try:
            import torch

            # Run diarization
            waveform = torch.from_numpy(audio).unsqueeze(0)
            diarization = self.diarization_pipeline({
                "waveform": waveform,
                "sample_rate": SAMPLE_RATE,
            })

            # Create speaker timeline
            speaker_timeline = []
            for turn, _, speaker in diarization.itertracks(yield_label=True):
                speaker_timeline.append((turn.start, turn.end, speaker))

            # Assign speakers to segments
            for segment in segments:
                segment_mid = (segment.start + segment.end) / 2
                for start, end, speaker in speaker_timeline:
                    if start <= segment_mid <= end:
                        segment.speaker = speaker
                        break

        except Exception as e:
            logger.error(f"Diarization failed: {e}")

        return segments

    def unload(self):
        """Unload models to free memory."""
        if self.model is not None:
            del self.model
            self.model = None
        if self.diarization_pipeline is not None:
            del self.diarization_pipeline
            self.diarization_pipeline = None

        # Force garbage collection
        import gc
        gc.collect()

        try:
            import torch
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except ImportError:
            pass

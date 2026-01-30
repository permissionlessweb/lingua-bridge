"""
Protocol Error Handling Tests

Tests that the Python inference service handles corrupted/invalid frames
gracefully without crashing. These scenarios WILL happen in production:
- Network corruption
- Malicious/malformed inputs
- Client bugs sending bad frames

If these tests don't pass, your service will crash on the first bad frame.
"""

import json
import struct
import sys
from pathlib import Path

import numpy as np
import pytest

# Add inference directory to path
inference_path = Path(__file__).parent.parent.parent / "inference"
sys.path.insert(0, str(inference_path))

from voice_protocol import (
    parse_binary_frame,
    parse_text_frame,
    VoiceProtocolError,
)


class TestCorruptedFrames:
    """Test handling of corrupted binary frames."""

    def test_frame_with_incorrect_header_length(self):
        """
        Test frame claiming 1000-byte header but only providing 100 bytes.

        This happens with network corruption or buggy clients.
        If not handled, causes index out of bounds crashes.
        """
        # Claim 1000-byte header but only send 100 bytes total
        fake_header_len = 1000
        actual_data = b'x' * 96  # 4 bytes for len + 96 = 100 total

        corrupted_frame = struct.pack('<I', fake_header_len) + actual_data

        with pytest.raises(VoiceProtocolError, match="Frame too short for header"):
            parse_binary_frame(corrupted_frame)

    def test_frame_with_negative_header_length(self):
        """
        Test frame with negative header length (via overflow).

        Malicious clients can try this. Must reject.
        """
        # Use a huge number that would overflow
        huge_len = 0xFFFFFFFF  # Max u32, would wrap negative if treated as i32

        corrupted_frame = struct.pack('<I', huge_len) + b'x' * 100

        with pytest.raises(VoiceProtocolError, match="Invalid header length"):
            parse_binary_frame(corrupted_frame)

    def test_frame_with_zero_length(self):
        """Test frame claiming 0-byte header (invalid)."""
        corrupted_frame = struct.pack('<I', 0) + b'some data'

        with pytest.raises(VoiceProtocolError):
            parse_binary_frame(corrupted_frame)

    def test_completely_empty_frame(self):
        """Test completely empty binary message."""
        with pytest.raises(VoiceProtocolError, match="Binary frame too short"):
            parse_binary_frame(b'')

    def test_single_byte_frame(self):
        """Test frame with only 1 byte (need at least 4 for length)."""
        with pytest.raises(VoiceProtocolError, match="Binary frame too short"):
            parse_binary_frame(b'\x01')

    def test_header_length_exactly_equals_frame_size(self):
        """
        Test edge case: header_len claims the entire frame is header.
        This means no PCM data, which should be valid (silence detection).
        """
        header = {'type': 'Audio', 'audio_hash': 999}
        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)

        # Frame is exactly [4-byte len][header] with no PCM
        frame = struct.pack('<I', header_len) + header_bytes

        # This should parse successfully with empty samples
        parsed_header, parsed_samples = parse_binary_frame(frame)
        assert parsed_header['audio_hash'] == 999
        assert len(parsed_samples) == 0


class TestInvalidUTF8:
    """Test handling of non-UTF8 data in JSON header."""

    def test_invalid_utf8_bytes_in_header(self):
        """
        Test header with invalid UTF-8 sequences.

        Network corruption or binary data interpreted as UTF-8.
        """
        header_len = 10
        # Invalid UTF-8 byte sequences
        invalid_utf8 = b'\xff\xfe\xfd\xfc\xfb\xfa\xf9\xf8\xf7\xf6'

        frame = struct.pack('<I', header_len) + invalid_utf8 + b'x' * 100

        with pytest.raises(VoiceProtocolError, match="Invalid UTF-8"):
            parse_binary_frame(frame)

    def test_partial_utf8_character_at_boundary(self):
        """
        Test header ending mid-UTF8 character.

        Happens if header length is calculated wrong and cuts off
        a multi-byte UTF-8 character.
        """
        # UTF-8 for "こんにちは" (5 characters, 15 bytes)
        japanese_text = "こんにちは"
        full_utf8 = japanese_text.encode('utf-8')

        # Cut off last byte (invalidates last character)
        truncated_utf8 = full_utf8[:-1]
        header_len = len(truncated_utf8)

        frame = struct.pack('<I', header_len) + truncated_utf8

        with pytest.raises(VoiceProtocolError, match="Invalid UTF-8"):
            parse_binary_frame(frame)


class TestMalformedJSON:
    """Test handling of syntactically invalid JSON."""

    def test_json_with_trailing_comma(self):
        """Test JSON with trailing comma (invalid JSON)."""
        malformed_json = b'{"type": "Audio", "hash": 123,}'  # Trailing comma
        header_len = len(malformed_json)

        frame = struct.pack('<I', header_len) + malformed_json

        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_binary_frame(frame)

    def test_json_with_single_quotes(self):
        """Test JSON using single quotes instead of double (invalid)."""
        malformed_json = b"{'type': 'Audio'}"  # Single quotes invalid
        header_len = len(malformed_json)

        frame = struct.pack('<I', header_len) + malformed_json

        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_binary_frame(frame)

    def test_json_with_unquoted_keys(self):
        """Test JSON with unquoted keys (invalid)."""
        malformed_json = b'{type: "Audio"}'  # Unquoted key
        header_len = len(malformed_json)

        frame = struct.pack('<I', header_len) + malformed_json

        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_binary_frame(frame)

    def test_incomplete_json_object(self):
        """Test JSON object cut off mid-parse."""
        incomplete_json = b'{"type": "Audio", "hash":'  # No value, no closing brace
        header_len = len(incomplete_json)

        frame = struct.pack('<I', header_len) + incomplete_json

        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_binary_frame(frame)


class TestPCMDataErrors:
    """Test handling of invalid PCM audio data."""

    def test_odd_number_of_pcm_bytes(self):
        """
        Test PCM data with odd byte count.

        i16 samples require 2 bytes each. Odd count means data is corrupted.
        """
        header = {'type': 'Audio', 'audio_hash': 456}
        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)

        # Add 1 byte of PCM (odd number - invalid for i16)
        frame = struct.pack('<I', header_len) + header_bytes + b'\x00'

        with pytest.raises(VoiceProtocolError, match="Odd number of PCM bytes"):
            parse_binary_frame(frame)

    def test_single_byte_pcm_data(self):
        """Test with exactly 1 byte of PCM data (odd, invalid)."""
        header = {'type': 'Audio'}
        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)

        frame = struct.pack('<I', header_len) + header_bytes + b'\xff'

        with pytest.raises(VoiceProtocolError, match="Odd number of PCM bytes"):
            parse_binary_frame(frame)

    def test_empty_pcm_is_valid(self):
        """
        Test that 0 bytes of PCM data is VALID.

        This is silence detection - no audio is a valid case.
        """
        header = {'type': 'Audio', 'audio_hash': 0}
        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)

        # No PCM data after header
        frame = struct.pack('<I', header_len) + header_bytes

        # Should parse successfully
        parsed_header, parsed_samples = parse_binary_frame(frame)
        assert parsed_header['audio_hash'] == 0
        assert len(parsed_samples) == 0


class TestEdgeCaseSizes:
    """Test edge cases for frame sizes."""

    def test_maximum_reasonable_audio_size(self):
        """
        Test 30 seconds of audio at 48kHz (1,440,000 samples).

        This is the upper bound of reasonable audio chunk size.
        """
        header = {'type': 'Audio', 'audio_hash': 777}
        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)

        # 30s * 48kHz = 1,440,000 samples = 2,880,000 bytes
        large_samples = np.random.randint(-32768, 32767, size=1_440_000, dtype=np.int16)
        pcm_bytes = large_samples.tobytes()

        frame = struct.pack('<I', header_len) + header_bytes + pcm_bytes

        # Should parse successfully
        parsed_header, parsed_samples = parse_binary_frame(frame)
        assert len(parsed_samples) == 1_440_000

    def test_minimum_valid_audio_size(self):
        """Test 1 sample (2 bytes) - minimum valid audio."""
        header = {'type': 'Audio', 'audio_hash': 1}
        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)

        # Single sample
        single_sample = np.array([42], dtype=np.int16)
        pcm_bytes = single_sample.tobytes()

        frame = struct.pack('<I', header_len) + header_bytes + pcm_bytes

        parsed_header, parsed_samples = parse_binary_frame(frame)
        assert len(parsed_samples) == 1
        assert parsed_samples[0] == 42

    def test_very_large_header_length(self):
        """
        Test rejecting frames claiming gigabyte-sized headers.

        Prevents DoS attacks via memory exhaustion.
        """
        huge_header_len = 1_000_000_000  # 1GB

        frame = struct.pack('<I', huge_header_len) + b'x' * 1000

        with pytest.raises(VoiceProtocolError, match="Invalid header length"):
            parse_binary_frame(frame)


class TestTextFrameErrors:
    """Test error handling for legacy text frames."""

    def test_empty_text_frame(self):
        """Test empty text message."""
        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_text_frame('')

    def test_non_json_text_frame(self):
        """Test text frame with plain text (not JSON)."""
        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_text_frame('Hello world')

    def test_json_array_instead_of_object(self):
        """Test text frame containing JSON array instead of object."""
        # Some protocols expect objects, arrays might not be valid
        json_array = json.dumps([1, 2, 3])

        # This should parse as valid JSON, but might not have expected fields
        # Depends on your protocol - adjust assertion as needed
        parsed = parse_text_frame(json_array)
        assert isinstance(parsed, list)


if __name__ == '__main__':
    pytest.main([__file__, '-v'])

"""
Voice Protocol E2E Tests

Tests the binary WebSocket protocol for voice translation.
Ensures Rust bot and Python inference service can communicate correctly.
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
    create_result_response,
    create_error_response,
    create_pong_response,
    VoiceProtocolError,
)


class TestBinaryFrameParsing:
    """Test binary frame parsing (Rust → Python)."""

    def test_parse_valid_binary_frame(self):
        """Test parsing a valid binary frame with audio samples."""
        # Create test header
        header = {
            'type': 'Audio',
            'guild_id': '123456789',
            'channel_id': '987654321',
            'user_id': '111222333',
            'username': 'TestUser',
            'sample_rate': 48000,
            'target_language': 'en',
            'generate_tts': True,
            'audio_hash': 12345678901234567890,
        }

        header_json = json.dumps(header)
        header_bytes = header_json.encode('utf-8')
        header_len = len(header_bytes)

        # Create test audio samples
        test_samples = np.array([100, 200, 300, 400, 500], dtype=np.int16)
        pcm_bytes = test_samples.tobytes()

        # Build binary frame: [4-byte len][JSON][PCM]
        binary_frame = struct.pack('<I', header_len) + header_bytes + pcm_bytes

        # Parse
        parsed_header, parsed_samples = parse_binary_frame(binary_frame)

        # Verify header
        assert parsed_header['type'] == 'Audio'
        assert parsed_header['guild_id'] == '123456789'
        assert parsed_header['username'] == 'TestUser'
        assert parsed_header['audio_hash'] == 12345678901234567890

        # Verify samples
        assert np.array_equal(test_samples, parsed_samples)

    def test_parse_realistic_audio_size(self):
        """Test parsing a realistic audio frame (1.5 seconds at 48kHz)."""
        header = {
            'type': 'Audio',
            'guild_id': '123',
            'channel_id': '456',
            'user_id': '789',
            'username': 'RealisticUser',
            'sample_rate': 48000,
            'target_language': 'es',
            'generate_tts': False,
            'audio_hash': 9876543210987654321,
        }

        header_json = json.dumps(header)
        header_bytes = header_json.encode('utf-8')
        header_len = len(header_bytes)

        # 1.5 seconds at 48kHz = 72,000 samples
        realistic_samples = np.random.randint(-32768, 32767, size=72000, dtype=np.int16)
        pcm_bytes = realistic_samples.tobytes()

        binary_frame = struct.pack('<I', header_len) + header_bytes + pcm_bytes

        # Parse
        parsed_header, parsed_samples = parse_binary_frame(binary_frame)

        assert parsed_header['username'] == 'RealisticUser'
        assert len(parsed_samples) == 72000
        assert parsed_samples.dtype == np.int16

    def test_parse_empty_audio(self):
        """Test parsing a frame with zero audio samples (silence detection)."""
        header = {
            'type': 'Audio',
            'guild_id': '123',
            'channel_id': '456',
            'user_id': '789',
            'username': 'SilentUser',
            'sample_rate': 48000,
            'target_language': 'en',
            'generate_tts': False,
            'audio_hash': 0,
        }

        header_json = json.dumps(header)
        header_bytes = header_json.encode('utf-8')
        header_len = len(header_bytes)

        # No PCM data (empty audio)
        binary_frame = struct.pack('<I', header_len) + header_bytes

        # Parse
        parsed_header, parsed_samples = parse_binary_frame(binary_frame)

        assert parsed_header['username'] == 'SilentUser'
        assert len(parsed_samples) == 0

    def test_parse_frame_too_short(self):
        """Test error handling for truncated frame."""
        with pytest.raises(VoiceProtocolError, match="Binary frame too short"):
            parse_binary_frame(b'\x00\x00')

    def test_parse_invalid_header_length(self):
        """Test error handling for invalid header length."""
        # Header length claims 1MB (way too big)
        with pytest.raises(VoiceProtocolError, match="Invalid header length"):
            parse_binary_frame(struct.pack('<I', 1000000) + b'fake')

    def test_parse_frame_too_short_for_header(self):
        """Test error handling when frame is shorter than advertised header."""
        # Claims 100-byte header but only provides 10 bytes
        with pytest.raises(VoiceProtocolError, match="Frame too short for header"):
            binary_frame = struct.pack('<I', 100) + b'short'
            parse_binary_frame(binary_frame)

    def test_parse_invalid_utf8_header(self):
        """Test error handling for non-UTF-8 header."""
        header_len = 10
        invalid_utf8 = b'\xff\xfe\xfd\xfc\xfb\xfa\xf9\xf8\xf7\xf6'
        binary_frame = struct.pack('<I', header_len) + invalid_utf8

        with pytest.raises(VoiceProtocolError, match="Invalid UTF-8"):
            parse_binary_frame(binary_frame)

    def test_parse_invalid_json_header(self):
        """Test error handling for malformed JSON header."""
        header_bytes = b'{not valid json}'
        header_len = len(header_bytes)
        binary_frame = struct.pack('<I', header_len) + header_bytes

        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_binary_frame(binary_frame)

    def test_parse_odd_pcm_byte_count(self):
        """Test error handling for odd number of PCM bytes (i16 requires even)."""
        header = {'test': 'data'}
        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)

        # Add 1 byte (odd number) of PCM data
        binary_frame = struct.pack('<I', header_len) + header_bytes + b'\x00'

        with pytest.raises(VoiceProtocolError, match="Odd number of PCM bytes"):
            parse_binary_frame(binary_frame)


class TestTextFrameParsing:
    """Test text frame parsing (legacy/commands)."""

    def test_parse_valid_text_frame(self):
        """Test parsing a valid JSON text frame."""
        message = {
            'type': 'Ping',
            'timestamp': 1234567890,
        }
        text_frame = json.dumps(message)

        parsed = parse_text_frame(text_frame)

        assert parsed['type'] == 'Ping'
        assert parsed['timestamp'] == 1234567890

    def test_parse_invalid_json_text_frame(self):
        """Test error handling for invalid JSON in text frame."""
        with pytest.raises(VoiceProtocolError, match="Invalid JSON"):
            parse_text_frame('{not valid json}')


class TestResponseCreation:
    """Test response message creation (Python → Rust)."""

    def test_create_result_response(self):
        """Test creating a Result response with all fields."""
        response_json = create_result_response(
            guild_id="123",
            channel_id="456",
            user_id="789",
            username="TestUser",
            original_text="Hello world",
            translated_text="Hola mundo",
            source_language="en",
            target_language="es",
            tts_audio="base64encodedwav==",
            latency_ms=250,
            audio_hash=12345678901234567890,
        )

        response = json.loads(response_json)

        assert response['type'] == 'Result'
        assert response['guild_id'] == "123"
        assert response['username'] == "TestUser"
        assert response['original_text'] == "Hello world"
        assert response['translated_text'] == "Hola mundo"
        assert response['source_language'] == "en"
        assert response['target_language'] == "es"
        assert response['tts_audio'] == "base64encodedwav=="
        assert response['latency_ms'] == 250
        assert response['audio_hash'] == 12345678901234567890

    def test_create_result_response_without_tts(self):
        """Test creating a Result response without TTS audio."""
        response_json = create_result_response(
            guild_id="123",
            channel_id="456",
            user_id="789",
            username="TestUser",
            original_text="Bonjour",
            translated_text="Hello",
            source_language="fr",
            target_language="en",
            tts_audio=None,
            latency_ms=180,
            audio_hash=11111111111111111111,
        )

        response = json.loads(response_json)

        assert response['type'] == 'Result'
        assert response['tts_audio'] is None
        assert response['audio_hash'] == 11111111111111111111

    def test_create_error_response(self):
        """Test creating an Error response."""
        error_json = create_error_response("STT model failed", "STT_ERROR")
        error = json.loads(error_json)

        assert error['type'] == 'Error'
        assert error['message'] == "STT model failed"
        assert error['code'] == "STT_ERROR"

    def test_create_pong_response(self):
        """Test creating a Pong response."""
        pong_json = create_pong_response()
        pong = json.loads(pong_json)

        assert pong['type'] == 'Pong'


class TestAudioHashCorrelation:
    """Test audio hash correlation for caching."""

    def test_audio_hash_roundtrip(self):
        """Test that audio_hash is preserved through request/response cycle."""
        # Simulate Rust sending a binary frame with audio_hash
        audio_hash_from_rust = 9876543210123456789

        header = {
            'type': 'Audio',
            'guild_id': '111',
            'channel_id': '222',
            'user_id': '333',
            'username': 'CacheTestUser',
            'sample_rate': 48000,
            'target_language': 'ja',
            'generate_tts': False,
            'audio_hash': audio_hash_from_rust,
        }

        header_bytes = json.dumps(header).encode('utf-8')
        header_len = len(header_bytes)
        samples = np.array([1, 2, 3, 4, 5], dtype=np.int16)
        binary_frame = struct.pack('<I', header_len) + header_bytes + samples.tobytes()

        # Python parses the frame
        parsed_header, parsed_samples = parse_binary_frame(binary_frame)
        received_hash = parsed_header['audio_hash']

        # Python creates response (echoing back audio_hash)
        response_json = create_result_response(
            guild_id=parsed_header['guild_id'],
            channel_id=parsed_header['channel_id'],
            user_id=parsed_header['user_id'],
            username=parsed_header['username'],
            original_text="こんにちは",
            translated_text="Hello",
            source_language="ja",
            target_language="en",
            tts_audio=None,
            latency_ms=200,
            audio_hash=received_hash,  # Echo back
        )

        # Rust receives response
        response = json.loads(response_json)
        returned_hash = response['audio_hash']

        # Verify hash matches
        assert returned_hash == audio_hash_from_rust


class TestProtocolCompatibility:
    """Test backward compatibility with legacy text protocol."""

    def test_supports_both_binary_and_text(self):
        """Verify both binary and text protocols can be handled."""
        # Binary frame
        header = {'type': 'Audio', 'audio_hash': 123}
        header_bytes = json.dumps(header).encode('utf-8')
        samples = np.array([100, 200], dtype=np.int16)
        binary_frame = struct.pack('<I', len(header_bytes)) + header_bytes + samples.tobytes()

        parsed_header_binary, parsed_samples = parse_binary_frame(binary_frame)
        assert parsed_header_binary['audio_hash'] == 123

        # Text frame
        text_frame = json.dumps({'type': 'Ping'})
        parsed_text = parse_text_frame(text_frame)
        assert parsed_text['type'] == 'Ping'


if __name__ == '__main__':
    pytest.main([__file__, '-v'])

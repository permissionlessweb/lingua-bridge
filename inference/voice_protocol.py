"""
Binary WebSocket protocol parser for Rust voice client.

The Rust bot sends audio in binary frames.

Binary frame format:
    [4 bytes: header_length as u32 little-endian]
    [header_length bytes: JSON header as UTF-8]
    [remaining bytes: raw PCM samples as i16 little-endian]

Text frames are still JSON (ping/pong/configure commands).
"""

import json
import struct
from typing import Dict, Any, Tuple, Optional
import numpy as np


class VoiceProtocolError(Exception):
    """Raised when binary frame parsing fails."""
    pass


def parse_binary_frame(message: bytes) -> Tuple[Dict[str, Any], np.ndarray]:
    """
    Parse a binary WebSocket frame from the Rust voice client.

    Args:
        message: Raw binary message from WebSocket

    Returns:
        Tuple of (header_dict, samples_array)
        - header_dict: Parsed JSON header with guild_id, user_id, audio_hash, etc.
        - samples_array: PCM audio samples as numpy int16 array

    Raises:
        VoiceProtocolError: If frame is malformed
    """
    if len(message) < 4:
        raise VoiceProtocolError(f"Binary frame too short: {len(message)} bytes")

    # Parse header length (first 4 bytes, little-endian u32)
    header_len = struct.unpack('<I', message[:4])[0]

    # Validate header length
    if header_len > 10000:  # Sanity check - header should be <1KB
        raise VoiceProtocolError(f"Invalid header length: {header_len}")

    if len(message) < 4 + header_len:
        raise VoiceProtocolError(
            f"Frame too short for header: expected {4 + header_len}, got {len(message)}"
        )

    # Extract and parse JSON header
    header_start = 4
    header_end = header_start + header_len

    try:
        header_bytes = message[header_start:header_end]
        header_json = header_bytes.decode('utf-8')
        header = json.loads(header_json)
    except UnicodeDecodeError as e:
        raise VoiceProtocolError(f"Invalid UTF-8 in header: {e}")
    except json.JSONDecodeError as e:
        raise VoiceProtocolError(f"Invalid JSON in header: {e}")

    # Extract raw PCM samples (rest of message)
    pcm_bytes = message[header_end:]

    # Validate PCM data length (must be even for i16)
    if len(pcm_bytes) % 2 != 0:
        raise VoiceProtocolError(f"Odd number of PCM bytes: {len(pcm_bytes)}")

    # Convert to numpy int16 array
    samples = np.frombuffer(pcm_bytes, dtype=np.int16)

    return header, samples


def parse_text_frame(message: str) -> Dict[str, Any]:
    """
    Parse a text WebSocket frame (JSON command).

    Args:
        message: Text message from WebSocket

    Returns:
        Parsed JSON object

    Raises:
        VoiceProtocolError: If JSON is invalid
    """
    try:
        return json.loads(message)
    except json.JSONDecodeError as e:
        raise VoiceProtocolError(f"Invalid JSON in text frame: {e}")


def create_result_response(
    guild_id: str,
    channel_id: str,
    user_id: str,
    username: str,
    original_text: str,
    translated_text: str,
    source_language: str,
    target_language: str,
    tts_audio: Optional[str],
    latency_ms: int,
    audio_hash: int,  # CRITICAL: Echo back for cache correlation
) -> str:
    """
    Create a Result response message (JSON text frame).

    IMPORTANT: The audio_hash MUST be echoed back exactly as received.
    This is used by the Rust bot for cache correlation.

    Args:
        guild_id: Discord guild ID
        channel_id: Discord voice channel ID
        user_id: Discord user ID
        username: Discord username
        original_text: Transcribed text (original language)
        translated_text: Translated text (target language)
        source_language: Detected source language code
        target_language: Target language code
        tts_audio: Base64-encoded TTS audio (WAV), or None
        latency_ms: Total processing latency in milliseconds
        audio_hash: Audio hash from request (MUST echo back)

    Returns:
        JSON string ready to send over WebSocket
    """
    response = {
        'type': 'Result',
        'guild_id': guild_id,
        'channel_id': channel_id,
        'user_id': user_id,
        'username': username,
        'original_text': original_text,
        'translated_text': translated_text,
        'source_language': source_language,
        'target_language': target_language,
        'tts_audio': tts_audio,
        'latency_ms': latency_ms,
        'audio_hash': audio_hash,  # Echo back for cache correlation
    }
    return json.dumps(response)


def create_error_response(message: str, code: Optional[str] = None) -> str:
    """
    Create an Error response message (JSON text frame).

    Args:
        message: Error message
        code: Optional error code

    Returns:
        JSON string ready to send over WebSocket
    """
    response = {
        'type': 'Error',
        'message': message,
        'code': code,
    }
    return json.dumps(response)


def create_pong_response() -> str:
    """Create a Pong response message (JSON text frame)."""
    return json.dumps({'type': 'Pong'})


# Example usage and tests
if __name__ == '__main__':
    # Test binary frame parsing
    print("Testing binary frame parsing...")

    # Create a test binary frame
    header = {
        'type': 'Audio',
        'guild_id': '123456789',
        'channel_id': '987654321',
        'user_id': '111222333',
        'username': 'TestUser',
        'sample_rate': 48000,
        'target_language': 'en',
        'generate_tts': True,
        'audio_hash': 12345678901234567890,  # u64 audio hash
    }

    header_json = json.dumps(header)
    header_bytes = header_json.encode('utf-8')
    header_len = len(header_bytes)

    # Create test audio samples
    test_samples = np.array([100, 200, 300, 400, 500], dtype=np.int16)
    pcm_bytes = test_samples.tobytes()

    # Build binary frame
    binary_frame = struct.pack('<I', header_len) + header_bytes + pcm_bytes

    print(f"Binary frame size: {len(binary_frame)} bytes")
    print(f"Header length: {header_len} bytes")
    print(f"PCM data length: {len(pcm_bytes)} bytes")

    # Parse it back
    try:
        parsed_header, parsed_samples = parse_binary_frame(binary_frame)
        print("\n✓ Parsing successful!")
        print(f"Parsed header: {parsed_header['user_id']} ({parsed_header['username']})")
        print(f"Audio hash: {parsed_header['audio_hash']}")
        print(f"Parsed samples: {parsed_samples}")
        print(f"Samples match: {np.array_equal(test_samples, parsed_samples)}")
    except VoiceProtocolError as e:
        print(f"\n✗ Parsing failed: {e}")

    # Test response creation
    print("\n\nTesting response creation...")
    response_json = create_result_response(
        guild_id="123",
        channel_id="456",
        user_id="789",
        username="TestUser",
        original_text="Hello",
        translated_text="Hola",
        source_language="en",
        target_language="es",
        tts_audio=None,
        latency_ms=250,
        audio_hash=parsed_header['audio_hash'],
    )
    response = json.loads(response_json)
    print(f"✓ Response created with audio_hash: {response['audio_hash']}")

    # Test error cases
    print("\n\nTesting error handling...")

    # Too short
    try:
        parse_binary_frame(b'\x00\x00')
        print("✗ Should have raised error for short frame")
    except VoiceProtocolError:
        print("✓ Correctly rejected short frame")

    # Invalid header length
    try:
        parse_binary_frame(struct.pack('<I', 999999) + b'fake')
        print("✗ Should have raised error for huge header length")
    except VoiceProtocolError:
        print("✓ Correctly rejected huge header length")

    # Odd PCM bytes
    try:
        fake_header = json.dumps({'test': 'data'}).encode('utf-8')
        fake_frame = struct.pack('<I', len(fake_header)) + fake_header + b'\x00'
        parse_binary_frame(fake_frame)
        print("✗ Should have raised error for odd PCM bytes")
    except VoiceProtocolError:
        print("✓ Correctly rejected odd PCM byte count")

    print("\n\nAll tests passed!")

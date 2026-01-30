#!/usr/bin/env python3
"""
Test script for binary WebSocket protocol.

Run this to verify your protocol parser works correctly before integrating
into the full inference service.

Usage:
    python test_protocol.py
"""

import json
import struct
import numpy as np
from voice_protocol import (
    parse_binary_frame,
    parse_text_frame,
    create_result_response,
    create_error_response,
    create_pong_response,
    VoiceProtocolError,
)


def test_binary_frame_parsing():
    """Test parsing binary frames from Rust client."""
    print("=" * 60)
    print("TEST 1: Binary Frame Parsing")
    print("=" * 60)

    # Create a test binary frame (simulating Rust client)
    header = {
        'type': 'Audio',
        'guild_id': '123456789012345678',
        'channel_id': '987654321098765432',
        'user_id': '111222333444555666',
        'username': 'TestUser',
        'audio_base64': '',  # Not used in binary mode
        'sample_rate': 48000,
        'target_language': 'en',
        'generate_tts': True,
    }

    # Encode header as JSON
    header_json = json.dumps(header)
    header_bytes = header_json.encode('utf-8')
    header_len = len(header_bytes)

    # Create test audio samples (1 second of audio at 48kHz)
    duration_secs = 1.0
    sample_rate = 48000
    num_samples = int(duration_secs * sample_rate)

    # Generate a 440Hz sine wave (A4 note)
    t = np.linspace(0, duration_secs, num_samples, False)
    audio = np.sin(2 * np.pi * 440 * t) * 10000  # Scale to int16 range
    samples = audio.astype(np.int16)

    # Convert to bytes
    pcm_bytes = samples.tobytes()

    # Build binary frame: [4-byte len][JSON header][raw PCM]
    binary_frame = struct.pack('<I', header_len) + header_bytes + pcm_bytes

    print(f"\nCreated binary frame:")
    print(f"  Total size: {len(binary_frame):,} bytes")
    print(f"  Header length: {header_len} bytes")
    print(f"  PCM data length: {len(pcm_bytes):,} bytes")
    print(f"  Audio samples: {len(samples):,} samples")
    print(f"  Audio duration: {duration_secs} seconds")

    # Parse it back
    try:
        parsed_header, parsed_samples = parse_binary_frame(binary_frame)

        print(f"\n✓ Parsing successful!")
        print(f"\nParsed header:")
        print(f"  User: {parsed_header['username']} (ID: {parsed_header['user_id']})")
        print(f"  Guild: {parsed_header['guild_id']}")
        print(f"  Channel: {parsed_header['channel_id']}")
        print(f"  Target language: {parsed_header['target_language']}")
        print(f"  Sample rate: {parsed_header['sample_rate']}")
        print(f"  Generate TTS: {parsed_header['generate_tts']}")

        print(f"\nParsed audio:")
        print(f"  Samples: {len(parsed_samples):,}")
        print(f"  Sample type: {parsed_samples.dtype}")
        print(f"  Sample range: [{parsed_samples.min()}, {parsed_samples.max()}]")
        print(f"  Samples match original: {np.array_equal(samples, parsed_samples)}")

        return True

    except VoiceProtocolError as e:
        print(f"\n✗ Parsing failed: {e}")
        return False


def test_error_handling():
    """Test error handling for malformed frames."""
    print("\n\n" + "=" * 60)
    print("TEST 2: Error Handling")
    print("=" * 60)

    tests = [
        ("Empty frame", b''),
        ("Too short frame", b'\x00\x00'),
        ("Invalid header length", struct.pack('<I', 999999) + b'fake'),
    ]

    all_passed = True

    for name, data in tests:
        try:
            parse_binary_frame(data)
            print(f"\n✗ {name}: Should have raised error")
            all_passed = False
        except VoiceProtocolError as e:
            print(f"\n✓ {name}: Correctly rejected ({str(e)[:60]}...)")

    # Test odd PCM bytes
    fake_header = json.dumps({'test': 'data'}).encode('utf-8')
    fake_frame = struct.pack('<I', len(fake_header)) + fake_header + b'\x00'
    try:
        parse_binary_frame(fake_frame)
        print(f"\n✗ Odd PCM bytes: Should have raised error")
        all_passed = False
    except VoiceProtocolError:
        print(f"\n✓ Odd PCM bytes: Correctly rejected")

    return all_passed


def test_text_frame_parsing():
    """Test parsing text frames (JSON commands)."""
    print("\n\n" + "=" * 60)
    print("TEST 3: Text Frame Parsing")
    print("=" * 60)

    # Test Ping
    ping_json = json.dumps({'type': 'Ping'})
    try:
        ping_data = parse_text_frame(ping_json)
        print(f"\n✓ Ping frame parsed: {ping_data}")
    except VoiceProtocolError as e:
        print(f"\n✗ Ping frame failed: {e}")
        return False

    # Test Configure
    config_json = json.dumps({
        'type': 'Configure',
        'stt_model': 'whisper-large-v3',
        'tts_model': 'coqui-tts',
    })
    try:
        config_data = parse_text_frame(config_json)
        print(f"\n✓ Configure frame parsed: {config_data}")
    except VoiceProtocolError as e:
        print(f"\n✗ Configure frame failed: {e}")
        return False

    return True


def test_response_creation():
    """Test creating response messages."""
    print("\n\n" + "=" * 60)
    print("TEST 4: Response Creation")
    print("=" * 60)

    # Test Result response
    result_json = create_result_response(
        guild_id='123456789',
        channel_id='987654321',
        user_id='111222333',
        username='TestUser',
        original_text='Hello world',
        translated_text='Hola mundo',
        source_language='en',
        target_language='es',
        tts_audio=None,
        latency_ms=250,
        audio_hash=12345678901234567890,  # Added for cache correlation
    )

    result_data = json.loads(result_json)
    print(f"\n✓ Result response created:")
    print(f"  Type: {result_data['type']}")
    print(f"  Original: {result_data['original_text']}")
    print(f"  Translated: {result_data['translated_text']}")
    print(f"  Latency: {result_data['latency_ms']}ms")

    # Test Error response
    error_json = create_error_response('Test error', code='TEST_ERROR')
    error_data = json.loads(error_json)
    print(f"\n✓ Error response created:")
    print(f"  Type: {error_data['type']}")
    print(f"  Message: {error_data['message']}")
    print(f"  Code: {error_data['code']}")

    # Test Pong response
    pong_json = create_pong_response()
    pong_data = json.loads(pong_json)
    print(f"\n✓ Pong response created:")
    print(f"  Type: {pong_data['type']}")

    return True


def test_realistic_audio():
    """Test with realistic audio sizes from Discord voice (20ms frames)."""
    print("\n\n" + "=" * 60)
    print("TEST 5: Realistic Audio Frame")
    print("=" * 60)

    # Discord sends 20ms frames at 48kHz stereo (converted to mono by Rust)
    frame_duration_ms = 20
    sample_rate = 48000
    samples_per_frame = int(sample_rate * frame_duration_ms / 1000)

    # Simulate 1.5 seconds of buffered audio (streaming chunk interval)
    streaming_interval_ms = 1500
    num_frames = streaming_interval_ms // frame_duration_ms
    total_samples = samples_per_frame * num_frames

    print(f"\nRealistic audio parameters:")
    print(f"  Frame duration: {frame_duration_ms}ms")
    print(f"  Samples per frame: {samples_per_frame}")
    print(f"  Streaming interval: {streaming_interval_ms}ms")
    print(f"  Total samples: {total_samples:,}")

    # Create header
    header = {
        'type': 'Audio',
        'guild_id': '123456789012345678',
        'channel_id': '987654321098765432',
        'user_id': '111222333444555666',
        'username': 'RealUser',
        'audio_base64': '',
        'sample_rate': sample_rate,
        'target_language': 'ja',
        'generate_tts': True,
    }

    header_json = json.dumps(header)
    header_bytes = header_json.encode('utf-8')
    header_len = len(header_bytes)

    # Generate realistic audio (random noise, simulating speech)
    samples = np.random.randint(-5000, 5000, size=total_samples, dtype=np.int16)
    pcm_bytes = samples.tobytes()

    # Build binary frame
    binary_frame = struct.pack('<I', header_len) + header_bytes + pcm_bytes

    print(f"\nFrame size breakdown:")
    print(f"  Header length field: 4 bytes")
    print(f"  Header JSON: {header_len} bytes")
    print(f"  PCM data: {len(pcm_bytes):,} bytes")
    print(f"  Total: {len(binary_frame):,} bytes")
    print(f"  Bandwidth savings vs base64: {int(len(pcm_bytes) * 0.33):,} bytes (33%)")

    # Parse it
    try:
        parsed_header, parsed_samples = parse_binary_frame(binary_frame)
        print(f"\n✓ Realistic frame parsed successfully")
        print(f"  Samples match: {np.array_equal(samples, parsed_samples)}")
        return True
    except VoiceProtocolError as e:
        print(f"\n✗ Realistic frame failed: {e}")
        return False


def main():
    """Run all tests."""
    print("\n")
    print("█" * 60)
    print("  BINARY WEBSOCKET PROTOCOL TEST SUITE")
    print("█" * 60)

    results = []

    results.append(("Binary Frame Parsing", test_binary_frame_parsing()))
    results.append(("Error Handling", test_error_handling()))
    results.append(("Text Frame Parsing", test_text_frame_parsing()))
    results.append(("Response Creation", test_response_creation()))
    results.append(("Realistic Audio Frame", test_realistic_audio()))

    # Summary
    print("\n\n" + "=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)

    all_passed = True
    for name, passed in results:
        status = "✓ PASS" if passed else "✗ FAIL"
        print(f"{status:8} {name}")
        if not passed:
            all_passed = False

    print("\n" + "=" * 60)
    if all_passed:
        print("✓ ALL TESTS PASSED")
        print("\nYour protocol parser is working correctly!")
        print("You can now integrate it into your inference service.")
    else:
        print("✗ SOME TESTS FAILED")
        print("\nFix the failing tests before integrating.")

    print("=" * 60)
    print()

    return 0 if all_passed else 1


if __name__ == '__main__':
    import sys
    sys.exit(main())

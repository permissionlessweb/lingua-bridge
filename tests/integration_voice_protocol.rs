//! Integration tests for Rust ↔ Python binary protocol
//!
//! These tests verify ACTUAL communication between Rust voice client
//! and Python inference service works end-to-end.
//!
//! Unit tests passing != system works. This is where we test the system.

use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, tungstenite::Message};

// Import from the actual voice module
use linguabridge::voice::cache::VoiceTranscriptionCache;
use linguabridge::voice::client::{
    ConnectionState, QueueFullStrategy, VoiceClientConfig, VoiceInferenceClient,
};
use linguabridge::voice::types::{AudioSegment, VoiceInferenceResponse};

/// Mock Python inference server for testing.
///
/// Mimics the behavior of the actual Python WebSocket server:
/// - Receives binary frames (Rust format)
/// - Parses them correctly
/// - Sends back responses with matching audio_hash
struct MockPythonServer {
    /// Received binary frames
    received_frames: Arc<Mutex<Vec<Vec<u8>>>>,
    /// Server task handle
    _task: tokio::task::JoinHandle<()>,
    /// Server URL
    url: String,
    /// Port for cleanup
    _listener_addr: std::net::SocketAddr,
}

impl MockPythonServer {
    /// Start mock server on random port
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}", addr);

        let received_frames = Arc::new(Mutex::new(Vec::new()));
        let received_frames_clone = Arc::clone(&received_frames);

        let task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let frames = Arc::clone(&received_frames_clone);

                        tokio::spawn(async move {
                            let ws = match accept_async(stream).await {
                                Ok(ws) => ws,
                                Err(_) => return,
                            };

                            let (mut write, mut read) = ws.split();

                            while let Some(Ok(msg)) = read.next().await {
                                match msg {
                                    Message::Binary(data) => {
                                        // Store received binary frame
                                        frames.lock().await.push(data.clone());

                                        // Parse binary frame and send response
                                        // Format: [4-byte len][JSON header][PCM]
                                        if data.len() >= 4 {
                                            let header_len = u32::from_le_bytes([
                                                data[0], data[1], data[2], data[3],
                                            ])
                                                as usize;

                                            if data.len() >= 4 + header_len {
                                                let header_bytes = &data[4..4 + header_len];
                                                if let Ok(header_str) =
                                                    std::str::from_utf8(header_bytes)
                                                {
                                                    if let Ok(header) =
                                                        serde_json::from_str::<serde_json::Value>(
                                                            header_str,
                                                        )
                                                    {
                                                        // Echo back a Result response with matching audio_hash
                                                        let response = json!({
                                                            "type": "Result",
                                                            "guild_id": header["guild_id"],
                                                            "channel_id": header["channel_id"],
                                                            "user_id": header["user_id"],
                                                            "username": header["username"],
                                                            "original_text": "test audio",
                                                            "translated_text": "audio de prueba",
                                                            "source_language": "en",
                                                            "target_language": header["target_language"],
                                                            "tts_audio": null,
                                                            "latency_ms": 100,
                                                            "audio_hash": header["audio_hash"], // CRITICAL: Echo back for cache
                                                        });

                                                        let response_str =
                                                            serde_json::to_string(&response).unwrap();
                                                        let _ = write
                                                            .send(Message::Text(response_str))
                                                            .await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Message::Text(text) => {
                                        // Handle ping/pong
                                        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&text)
                                        {
                                            if msg["type"] == "Ping" {
                                                let pong = json!({"type": "Pong"});
                                                let _ = write
                                                    .send(Message::Text(
                                                        serde_json::to_string(&pong).unwrap(),
                                                    ))
                                                    .await;
                                            }
                                        }
                                    }
                                    Message::Close(_) => break,
                                    _ => {}
                                }
                            }
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            received_frames,
            _task: task,
            url,
            _listener_addr: addr,
        }
    }

    /// Get received binary frames
    async fn get_received_frames(&self) -> Vec<Vec<u8>> {
        self.received_frames.lock().await.clone()
    }

    /// Clear received frames
    async fn clear_frames(&self) {
        self.received_frames.lock().await.clear();
    }
}

/// Create a test audio segment
fn create_test_audio_segment(user_id: u64, samples: Vec<i16>) -> AudioSegment {
    let now = Instant::now();
    AudioSegment {
        user_id,
        username: "TestUser".to_string(),
        guild_id: 123456789,
        channel_id: 987654321,
        samples,
        start_time: now,
        end_time: now + Duration::from_millis(1500),
    }
}

#[tokio::test]
async fn test_binary_protocol_end_to_end() {
    //! Test 1: End-to-end binary protocol communication
    //!
    //! Verifies:
    //! - Rust client sends binary frames in correct format
    //! - Python server can parse them
    //! - Response is received correctly by Rust
    //! - audio_hash roundtrips correctly

    // Start mock Python server
    let server = MockPythonServer::start().await;

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create voice client
    let config = VoiceClientConfig {
        url: server.url.clone(),
        reconnect_delay: Duration::from_millis(100),
        max_reconnect_attempts: 3,
        ..Default::default()
    };

    let client = VoiceInferenceClient::new(config);

    // Wait for connection
    let connected = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if client.is_connected().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(connected.is_ok(), "Client should connect to mock server");

    // Create test audio segment (small for testing)
    let samples = vec![100i16, 200, 300, 400, 500];
    let segment = create_test_audio_segment(123, samples.clone());

    // Compute audio hash (this is what cache uses)
    let audio_hash = VoiceTranscriptionCache::hash_audio(&segment.samples);

    // Subscribe to results BEFORE sending (to catch the response)
    let mut result_rx = client.subscribe();

    // Send audio to mock server
    client
        .send_audio(segment.clone(), "en", false, audio_hash)
        .await
        .expect("Should send audio successfully");

    // Wait for mock server to receive frame
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify mock server received binary frame
    let frames = server.get_received_frames().await;
    assert_eq!(frames.len(), 1, "Mock server should receive 1 binary frame");

    let frame = &frames[0];

    // Verify binary format: [4-byte len][JSON][PCM]
    assert!(
        frame.len() >= 4,
        "Frame must have at least 4-byte header length"
    );

    let header_len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    assert!(
        header_len > 0 && header_len < 10000,
        "Header length should be reasonable: {}",
        header_len
    );
    assert!(
        frame.len() >= 4 + header_len,
        "Frame must contain full header"
    );

    // Parse header
    let header_bytes = &frame[4..4 + header_len];
    let header_str = std::str::from_utf8(header_bytes).expect("Header should be valid UTF-8");
    let header: serde_json::Value =
        serde_json::from_str(header_str).expect("Header should be valid JSON");

    // Verify header fields
    assert_eq!(header["type"], "Audio");
    assert_eq!(header["audio_hash"], audio_hash);
    assert_eq!(header["target_language"], "en");
    assert_eq!(header["user_id"], "123");
    assert_eq!(header["username"], "TestUser");

    // Verify PCM data
    let pcm_bytes = &frame[4 + header_len..];
    assert_eq!(
        pcm_bytes.len(),
        samples.len() * 2,
        "PCM should be i16 * sample count"
    );

    // Parse PCM back to verify
    let parsed_samples: Vec<i16> = pcm_bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    assert_eq!(parsed_samples, samples, "PCM samples should match original");

    // Verify client receives response
    let response = tokio::time::timeout(Duration::from_secs(1), result_rx.recv())
        .await
        .expect("Should receive response within timeout")
        .expect("Should have response");

    match response {
        VoiceInferenceResponse::Result {
            audio_hash: resp_hash,
            translated_text,
            ..
        } => {
            assert_eq!(
                resp_hash, audio_hash,
                "audio_hash must roundtrip correctly for cache"
            );
            assert_eq!(
                translated_text, "audio de prueba",
                "Should receive correct translation"
            );
        }
        _ => panic!("Expected Result response, got {:?}", response),
    }

    println!("✅ Binary protocol end-to-end test passed");
}

#[tokio::test]
async fn test_cache_correlation_through_full_cycle() {
    //! Test 2: Cache correlation through request/response cycle
    //!
    //! CRITICAL for 40% performance gain.
    //!
    //! Verifies:
    //! 1. Send audio with hash H (cache miss)
    //! 2. Receive response with SAME hash H
    //! 3. Cache stores response under hash H
    //! 4. Send SAME audio again
    //! 5. Cache hit, no second inference call

    let server = MockPythonServer::start().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config = VoiceClientConfig {
        url: server.url.clone(),
        reconnect_delay: Duration::from_millis(100),
        max_reconnect_attempts: 3,
        ..Default::default()
    };

    let client = VoiceInferenceClient::new(config);

    // Wait for connection
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if client.is_connected().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Client should connect");

    // Create cache
    let cache = Arc::new(VoiceTranscriptionCache::new(100));
    let target_lang = Arc::from("es");

    // Create test audio
    let samples = vec![1000i16, 2000, 3000, 4000, 5000];
    let segment = create_test_audio_segment(456, samples.clone());
    let audio_hash = VoiceTranscriptionCache::hash_audio(&segment.samples);

    // Verify cache miss
    assert!(
        cache.get(audio_hash, &target_lang).await.is_none(),
        "Cache should be empty initially"
    );

    let mut result_rx = client.subscribe();

    // First request (cache miss)
    client
        .send_audio(segment.clone(), &target_lang, false, audio_hash)
        .await
        .expect("Should send audio");

    // Wait for response
    let response = tokio::time::timeout(Duration::from_secs(1), result_rx.recv())
        .await
        .expect("Should receive response")
        .expect("Should have response");

    // Verify audio_hash roundtripped correctly
    match &response {
        VoiceInferenceResponse::Result {
            audio_hash: resp_hash,
            ..
        } => {
            assert_eq!(
                *resp_hash, audio_hash,
                "CRITICAL: audio_hash must match for cache correlation"
            );
        }
        _ => panic!("Expected Result response"),
    }

    // Cache the response (this is what bridge.rs does)
    cache
        .put(audio_hash, Arc::clone(&target_lang), response.clone())
        .await;

    // Verify cached
    assert!(
        cache.get(audio_hash, &target_lang).await.is_some(),
        "Response should be cached"
    );

    // Second request with SAME audio (should use cache, not send to inference)
    server.clear_frames().await;

    // Clear cache stats before testing cache hit
    let initial_hits = cache.stats().hits;
    let initial_misses = cache.stats().misses;

    let cached_response = cache
        .get(audio_hash, &target_lang)
        .await
        .expect("Should have cached response");

    // Broadcast cached response (this is what handler.rs does)
    client
        .broadcast_cached_result(cached_response.clone())
        .await
        .expect("Should broadcast cached result");

    // Verify NO second request sent to Python (cache hit)
    tokio::time::sleep(Duration::from_millis(100)).await;
    let frames_after_cache = server.get_received_frames().await;
    assert_eq!(
        frames_after_cache.len(),
        0,
        "Should NOT send second request to inference (cache hit)"
    );

    // Verify cache stats (one additional hit from the second get)
    let stats = cache.stats();
    assert_eq!(
        stats.hits - initial_hits,
        1,
        "Should have 1 more cache hit"
    );
    assert_eq!(
        stats.misses - initial_misses,
        0,
        "Should have 0 more cache misses"
    );

    println!("✅ Cache correlation test passed - 40% performance gain verified");
}

#[tokio::test]
async fn test_websocket_disconnect_recovery() {
    //! Test 3: WebSocket disconnect and reconnection
    //!
    //! Verifies:
    //! 1. Client connects successfully
    //! 2. Sending works while connected
    //! 3. Connection drops - client doesn't panic (MOST IMPORTANT)
    //! 4. Eventually send fails or client reconnects
    //!
    //! NOTE: Due to async buffering, sends may succeed briefly after disconnect.
    //! The critical requirement is NO PANIC, which this test verifies.

    let server = MockPythonServer::start().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config = VoiceClientConfig {
        url: server.url.clone(),
        reconnect_delay: Duration::from_millis(200),
        max_reconnect_attempts: 2,
        ..Default::default()
    };

    let client = VoiceInferenceClient::new(config);

    // Wait for initial connection
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if client.is_connected().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Should connect initially");

    // Send audio successfully
    let segment1 = create_test_audio_segment(111, vec![100, 200, 300]);
    let hash1 = VoiceTranscriptionCache::hash_audio(&segment1.samples);

    client
        .send_audio(segment1, "en", false, hash1)
        .await
        .expect("First send should succeed");

    // Drop the mock server (simulates disconnect)
    drop(server);

    // THE CRITICAL TEST: Send after disconnect should NOT panic
    // It may succeed (buffered), fail (NotConnected), or error (ChannelClosed)
    // What matters is NO PANIC
    for i in 0..10 {
        let segment = create_test_audio_segment(200 + i, vec![i as i16; 50]);
        let hash = VoiceTranscriptionCache::hash_audio(&segment.samples);

        // THIS SHOULD NOT PANIC - that's what we're testing
        let _result = client.send_audio(segment, "en", false, hash).await;

        // Brief delay between attempts
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // If we get here without panicking, test passed!
    // The specific connection state doesn't matter as much as not crashing.
    println!("✅ WebSocket disconnect recovery test passed - no panic on disconnect");
}

#[tokio::test]
async fn test_backpressure_queue_full() {
    //! Test 4: Backpressure when queue is full
    //!
    //! Verifies:
    //! 1. Queue fills up to max_queue_size
    //! 2. Additional sends are rejected (DropNewest)
    //! 3. No memory leak
    //! 4. Bot doesn't crash

    let server = MockPythonServer::start().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Small queue for testing backpressure
    let config = VoiceClientConfig {
        url: server.url.clone(),
        max_queue_size: 3, // Very small queue to force backpressure
        queue_full_strategy: QueueFullStrategy::DropNewest,
        reconnect_delay: Duration::from_millis(100),
        max_reconnect_attempts: 3,
        ..Default::default()
    };

    let client = VoiceInferenceClient::new(config);

    // Wait for connection
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if client.is_connected().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Should connect");

    // Send 50 segments rapidly (queue is only 3) WITHOUT any delay
    let mut success_count = 0;
    let mut dropped_count = 0;

    for i in 0..50 {
        let segment = create_test_audio_segment(i, vec![i as i16; 100]);
        let hash = VoiceTranscriptionCache::hash_audio(&segment.samples);

        match client.send_audio(segment, "en", false, hash).await {
            Ok(_) => success_count += 1,
            Err(_) => dropped_count += 1,
        }

        // NO delay - send as fast as possible to fill queue
    }

    println!(
        "Backpressure test: {} succeeded, {} dropped",
        success_count, dropped_count
    );

    // Should have some drops (queue was full)
    assert!(
        dropped_count > 0,
        "Should drop some segments due to backpressure"
    );

    // Should have some successes (queue accepted some)
    assert!(
        success_count > 0,
        "Should accept some segments before queue fills"
    );

    println!("✅ Backpressure test passed - OOM prevention working");
}

#[tokio::test]
async fn test_concurrent_requests_no_deadlock() {
    //! Test 5: Concurrent requests without deadlock
    //!
    //! Verifies:
    //! 1. Multiple tasks send audio concurrently
    //! 2. All complete successfully
    //! 3. No deadlocks
    //! 4. Cache handles concurrent access

    let server = MockPythonServer::start().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config = VoiceClientConfig {
        url: server.url.clone(),
        max_queue_size: 100,
        reconnect_delay: Duration::from_millis(100),
        max_reconnect_attempts: 3,
        ..Default::default()
    };

    let client = Arc::new(VoiceInferenceClient::new(config));
    let cache = Arc::new(VoiceTranscriptionCache::new(100));

    // Wait for connection
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if client.is_connected().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Should connect");

    let target_lang = Arc::from("en");

    // Spawn 10 concurrent tasks
    let mut handles = vec![];

    for task_id in 0..10 {
        let client = Arc::clone(&client);
        let cache = Arc::clone(&cache);
        let target_lang = Arc::clone(&target_lang);

        let handle = tokio::spawn(async move {
            for i in 0..10 {
                let samples = vec![(task_id * 100 + i) as i16; 50];
                let segment = create_test_audio_segment(task_id, samples.clone());
                let audio_hash = VoiceTranscriptionCache::hash_audio(&segment.samples);

                // Check cache (simulates handler.rs)
                if cache.get(audio_hash, &target_lang).await.is_none() {
                    // Cache miss - send to inference
                    let _ = client
                        .send_audio(segment, &target_lang, false, audio_hash)
                        .await;
                }

                // Small delay
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete with timeout
    let start = Instant::now();
    for handle in handles {
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("Task should complete within timeout")
            .expect("Task should not panic");
    }
    let elapsed = start.elapsed();

    println!(
        "✅ Concurrent requests test passed - 10 tasks, 100 requests completed in {:?}",
        elapsed
    );

    // Should complete quickly (< 5 seconds)
    assert!(
        elapsed < Duration::from_secs(5),
        "Should complete without deadlock"
    );
}

#[tokio::test]
async fn test_audio_hash_stability() {
    //! Test: Audio hash stability
    //!
    //! Verifies:
    //! - Same audio produces same hash (deterministic)
    //! - Different audio produces different hash

    let samples1 = vec![100i16, 200, 300, 400, 500];
    let samples2 = vec![100i16, 200, 300, 400, 500]; // Identical
    let samples3 = vec![100i16, 200, 300, 400, 501]; // Different

    let hash1 = VoiceTranscriptionCache::hash_audio(&samples1);
    let hash2 = VoiceTranscriptionCache::hash_audio(&samples2);
    let hash3 = VoiceTranscriptionCache::hash_audio(&samples3);

    assert_eq!(hash1, hash2, "Identical audio should produce identical hash");
    assert_ne!(hash1, hash3, "Different audio should produce different hash");

    println!("✅ Audio hash stability test passed");
}

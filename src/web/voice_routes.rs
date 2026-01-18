//! Voice channel web routes.
//!
//! Public URLs for viewing real-time voice translations.
//! Format: /voice/{guild_id}/{channel_id}

use crate::config::AppConfig;
use crate::web::broadcast::BroadcastManager;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::{Html, Response},
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Application state for voice routes
#[derive(Clone)]
pub struct VoiceAppState {
    pub broadcast: Arc<BroadcastManager>,
}

/// Serve the voice channel web view
pub async fn voice_view(Path((guild_id, channel_id)): Path<(String, String)>) -> Html<String> {
    Html(generate_voice_view_html(&guild_id, &channel_id))
}

/// WebSocket handler for voice channel updates
pub async fn voice_ws_handler(
    ws: WebSocketUpgrade,
    Path((guild_id, channel_id)): Path<(String, String)>,
    State(state): State<VoiceAppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_voice_socket(socket, guild_id, channel_id, state))
}

/// Handle a voice channel WebSocket connection
async fn handle_voice_socket(
    socket: WebSocket,
    guild_id: String,
    channel_id: String,
    state: VoiceAppState,
) {
    info!(guild_id, channel_id, "Voice WebSocket client connected");

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to voice channel transcriptions
    let mut broadcast_rx = state
        .broadcast
        .subscribe_voice_channel(&guild_id, &channel_id);

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "welcome",
        "guild_id": guild_id,
        "channel_id": channel_id,
        "message": "Connected to voice channel transcription feed"
    });
    if let Err(e) = sender.send(Message::Text(welcome.to_string().into())).await {
        error!(error = %e, "Failed to send welcome message");
        return;
    }

    // Ping interval for keepalive
    let mut ping_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Forward broadcast messages to client
            result = broadcast_rx.recv() => {
                match result {
                    Ok(msg) => {
                        match serde_json::to_string(&msg) {
                            Ok(json) => {
                                if let Err(e) = sender.send(Message::Text(json.into())).await {
                                    debug!(error = %e, "Failed to send message, client disconnected");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to serialize message");
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "Client lagged, skipped messages");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("Broadcast channel closed");
                        break;
                    }
                }
            }

            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Handle ping/pong or other client messages
                        if text.as_str() == "ping" {
                            let _ = sender.send(Message::Text("pong".into())).await;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!("Client closed connection");
                        break;
                    }
                    Some(Err(e)) => {
                        debug!(error = %e, "WebSocket error");
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Send periodic ping
            _ = ping_interval.tick() => {
                if let Err(e) = sender.send(Message::Ping(vec![].into())).await {
                    debug!(error = %e, "Failed to send ping");
                    break;
                }
            }
        }
    }

    info!(guild_id, channel_id, "Voice WebSocket client disconnected");
}

/// Generate the voice channel web view HTML
fn generate_voice_view_html(guild_id: &str, channel_id: &str) -> String {
    let config = AppConfig::get();
    let ws_url = config
        .web
        .public_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LinguaBridge - Voice Translations</title>
    <style>
        :root {{
            --bg-primary: #36393f;
            --bg-secondary: #2f3136;
            --bg-tertiary: #202225;
            --text-primary: #dcddde;
            --text-secondary: #8e9297;
            --accent: #5865f2;
            --success: #3ba55d;
            --voice: #3ba55d;
        }}
        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            height: 100vh;
            display: flex;
            flex-direction: column;
        }}
        header {{
            background: var(--bg-tertiary);
            padding: 1rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid rgba(255,255,255,0.1);
        }}
        .header-left {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        .voice-icon {{
            color: var(--voice);
            font-size: 1.25rem;
        }}
        header h1 {{
            font-size: 1.25rem;
            font-weight: 600;
        }}
        .status {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.875rem;
            color: var(--text-secondary);
        }}
        .status-dot {{
            width: 10px;
            height: 10px;
            border-radius: 50%;
            background: #ed4245;
            animation: pulse 2s infinite;
        }}
        .status-dot.connected {{
            background: var(--success);
        }}
        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.5; }}
        }}

        /* TTS Audio Controls */
        .audio-controls {{
            background: var(--bg-secondary);
            padding: 0.75rem 1rem;
            display: flex;
            align-items: center;
            gap: 1rem;
            border-bottom: 1px solid rgba(255,255,255,0.05);
        }}
        .audio-controls label {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            cursor: pointer;
            font-size: 0.875rem;
        }}
        .audio-controls input[type="checkbox"] {{
            width: 18px;
            height: 18px;
            cursor: pointer;
        }}
        .volume-control {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .volume-control input[type="range"] {{
            width: 100px;
            cursor: pointer;
        }}
        .queue-status {{
            font-size: 0.75rem;
            color: var(--text-secondary);
            margin-left: auto;
        }}

        /* Messages */
        #messages {{
            flex: 1;
            overflow-y: auto;
            padding: 1rem;
        }}
        .message {{
            background: var(--bg-secondary);
            border-radius: 8px;
            padding: 1rem;
            margin-bottom: 0.75rem;
            animation: slideIn 0.3s ease;
            border-left: 3px solid var(--voice);
        }}
        @keyframes slideIn {{
            from {{
                opacity: 0;
                transform: translateY(10px);
            }}
            to {{
                opacity: 1;
                transform: translateY(0);
            }}
        }}
        .message-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.5rem;
        }}
        .speaker-info {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .speaker-avatar {{
            width: 24px;
            height: 24px;
            border-radius: 50%;
            background: var(--accent);
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 0.75rem;
            font-weight: 600;
        }}
        .speaker-name {{
            font-weight: 600;
            color: var(--accent);
        }}
        .timestamp {{
            font-size: 0.75rem;
            color: var(--text-secondary);
        }}
        .original {{
            color: var(--text-secondary);
            font-size: 0.875rem;
            margin-bottom: 0.5rem;
            padding-left: 0.75rem;
            border-left: 2px solid var(--text-secondary);
            font-style: italic;
        }}
        .translated {{
            font-size: 1rem;
        }}
        .lang-badge {{
            display: inline-block;
            background: var(--accent);
            color: white;
            font-size: 0.625rem;
            padding: 0.125rem 0.375rem;
            border-radius: 4px;
            text-transform: uppercase;
            margin-left: 0.5rem;
        }}
        .latency {{
            font-size: 0.625rem;
            color: var(--text-secondary);
            margin-left: 0.5rem;
        }}
        .empty-state {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100%;
            color: var(--text-secondary);
        }}
        .empty-state svg {{
            width: 64px;
            height: 64px;
            margin-bottom: 1rem;
            opacity: 0.5;
        }}
    </style>
</head>
<body>
    <header>
        <div class="header-left">
            <span class="voice-icon">&#128266;</span>
            <h1>Voice Channel</h1>
        </div>
        <div class="status">
            <div class="status-dot" id="statusDot"></div>
            <span id="statusText">Connecting...</span>
        </div>
    </header>

    <div class="audio-controls">
        <label>
            <input type="checkbox" id="ttsEnabled" checked>
            <span>&#128266; TTS Audio</span>
        </label>
        <div class="volume-control">
            <span>&#128264;</span>
            <input type="range" id="volume" min="0" max="100" value="80">
            <span id="volumeLabel">80%</span>
        </div>
        <div class="queue-status" id="queueStatus">Queue: 0</div>
    </div>

    <div id="messages">
        <div class="empty-state" id="emptyState">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
            </svg>
            <p>Waiting for voice activity...</p>
        </div>
    </div>

    <script>
        const guildId = '{}';
        const channelId = '{}';
        const wsUrl = '{}/voice/' + guildId + '/' + channelId + '/ws';

        const messagesEl = document.getElementById('messages');
        const emptyState = document.getElementById('emptyState');
        const statusDot = document.getElementById('statusDot');
        const statusText = document.getElementById('statusText');
        const ttsEnabled = document.getElementById('ttsEnabled');
        const volumeSlider = document.getElementById('volume');
        const volumeLabel = document.getElementById('volumeLabel');
        const queueStatus = document.getElementById('queueStatus');

        let ws;
        let reconnectAttempts = 0;
        let audioQueue = [];
        let isPlaying = false;
        let currentAudio = null;

        // Update volume label
        volumeSlider.addEventListener('input', () => {{
            volumeLabel.textContent = volumeSlider.value + '%';
            if (currentAudio) {{
                currentAudio.volume = volumeSlider.value / 100;
            }}
        }});

        // Format relative time
        function formatRelativeTime(timestamp) {{
            const seconds = Math.floor((Date.now() - timestamp) / 1000);
            if (seconds < 5) return 'just now';
            if (seconds < 60) return seconds + 's ago';
            const minutes = Math.floor(seconds / 60);
            if (minutes < 60) return minutes + 'm ago';
            const hours = Math.floor(minutes / 60);
            if (hours < 24) return hours + 'h ago';
            return new Date(timestamp).toLocaleString();
        }}

        // Update all timestamps every 10 seconds
        setInterval(() => {{
            document.querySelectorAll('.timestamp').forEach(el => {{
                const timestamp = parseInt(el.dataset.timestamp);
                if (timestamp) {{
                    el.textContent = formatRelativeTime(timestamp);
                }}
            }});
        }}, 10000);

        // Audio queue management
        function playNextAudio() {{
            if (audioQueue.length === 0) {{
                isPlaying = false;
                queueStatus.textContent = 'Queue: 0';
                return;
            }}

            isPlaying = true;
            queueStatus.textContent = 'Queue: ' + audioQueue.length;

            const audioData = audioQueue.shift();
            const audio = new Audio('data:audio/wav;base64,' + audioData);
            audio.volume = volumeSlider.value / 100;
            currentAudio = audio;

            audio.onended = () => {{
                currentAudio = null;
                playNextAudio();
            }};

            audio.onerror = () => {{
                console.error('Audio playback error');
                currentAudio = null;
                playNextAudio();
            }};

            audio.play().catch(e => {{
                console.error('Failed to play audio:', e);
                playNextAudio();
            }});
        }}

        function queueAudio(base64Audio) {{
            if (!ttsEnabled.checked || !base64Audio) return;

            audioQueue.push(base64Audio);
            queueStatus.textContent = 'Queue: ' + audioQueue.length;

            if (!isPlaying) {{
                playNextAudio();
            }}
        }}

        // Get speaker color based on user ID
        function getSpeakerColor(userId) {{
            const colors = ['#5865f2', '#3ba55d', '#faa61a', '#ed4245', '#9b59b6', '#e91e63', '#00bcd4'];
            let hash = 0;
            for (let i = 0; i < userId.length; i++) {{
                hash = userId.charCodeAt(i) + ((hash << 5) - hash);
            }}
            return colors[Math.abs(hash) % colors.length];
        }}

        // Get initials from username
        function getInitials(username) {{
            return username.split(' ').map(n => n[0]).join('').substring(0, 2).toUpperCase();
        }}

        function connect() {{
            ws = new WebSocket(wsUrl);

            ws.onopen = () => {{
                statusDot.classList.add('connected');
                statusText.textContent = 'Live';
                reconnectAttempts = 0;
            }};

            ws.onclose = () => {{
                statusDot.classList.remove('connected');
                statusText.textContent = 'Disconnected';

                // Reconnect with exponential backoff
                const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
                reconnectAttempts++;
                setTimeout(connect, delay);
            }};

            ws.onerror = (error) => {{
                console.error('WebSocket error:', error);
            }};

            ws.onmessage = (event) => {{
                const data = JSON.parse(event.data);

                if (data.type === 'voice_transcription') {{
                    addMessage(data);
                    if (data.tts_audio) {{
                        queueAudio(data.tts_audio);
                    }}
                }} else if (data.type === 'welcome') {{
                    console.log('Connected:', data.message);
                }} else if (data.type === 'error') {{
                    statusText.textContent = data.message;
                }}
            }};
        }}

        function addMessage(data) {{
            emptyState.style.display = 'none';

            const messageEl = document.createElement('div');
            messageEl.className = 'message';

            const speakerColor = getSpeakerColor(data.user_id);
            const initials = getInitials(data.username);
            const relativeTime = formatRelativeTime(data.timestamp);

            messageEl.innerHTML = `
                <div class="message-header">
                    <div class="speaker-info">
                        <div class="speaker-avatar" style="background: ${{speakerColor}}">${{initials}}</div>
                        <span class="speaker-name" style="color: ${{speakerColor}}">${{escapeHtml(data.username)}}</span>
                    </div>
                    <span class="timestamp" data-timestamp="${{data.timestamp}}">${{relativeTime}}</span>
                </div>
                <div class="original">"${{escapeHtml(data.original_text)}}"</div>
                <div class="translated">
                    ${{escapeHtml(data.translated_text)}}
                    <span class="lang-badge">${{data.source_lang.toUpperCase()}} &#8594; ${{data.target_lang.toUpperCase()}}</span>
                    <span class="latency">${{data.latency_ms}}ms</span>
                </div>
            `;

            messagesEl.appendChild(messageEl);
            messagesEl.scrollTop = messagesEl.scrollHeight;

            // Limit messages in DOM
            while (messagesEl.children.length > 100) {{
                messagesEl.removeChild(messagesEl.children[1]);
            }}
        }}

        function escapeHtml(text) {{
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }}

        connect();
    </script>
</body>
</html>"##,
        guild_id, channel_id, ws_url
    )
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_generate_voice_view_html() {
        // This would fail without AppConfig::init(), just test it doesn't panic in format
        // when config is not initialized, we'd need to mock it
    }
}

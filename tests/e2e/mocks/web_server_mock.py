"""
Mock Web Server

Serves the voice webview HTML for e2e testing.
"""

from fastapi import FastAPI, WebSocket
from fastapi.responses import HTMLResponse
import uvicorn

mock_web_app = FastAPI(title="Mock Linguabridge Web Server")

# Simple voice webview HTML for testing
VOICE_WEBVIEW_HTML = """
<!DOCTYPE html>
<html>
<head>
    <title>LinguaBridge - Live Translations</title>
    <style>
        .status-dot {
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background-color: #ccc;
            display: inline-block;
        }
        .status-dot.connected {
            background-color: #4caf50;
        }
        #messages {
            min-height: 50px;
            border: 1px solid #eee;
        }
        .message {
            padding: 8px;
            margin: 4px 0;
            border: 1px solid #ddd;
        }
        .author { font-weight: bold; }
        .lang-badge { color: #666; font-size: 0.9em; }
        .timestamp { color: #999; font-size: 0.8em; }
    </style>
</head>
<body>
    <h1>LinguaBridge</h1>
    <div class="status-dot" id="statusDot"></div>
    <span id="statusText">Connecting...</span>
    <div id="emptyState">Waiting for messages...</div>
    <div id="messages"></div>

    <script>
        let ws = null;

        function connect() {
            const wsUrl = `ws://${window.location.host}/ws`;
            ws = new WebSocket(wsUrl);
            window.ws = ws;

            ws.onopen = () => {
                document.getElementById('statusText').textContent = 'Connected';
                document.getElementById('statusDot').classList.add('connected');
            };

            ws.onclose = () => {
                document.getElementById('statusText').textContent = 'Disconnected';
                document.getElementById('statusDot').classList.remove('connected');
                // Reconnect after 2 seconds
                setTimeout(connect, 2000);
            };

            ws.onmessage = (event) => {
                const data = JSON.parse(event.data);

                if (data.type === 'translation') {
                    const messages = document.getElementById('messages');
                    const emptyState = document.getElementById('emptyState');
                    emptyState.style.display = 'none';

                    const msg = document.createElement('div');
                    msg.className = 'message';
                    msg.innerHTML = `
                        <span class="author">${data.author_name}</span>
                        <span class="original">${data.original_text}</span>
                        <span class="translated">${data.translated_text}</span>
                        <span class="lang-badge">${data.source_lang} → ${data.target_lang}</span>
                        <span class="timestamp">${new Date(data.timestamp).toLocaleTimeString()}</span>
                    `;
                    messages.appendChild(msg);
                } else if (data.type === 'error') {
                    document.getElementById('statusText').textContent = data.message;
                }
            };
        }

        connect();
    </script>
</body>
</html>
"""


@mock_web_app.get("/voice/{guild_id}/{channel_id}", response_class=HTMLResponse)
async def voice_webview(guild_id: str, channel_id: str):
    """Serve the voice webview page."""
    return VOICE_WEBVIEW_HTML


@mock_web_app.get("/health")
async def health():
    """Health check endpoint."""
    return {"status": "ok"}


@mock_web_app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """WebSocket endpoint for real-time updates."""
    await websocket.accept()
    try:
        while True:
            data = await websocket.receive_text()
            # Echo back for testing
            await websocket.send_text(data)
    except:
        pass


async def run_mock_web_server(host: str = "localhost", port: int = 9999):
    """Run the mock web server asynchronously."""
    config = uvicorn.Config(mock_web_app, host=host, port=port, log_level="warning")
    server = uvicorn.Server(config)
    await server.serve()


def run_mock_web_server_sync(host: str = "localhost", port: int = 9999):
    """Run the mock web server (blocking)."""
    uvicorn.run(mock_web_app, host=host, port=port, log_level="warning")

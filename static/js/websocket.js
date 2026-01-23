/**
 * Shared WebSocket connection with exponential backoff reconnect.
 *
 * Usage:
 *   const ws = createWebSocket(url, {
 *       onMessage: (data) => { ... },
 *       onStatusChange: (connected) => { ... }
 *   });
 */
function createWebSocket(url, { onMessage, onStatusChange }) {
    let ws;
    let reconnectAttempts = 0;

    function connect() {
        ws = new WebSocket(url);

        ws.onopen = () => {
            onStatusChange(true);
            reconnectAttempts = 0;
        };

        ws.onclose = () => {
            onStatusChange(false);
            const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
            reconnectAttempts++;
            setTimeout(connect, delay);
        };

        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };

        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            onMessage(data);
        };
    }

    connect();
    return { getSocket: () => ws };
}

/**
 * Escape HTML to prevent XSS.
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

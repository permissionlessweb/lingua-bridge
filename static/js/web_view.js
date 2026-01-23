(function () {
    const config = window.__CONFIG;
    const wsUrl = config.wsUrl + '/ws/' + config.sessionId;

    const messagesEl = document.getElementById('messages');
    const emptyState = document.getElementById('emptyState');
    const statusDot = document.getElementById('statusDot');
    const statusText = document.getElementById('statusText');

    function onStatusChange(connected) {
        if (connected) {
            statusDot.classList.add('connected');
            statusText.textContent = 'Connected';
        } else {
            statusDot.classList.remove('connected');
            statusText.textContent = 'Disconnected';
        }
    }

    function onMessage(data) {
        if (data.type === 'translation') {
            addMessage(data);
        } else if (data.type === 'error') {
            statusText.textContent = data.message;
        }
    }

    function addMessage(data) {
        emptyState.style.display = 'none';

        const messageEl = document.createElement('div');
        messageEl.className = 'message';

        const time = new Date(data.timestamp).toLocaleTimeString();

        messageEl.innerHTML = `
            <div class="message-header">
                <span class="author">${escapeHtml(data.author_name)}</span>
                <span class="timestamp">${time}</span>
            </div>
            <div class="original">${escapeHtml(data.original_text)}</div>
            <div class="translated">
                ${escapeHtml(data.translated_text)}
                <span class="lang-badge">${data.source_lang} &rarr; ${data.target_lang}</span>
            </div>
        `;

        messagesEl.appendChild(messageEl);
        messagesEl.scrollTop = messagesEl.scrollHeight;

        // Limit messages in DOM
        while (messagesEl.children.length > 100) {
            messagesEl.removeChild(messagesEl.children[1]);
        }
    }

    createWebSocket(wsUrl, { onMessage, onStatusChange });
})();

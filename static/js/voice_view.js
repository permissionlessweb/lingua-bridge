(function () {
    const config = window.__CONFIG;
    const wsUrl = config.wsUrl + '/voice/' + config.guildId + '/' + config.channelId + '/ws';

    const messagesEl = document.getElementById('messages');
    const emptyState = document.getElementById('emptyState');
    const statusDot = document.getElementById('statusDot');
    const statusText = document.getElementById('statusText');
    const ttsEnabled = document.getElementById('ttsEnabled');
    const volumeSlider = document.getElementById('volume');
    const volumeLabel = document.getElementById('volumeLabel');
    const queueStatus = document.getElementById('queueStatus');

    let audioQueue = [];
    let isPlaying = false;
    let currentAudio = null;

    // Volume control
    volumeSlider.addEventListener('input', () => {
        volumeLabel.textContent = volumeSlider.value + '%';
        if (currentAudio) {
            currentAudio.volume = volumeSlider.value / 100;
        }
    });

    // Relative time formatting
    function formatRelativeTime(timestamp) {
        const seconds = Math.floor((Date.now() - timestamp) / 1000);
        if (seconds < 5) return 'just now';
        if (seconds < 60) return seconds + 's ago';
        const minutes = Math.floor(seconds / 60);
        if (minutes < 60) return minutes + 'm ago';
        const hours = Math.floor(minutes / 60);
        if (hours < 24) return hours + 'h ago';
        return new Date(timestamp).toLocaleString();
    }

    // Update timestamps every 10 seconds
    setInterval(() => {
        document.querySelectorAll('.timestamp').forEach(el => {
            const timestamp = parseInt(el.dataset.timestamp);
            if (timestamp) {
                el.textContent = formatRelativeTime(timestamp);
            }
        });
    }, 10000);

    // Audio queue management
    function playNextAudio() {
        if (audioQueue.length === 0) {
            isPlaying = false;
            queueStatus.textContent = 'Queue: 0';
            return;
        }

        isPlaying = true;
        queueStatus.textContent = 'Queue: ' + audioQueue.length;

        const audioData = audioQueue.shift();
        const audio = new Audio('data:audio/wav;base64,' + audioData);
        audio.volume = volumeSlider.value / 100;
        currentAudio = audio;

        audio.onended = () => {
            currentAudio = null;
            playNextAudio();
        };

        audio.onerror = () => {
            console.error('Audio playback error');
            currentAudio = null;
            playNextAudio();
        };

        audio.play().catch(e => {
            console.error('Failed to play audio:', e);
            playNextAudio();
        });
    }

    function queueAudio(base64Audio) {
        if (!ttsEnabled.checked || !base64Audio) return;

        audioQueue.push(base64Audio);
        queueStatus.textContent = 'Queue: ' + audioQueue.length;

        if (!isPlaying) {
            playNextAudio();
        }
    }

    // Speaker color based on user ID
    function getSpeakerColor(userId) {
        const colors = ['#5865f2', '#3ba55d', '#faa61a', '#ed4245', '#9b59b6', '#e91e63', '#00bcd4'];
        let hash = 0;
        for (let i = 0; i < userId.length; i++) {
            hash = userId.charCodeAt(i) + ((hash << 5) - hash);
        }
        return colors[Math.abs(hash) % colors.length];
    }

    function getInitials(username) {
        return username.split(' ').map(n => n[0]).join('').substring(0, 2).toUpperCase();
    }

    function onStatusChange(connected) {
        if (connected) {
            statusDot.classList.add('connected');
            statusText.textContent = 'Live';
        } else {
            statusDot.classList.remove('connected');
            statusText.textContent = 'Disconnected';
        }
    }

    function onMessage(data) {
        if (data.type === 'voice_transcription') {
            addMessage(data);
            if (data.tts_audio) {
                queueAudio(data.tts_audio);
            }
        } else if (data.type === 'welcome') {
            console.log('Connected:', data.message);
        } else if (data.type === 'error') {
            statusText.textContent = data.message;
        }
    }

    function addMessage(data) {
        emptyState.style.display = 'none';

        const messageEl = document.createElement('div');
        messageEl.className = 'message';

        const speakerColor = getSpeakerColor(data.user_id);
        const initials = getInitials(data.username);
        const relativeTime = formatRelativeTime(data.timestamp);

        messageEl.innerHTML = `
            <div class="message-header">
                <div class="speaker-info">
                    <div class="speaker-avatar" style="background: ${speakerColor}">${initials}</div>
                    <span class="speaker-name" style="color: ${speakerColor}">${escapeHtml(data.username)}</span>
                </div>
                <span class="timestamp" data-timestamp="${data.timestamp}">${relativeTime}</span>
            </div>
            <div class="original">"${escapeHtml(data.original_text)}"</div>
            <div class="translated">
                ${escapeHtml(data.translated_text)}
                <span class="lang-badge">${data.source_lang.toUpperCase()} &rarr; ${data.target_lang.toUpperCase()}</span>
                <span class="latency">${data.latency_ms}ms</span>
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

"""
Web Interface E2E Tests

Tests the browser-based web interface for real-time voice translation
display using Playwright.

Note: These tests use pytest-playwright's synchronous fixtures.
"""

import pytest
import json
import time
import httpx
from typing import Dict, Any

from playwright.sync_api import Page, expect


# Shorter timeout for tests (5 seconds)
TEST_TIMEOUT = 5000


class TestVoiceWebView:
    """Test voice channel web view functionality."""

    def test_webview_page_loads(
        self,
        page: Page,
        test_config: Dict[str, str]
    ):
        """Test that voice web view page loads correctly."""
        guild_id = "123456789"
        channel_id = "987654321"
        base_url = test_config['web_base_url']
        webview_url = f"{base_url}/voice/{guild_id}/{channel_id}"

        print(f"\n[DEBUG] Base URL: {base_url}")
        print(f"[DEBUG] Full URL: {webview_url}")

        # First check if server is reachable
        print(f"[DEBUG] Checking server health...")
        try:
            with httpx.Client() as client:
                health_resp = client.get(f"{base_url}/health", timeout=2.0)
                print(f"[DEBUG] Health check: {health_resp.status_code} - {health_resp.text}")
        except Exception as e:
            print(f"[DEBUG] Health check failed: {e}")
            pytest.fail(f"Web server not reachable at {base_url}: {e}")

        print(f"[DEBUG] Navigating to {webview_url}...")
        response = page.goto(webview_url, timeout=TEST_TIMEOUT, wait_until="domcontentloaded")

        print(f"[DEBUG] Response: status={response.status if response else 'None'}")
        assert response is not None, "No response from page.goto()"
        assert response.status == 200, f"Expected 200, got {response.status}"

        print(f"[DEBUG] Checking page title...")
        title = page.title()
        print(f"[DEBUG] Page title: '{title}'")
        expect(page).to_have_title("LinguaBridge - Live Translations", timeout=TEST_TIMEOUT)

        print(f"[DEBUG] Checking header...")
        header = page.locator("h1")
        print(f"[DEBUG] Header text: '{header.text_content()}'")
        expect(header).to_contain_text("LinguaBridge", timeout=TEST_TIMEOUT)

        print(f"[DEBUG] PASSED")

    def test_page_has_required_elements(
        self,
        page: Page,
        test_config: Dict[str, str]
    ):
        """Test that page contains all required UI elements."""
        guild_id = "123456789"
        channel_id = "987654321"
        webview_url = f"{test_config['web_base_url']}/voice/{guild_id}/{channel_id}"

        page.goto(webview_url, timeout=TEST_TIMEOUT)

        # Check status indicator exists
        status_dot = page.locator(".status-dot")
        expect(status_dot).to_be_visible(timeout=TEST_TIMEOUT)

        # Check messages container exists
        messages = page.locator("#messages")
        expect(messages).to_be_visible(timeout=TEST_TIMEOUT)

        # Check empty state message
        empty_state = page.locator("#emptyState")
        expect(empty_state).to_be_visible(timeout=TEST_TIMEOUT)
        expect(empty_state).to_contain_text("Waiting for messages", timeout=TEST_TIMEOUT)

    def test_websocket_connects(
        self,
        page: Page,
        test_config: Dict[str, str]
    ):
        """Test that WebSocket connection is established."""
        guild_id = "123456789"
        channel_id = "987654321"
        webview_url = f"{test_config['web_base_url']}/voice/{guild_id}/{channel_id}"

        page.goto(webview_url, timeout=TEST_TIMEOUT)

        # Wait for WebSocket to connect
        time.sleep(1)

        # Check via JavaScript if WebSocket exists and is open
        ws_state = page.evaluate("""
            () => {
                if (!window.ws) return 'no_ws';
                return window.ws.readyState === 1 ? 'open' : 'not_open';
            }
        """)

        assert ws_state == 'open', f"WebSocket not connected: {ws_state}"

    def test_status_shows_connected(
        self,
        page: Page,
        test_config: Dict[str, str]
    ):
        """Test that status shows connected after WebSocket connects."""
        guild_id = "123456789"
        channel_id = "987654321"
        webview_url = f"{test_config['web_base_url']}/voice/{guild_id}/{channel_id}"

        page.goto(webview_url, timeout=TEST_TIMEOUT)

        # Wait for connection status
        status_text = page.locator("#statusText")
        expect(status_text).to_have_text("Connected", timeout=TEST_TIMEOUT)

    def test_translation_message_renders(
        self,
        page: Page,
        test_config: Dict[str, str],
        web_testdata: Dict[str, Any]
    ):
        """Test that translation messages render correctly."""
        guild_id = "123456789"
        channel_id = "987654321"
        webview_url = f"{test_config['web_base_url']}/voice/{guild_id}/{channel_id}"

        page.goto(webview_url, timeout=TEST_TIMEOUT)
        time.sleep(0.5)

        # Get test message from centralized test data
        mock_translation = web_testdata["webview_messages"]["translation_message"].copy()
        mock_translation["timestamp"] = "2024-01-01T12:00:00Z"

        # Inject message via JavaScript
        page.evaluate(f"""
            if (window.ws && window.ws.onmessage) {{
                window.ws.onmessage({{data: JSON.stringify({json.dumps(mock_translation)})}});
            }}
        """)

        # Check message appears
        messages = page.locator("#messages .message")
        expect(messages).to_have_count(1, timeout=TEST_TIMEOUT)

        # Check content
        message = messages.first
        expect(message.locator(".author")).to_contain_text("TestUser", timeout=TEST_TIMEOUT)
        expect(message.locator(".original")).to_contain_text("Hello world", timeout=TEST_TIMEOUT)
        expect(message.locator(".translated")).to_contain_text("Hola mundo", timeout=TEST_TIMEOUT)

    def test_multiple_messages_render(
        self,
        page: Page,
        test_config: Dict[str, str]
    ):
        """Test that multiple messages render in order."""
        guild_id = "123456789"
        channel_id = "987654321"
        webview_url = f"{test_config['web_base_url']}/voice/{guild_id}/{channel_id}"

        page.goto(webview_url, timeout=TEST_TIMEOUT)
        time.sleep(0.5)

        messages_data = [
            {
                "type": "translation",
                "author_name": "User1",
                "original_text": "Good morning",
                "translated_text": "Buenos días",
                "source_lang": "en",
                "target_lang": "es",
                "timestamp": "2024-01-01T08:00:00Z"
            },
            {
                "type": "translation",
                "author_name": "User2",
                "original_text": "How are you?",
                "translated_text": "¿Cómo estás?",
                "source_lang": "en",
                "target_lang": "es",
                "timestamp": "2024-01-01T08:05:00Z"
            }
        ]

        for msg in messages_data:
            page.evaluate(f"""
                if (window.ws && window.ws.onmessage) {{
                    window.ws.onmessage({{data: JSON.stringify({json.dumps(msg)})}});
                }}
            """)
            time.sleep(0.1)

        messages = page.locator("#messages .message")
        expect(messages).to_have_count(2, timeout=TEST_TIMEOUT)

    def test_error_message_displays(
        self,
        page: Page,
        test_config: Dict[str, str],
        web_testdata: Dict[str, Any]
    ):
        """Test that error messages are displayed."""
        guild_id = "123456789"
        channel_id = "987654321"
        webview_url = f"{test_config['web_base_url']}/voice/{guild_id}/{channel_id}"

        page.goto(webview_url, timeout=TEST_TIMEOUT)
        time.sleep(0.5)

        error_data = web_testdata["webview_messages"]["error_message"]

        page.evaluate(f"""
            if (window.ws && window.ws.onmessage) {{
                window.ws.onmessage({{data: JSON.stringify({json.dumps(error_data)})}});
            }}
        """)

        status_text = page.locator("#statusText")
        expect(status_text).to_contain_text("unavailable", timeout=TEST_TIMEOUT)

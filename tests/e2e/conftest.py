"""
Linguabridge E2E Test Suite

This module provides shared fixtures and utilities for end-to-end testing
of the Linguabridge Discord bot and web interface.
"""

from __future__ import annotations

import pytest
import asyncio
import os
import json
import subprocess
from pathlib import Path
from typing import Dict, Any, AsyncGenerator, Optional, TYPE_CHECKING

import websockets
from playwright.async_api import async_playwright, Browser, Page

# Discord imports are optional (only needed for Discord tests)
try:
    import discord
    from discord.ext import commands
    DISCORD_AVAILABLE = True
except ImportError:
    DISCORD_AVAILABLE = False
    discord = None
    commands = None

if TYPE_CHECKING:
    from discord.ext.commands import Bot


# Test configuration
TEST_CONFIG = {
    "discord_token": os.getenv("TEST_DISCORD_TOKEN", ""),
    "discord_guild_id": os.getenv("TEST_DISCORD_GUILD_ID", ""),
    "web_base_url": os.getenv("TEST_WEB_BASE_URL", "http://localhost:9999"),
    "inference_text_url": os.getenv("TEST_INFERENCE_TEXT_URL", "http://localhost:8000"),
    "inference_voice_url": os.getenv("TEST_INFERENCE_VOICE_URL", "ws://localhost:8001"),
}

# Test data paths
TEST_DATA_DIR = Path(__file__).parent / "fixtures"
VOICE_SAMPLES_DIR = TEST_DATA_DIR / "voice_samples"


@pytest.fixture(scope="session")
def test_config() -> Dict[str, str]:
    """Test configuration fixture."""
    return TEST_CONFIG


@pytest.fixture(scope="session")
def testdata():
    """Centralized test data fixture."""
    from fixtures import TestData
    return TestData()


@pytest.fixture(scope="session")
def translation_scenarios(testdata) -> Dict[str, Any]:
    """Load translation test scenarios (text only, legacy format)."""
    return testdata.text_translations


@pytest.fixture(scope="session")
def web_testdata(testdata) -> Dict[str, Any]:
    """Web interface test data."""
    return testdata.web


@pytest.fixture(scope="session")
def discord_testdata(testdata) -> Dict[str, Any]:
    """Discord test data."""
    return testdata.discord


@pytest.fixture(scope="session")
async def discord_bot(test_config):
    """Discord bot fixture for testing."""
    if not DISCORD_AVAILABLE:
        pytest.skip("discord.py not installed")

    if not test_config.get("discord_token"):
        pytest.skip("TEST_DISCORD_TOKEN not set")

    bot = commands.Bot(command_prefix="!", intents=discord.Intents.all())

    @bot.event
    async def on_ready():
        print(f"Test bot logged in as {bot.user}")

    await bot.login(test_config["discord_token"])
    yield bot
    await bot.close()


# Note: pytest-playwright provides 'page' and 'browser' fixtures automatically.
# We don't need to define them here.


@pytest.fixture(scope="session")
def mock_text_inference():
    """Mock text inference service fixture."""
    from mocks.text_inference_mock import mock_inference
    return mock_inference


@pytest.fixture(scope="session")
def mock_voice_inference():
    """Mock voice inference service fixture."""
    from mocks.voice_inference_mock import mock_voice_inference
    return mock_voice_inference


@pytest.fixture
def mock_text_client():
    """FastAPI test client for mock text inference."""
    from fastapi.testclient import TestClient
    from mocks.text_inference_mock import mock_app
    return TestClient(mock_app)


@pytest.fixture
async def mock_voice_websocket():
    """WebSocket connection to mock voice service."""
    uri = "ws://localhost:8001"
    async with websockets.connect(uri) as ws:
        yield ws


@pytest.fixture
def sample_audio_hello():
    """Sample audio data for 'hello'."""
    from mocks.voice_inference_mock import create_mock_audio_data
    return create_mock_audio_data("hello world")


@pytest.fixture
def sample_audio_long():
    """Sample audio data for longer message."""
    from mocks.voice_inference_mock import create_mock_audio_data
    return create_mock_audio_data("this is a longer test message for voice translation")


@pytest.fixture(autouse=True)
def setup_test_environment():
    """Setup test environment before each test."""
    # Ensure test directories exist
    VOICE_SAMPLES_DIR.mkdir(parents=True, exist_ok=True)

    # Set test environment variables
    os.environ["LINGUABRIDGE_ENV"] = "test"


def load_voice_sample(filename: str) -> bytes:
    """Load a voice sample file."""
    path = VOICE_SAMPLES_DIR / filename
    with open(path, "rb") as f:
        return f.read()


async def wait_for_discord_message(bot: "Bot", channel_id: int, timeout: int = 30):
    """Wait for a message in a Discord channel from the bot."""
    def check(message):
        return message.channel.id == channel_id and message.author == bot.user

    try:
        message = await bot.wait_for("message", check=check, timeout=timeout)
        return message
    except asyncio.TimeoutError:
        raise TimeoutError(f"No message received in channel {channel_id} within {timeout}s")


def assert_translation_accuracy(actual: str, expected: str, exact_match: bool = True):
    """Assert translation accuracy."""
    if exact_match:
        assert actual == expected, f"Translation mismatch: got '{actual}', expected '{expected}'"
    else:
        # Implement fuzzy matching for semantic similarity
        pass
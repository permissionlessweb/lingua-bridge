"""
Voice Channel E2E Tests

Tests the Discord bot's voice channel functionality including
STT, translation, TTS, and thread transcripts.
"""

import pytest
import asyncio
import base64
import json
from typing import Dict, Any

import discord
from discord.ext import commands

from conftest import wait_for_discord_message
from mocks.voice_inference_mock import create_mock_audio_data


class TestVoiceChannel:
    """Test voice channel workflows."""

    @pytest.mark.asyncio
    async def test_bot_joins_voice_channel(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test that bot can join voice channel on command."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        text_channel = discord.utils.get(guild.text_channels, name="test-translations")
        voice_channel = discord.utils.get(guild.voice_channels, name="Test Voice")

        assert voice_channel is not None, "Test voice channel not found"

        # Send join command
        join_command = "!join"
        await text_channel.send(join_command)

        # Wait for bot to join voice channel
        await asyncio.sleep(2)

        # Check if bot is in voice channel
        voice_client = discord.utils.get(discord_bot.voice_clients, guild=guild)
        assert voice_client is not None, "Bot did not join voice channel"
        assert voice_client.channel == voice_channel, "Bot joined wrong voice channel"

    @pytest.mark.asyncio
    async def test_voice_transcription_and_translation(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str],
        mock_voice_websocket
    ):
        """Test STT transcription and translation in voice channel."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        text_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Ensure bot is in voice channel
        voice_client = discord.utils.get(discord_bot.voice_clients, guild=guild)
        if not voice_client:
            # Join voice channel first
            voice_channel = discord.utils.get(guild.voice_channels, name="Test Voice")
            await voice_channel.connect()

        # Simulate audio input (in real test, this would come from voice activity)
        test_audio = create_mock_audio_data("Hello world", duration_ms=2000)

        # Send audio to mock voice inference
        audio_b64 = base64.b64encode(test_audio).decode()
        message = {
            "type": "audio",
            "audio": audio_b64,
            "user_id": "123456789",
            "channel_id": str(text_channel.id)
        }

        await mock_voice_websocket.send(json.dumps(message))

        # Wait for response
        response = await mock_voice_websocket.recv()
        response_data = json.loads(response)

        # Assert transcription and translation
        assert response_data["type"] == "translation"
        assert "transcription" in response_data
        assert "translation" in response_data
        assert response_data["source_lang"] == "en"
        assert response_data["target_lang"] == "es"

    @pytest.mark.skip(reason="Async loop mismatch with subprocess mock server")
    @pytest.mark.asyncio
    async def test_tts_audio_generation(
        self,
        mock_voice_websocket
    ):
        """Test TTS audio generation."""
        # Send translation request that should trigger TTS
        message = {
            "type": "audio",
            "audio": base64.b64encode(b"test_audio_data").decode(),
            "user_id": "123456789",
            "channel_id": "987654321",
            "enable_tts": True
        }

        await mock_voice_websocket.send(json.dumps(message))

        response = await mock_voice_websocket.recv()
        response_data = json.loads(response)

        # Assert TTS audio is included
        assert "tts_audio" in response_data
        assert response_data["tts_audio"] is not None

        # Decode and verify it's audio data
        tts_audio = base64.b64decode(response_data["tts_audio"])
        assert len(tts_audio) > 0

    @pytest.mark.asyncio
    async def test_thread_transcript_creation(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test automatic creation of thread transcripts."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        text_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Send a voice command that should create transcript
        transcript_command = "!transcript"
        await text_channel.send(transcript_command)

        # Wait for thread creation
        await asyncio.sleep(3)

        # Check if transcript thread was created
        threads = text_channel.threads
        transcript_threads = [t for t in threads if "transcript" in t.name.lower()]

        assert len(transcript_threads) > 0, "No transcript thread created"

        # Check thread content (this would be populated by voice activity)
        latest_thread = max(transcript_threads, key=lambda t: t.created_at)
        # In real test, we'd check thread messages contain transcriptions

    @pytest.mark.asyncio
    async def test_voice_channel_leave_command(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test bot leaving voice channel on command."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        text_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Ensure bot is in voice channel
        voice_client = discord.utils.get(discord_bot.voice_clients, guild=guild)
        if not voice_client:
            voice_channel = discord.utils.get(guild.voice_channels, name="Test Voice")
            await voice_channel.connect()
            voice_client = discord.utils.get(discord_bot.voice_clients, guild=guild)

        assert voice_client is not None, "Bot should be in voice channel"

        # Send leave command
        leave_command = "!leave"
        await text_channel.send(leave_command)

        # Wait for bot to leave
        await asyncio.sleep(2)

        # Check bot left voice channel
        voice_client_after = discord.utils.get(discord_bot.voice_clients, guild=guild)
        assert voice_client_after is None, "Bot did not leave voice channel"

    @pytest.mark.asyncio
    async def test_multiple_users_voice_channel(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str],
        mock_voice_websocket
    ):
        """Test voice translation with multiple users in channel."""
        # This test would simulate multiple users speaking
        # In a real scenario, we'd have multiple mock audio streams

        test_audios = [
            create_mock_audio_data("Hello everyone", duration_ms=1500),
            create_mock_audio_data("How are you today", duration_ms=1800),
            create_mock_audio_data("Nice to meet you", duration_ms=1200)
        ]

        translations_received = []

        for i, audio in enumerate(test_audios):
            audio_b64 = base64.b64encode(audio).decode()
            message = {
                "type": "audio",
                "audio": audio_b64,
                "user_id": f"user_{i}",
                "channel_id": "test_channel"
            }

            await mock_voice_websocket.send(json.dumps(message))

            response = await mock_voice_websocket.recv()
            response_data = json.loads(response)
            translations_received.append(response_data)

        # Assert we got translations for all audio inputs
        assert len(translations_received) == len(test_audios)
        for response in translations_received:
            assert response["type"] == "translation"
            assert "translation" in response
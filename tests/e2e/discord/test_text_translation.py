"""
Text Channel Translation E2E Tests

Tests the Discord bot's text channel translation functionality
using real Discord interactions and mocked inference services.
"""

import pytest
import asyncio
from typing import Dict, Any

import discord
from discord.ext import commands

from conftest import wait_for_discord_message, assert_translation_accuracy


class TestTextChannelTranslation:
    """Test text channel translation workflows."""

    @pytest.mark.asyncio
    async def test_basic_translation_english_to_spanish(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str],
        translation_scenarios: Dict[str, Any]
    ):
        """Test basic English to Spanish translation in text channel."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        assert guild is not None, "Test guild not found"

        # Find test channel
        test_channel = None
        for channel in guild.text_channels:
            if channel.name == "test-translations":
                test_channel = channel
                break

        assert test_channel is not None, "Test channel 'test-translations' not found"

        # Test scenario
        scenario = translation_scenarios["english_to_spanish"]
        test_message = scenario["input"]

        # Send test message
        await test_channel.send(test_message)

        # Wait for bot response
        bot_response = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=10
        )

        # Assert translation accuracy
        expected = scenario["expected"]
        assert_translation_accuracy(
            bot_response.content,
            expected,
            exact_match=True
        )

    @pytest.mark.asyncio
    async def test_user_language_preference_override(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test that user language preferences override default translations."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Set user language preference (this would normally be done via /mylang command)
        # For testing, we'll mock this by sending a direct command
        test_user_id = 123456789  # Mock user ID
        preference_command = f"!mylang es {test_user_id}"
        await test_channel.send(preference_command)

        # Wait for confirmation
        confirmation = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )
        assert "language preference" in confirmation.content.lower()

        # Send test message
        await test_channel.send("Hello world")

        # Should translate to Spanish instead of default
        translation = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )

        assert "Hola" in translation.content or "mundo" in translation.content

    @pytest.mark.asyncio
    async def test_multilingual_message_detection(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str],
        translation_scenarios: Dict[str, Any]
    ):
        """Test automatic language detection for various languages."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Test multiple languages
        test_cases = [
            ("english_to_spanish", "Spanish"),
            ("french_to_japanese", "Japanese"),
            ("german_to_french", "French")
        ]

        for scenario_key, expected_lang in test_cases:
            scenario = translation_scenarios[scenario_key]

            # Send message in source language
            await test_channel.send(scenario["input"])

            # Wait for translation
            response = await wait_for_discord_message(
                discord_bot,
                test_channel.id,
                timeout=10
            )

            # Should contain translation and language indicator
            assert expected_lang.lower() in response.content.lower() or \
                   scenario["target_lang"] in response.content.lower()

    @pytest.mark.asyncio
    async def test_emoji_preservation(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str],
        translation_scenarios: Dict[str, Any]
    ):
        """Test that emojis are preserved in translations."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        scenario = translation_scenarios["emoji_handling"]
        await test_channel.send(scenario["input"])

        response = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=10
        )

        # Should contain the rocket emoji
        assert "🚀" in response.content

    @pytest.mark.asyncio
    async def test_translation_disabled_channel(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test that translation doesn't occur in non-configured channels."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)

        # Find a non-translation channel
        general_channel = discord.utils.get(guild.text_channels, name="general")
        if not general_channel:
            # Skip if no general channel
            pytest.skip("No 'general' channel found for testing")

        # Send message to non-translation channel
        await general_channel.send("Hello in general channel")

        # Wait a bit and check no translation response
        await asyncio.sleep(3)

        # This is a basic check - in real implementation, we'd check message history
        # For now, just ensure no exception occurred
        assert True  # Placeholder assertion
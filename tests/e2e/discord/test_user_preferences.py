"""
User Preferences E2E Tests

Tests user language preference functionality including setting,
persistence, and override behavior.
"""

import pytest
import asyncio
from typing import Dict, Any

import discord
from discord.ext import commands

from conftest import wait_for_discord_message


class TestUserPreferences:
    """Test user language preference functionality."""

    @pytest.mark.asyncio
    async def test_set_user_language_preference(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test setting user language preference via command."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Test setting Spanish as preferred language
        mylang_command = "!mylang es"
        await test_channel.send(mylang_command)

        # Wait for confirmation
        confirmation = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )

        assert "spanish" in confirmation.content.lower() or \
               "es" in confirmation.content.lower() or \
               "preference" in confirmation.content.lower()

    @pytest.mark.asyncio
    async def test_user_preference_persistence(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test that user preferences persist across messages."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Set preference to French
        await test_channel.send("!mylang fr")

        # Wait for confirmation
        await wait_for_discord_message(discord_bot, test_channel.id, timeout=5)

        # Send multiple test messages
        test_messages = [
            "Hello world",
            "Good morning",
            "Thank you"
        ]

        for message in test_messages:
            await test_channel.send(message)

            # Wait for translation
            translation = await wait_for_discord_message(
                discord_bot,
                test_channel.id,
                timeout=5
            )

            # Should be translated to French (not default language)
            # Note: This is a simplified check - real implementation would verify
            # the actual translation content
            assert translation.content != message  # Should be translated

    @pytest.mark.asyncio
    async def test_preference_override_behavior(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str],
        translation_scenarios: Dict[str, Any]
    ):
        """Test that user preferences override default channel translation."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Set user preference to German
        await test_channel.send("!mylang de")
        await wait_for_discord_message(discord_bot, test_channel.id, timeout=5)

        # Send English message
        english_message = "How are you?"
        await test_channel.send(english_message)

        # Should translate to German (user preference) instead of default
        translation = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )

        # Check that it's not the default translation (assuming default is Spanish)
        spanish_scenario = translation_scenarios["english_to_spanish"]
        assert translation.content != spanish_scenario["expected"]

    @pytest.mark.asyncio
    async def test_clear_user_preference(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test clearing user language preference."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # First set a preference
        await test_channel.send("!mylang ja")
        await wait_for_discord_message(discord_bot, test_channel.id, timeout=5)

        # Clear preference
        clear_command = "!mylang clear"
        await test_channel.send(clear_command)

        # Wait for confirmation
        confirmation = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )

        assert "clear" in confirmation.content.lower() or \
               "reset" in confirmation.content.lower() or \
               "remove" in confirmation.content.lower()

    @pytest.mark.asyncio
    async def test_invalid_language_code(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test handling of invalid language codes."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Try to set invalid language
        invalid_command = "!mylang xyz"
        await test_channel.send(invalid_command)

        # Should get error message
        error_response = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )

        assert "invalid" in error_response.content.lower() or \
               "not supported" in error_response.content.lower() or \
               "error" in error_response.content.lower()

    @pytest.mark.asyncio
    async def test_list_available_languages(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test listing available languages."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Request language list
        list_command = "!languages"
        await test_channel.send(list_command)

        # Should get list of supported languages
        response = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )

        # Check for some expected languages
        content_lower = response.content.lower()
        assert any(lang in content_lower for lang in ["english", "spanish", "french", "german"])

    @pytest.mark.asyncio
    async def test_preference_case_insensitive(
        self,
        discord_bot: commands.Bot,
        test_config: Dict[str, str]
    ):
        """Test that language codes are case insensitive."""
        guild_id = int(test_config["discord_guild_id"])
        guild = discord_bot.get_guild(guild_id)
        test_channel = discord.utils.get(guild.text_channels, name="test-translations")

        # Try uppercase language code
        await test_channel.send("!mylang ES")  # Uppercase Spanish

        confirmation = await wait_for_discord_message(
            discord_bot,
            test_channel.id,
            timeout=5
        )

        assert "spanish" in confirmation.content.lower() or \
               "es" in confirmation.content.lower()
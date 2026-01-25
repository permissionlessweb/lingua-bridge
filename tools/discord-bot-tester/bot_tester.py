#!/usr/bin/env python3
"""
Discord Bot Configuration & Testing Automation
Configures a Discord bot to communicate with a backend endpoint and runs automated tests.
"""

import os
import sys
import asyncio
import logging
from typing import Optional
from dotenv import load_dotenv
import discord
from discord import app_commands
import aiohttp

# ============================================================================
# Configuration
# ============================================================================

load_dotenv()

BOT_TOKEN = os.getenv("BOT_TOKEN")
GUILD_ID = int(os.getenv("GUILD_ID", 0))
BACKEND_URL = os.getenv("BACKEND_URL")

if not BOT_TOKEN or not GUILD_ID or not BACKEND_URL:
    print("ERROR: Missing required env vars (BOT_TOKEN, GUILD_ID, BACKEND_URL)")
    print("Create a .env file with:")
    print("  BOT_TOKEN=your_bot_token_here")
    print("  GUILD_ID=your_guild_id_here")
    print("  BACKEND_URL=https://your-backend.example.com:9999")
    sys.exit(1)

# ============================================================================
# Logging
# ============================================================================

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler()]
)
log = logging.getLogger(__name__)

# ============================================================================
# Bot Client
# ============================================================================

class BotTester(discord.Client):
    def __init__(self):
        intents = discord.Intents.default()
        intents.guilds = True
        intents.messages = True
        intents.voice_states = True
        super().__init__(intents=intents)
        self.tree = app_commands.CommandTree(self)
        self.guild = discord.Object(id=GUILD_ID)
        self.test_results = {
            "bot_online": False,
            "guild_joined": False,
            "backend_health": False,
            "backend_status": False,
            "commands_registered": False,
            "ping_test": False,
            "translate_test": False,
        }

    async def setup_hook(self):
        """Called before bot connects. Register commands here."""
        log.info("Setting up bot commands...")

        # Register slash commands (guild-scoped for instant sync)
        @self.tree.command(name="ping", description="Test bot responsiveness", guild=self.guild)
        async def ping(interaction: discord.Interaction):
            await interaction.response.send_message("Pong! Bot is alive.")
            self.test_results["ping_test"] = True
            log.info("✅ Ping test passed")

        @self.tree.command(name="translate", description="Test backend translation", guild=self.guild)
        async def translate(interaction: discord.Interaction, text: str, target: str = "en"):
            await interaction.response.defer()  # Translation may take time
            try:
                async with aiohttp.ClientSession() as session:
                    # Try the inference endpoint directly
                    async with session.post(
                        f"{BACKEND_URL.replace(':9999', ':8000')}/translate",
                        json={"text": text, "target_language": target},
                        timeout=30
                    ) as resp:
                        if resp.status == 200:
                            data = await resp.json()
                            translated = data.get('translated_text', 'N/A')
                            await interaction.followup.send(f"✅ Translated: {translated}")
                            self.test_results["translate_test"] = True
                            log.info(f"✅ Translation test passed: '{text}' -> '{translated}'")
                        else:
                            error_text = await resp.text()
                            await interaction.followup.send(f"❌ Backend error: {resp.status}\n{error_text[:200]}")
                            log.error(f"❌ Backend returned {resp.status}: {error_text[:200]}")
            except asyncio.TimeoutError:
                await interaction.followup.send("❌ Backend timeout (>30s)")
                log.error("❌ Translation test failed: timeout")
            except Exception as e:
                await interaction.followup.send(f"❌ Error: {e}")
                log.error(f"❌ Translation test failed: {e}")

        # Sync commands to guild
        try:
            await self.tree.sync(guild=self.guild)
            self.test_results["commands_registered"] = True
            log.info("✅ Commands registered and synced")
        except Exception as e:
            log.error(f"❌ Failed to sync commands: {e}")

    async def on_ready(self):
        """Called when bot successfully connects to Discord."""
        log.info(f"Bot connected as {self.user} (ID: {self.user.id})")
        self.test_results["bot_online"] = True

        # Check guild membership
        guild = self.get_guild(GUILD_ID)
        if guild:
            log.info(f"✅ Bot is in guild: {guild.name} (ID: {guild.id})")
            self.test_results["guild_joined"] = True
        else:
            log.error(f"❌ Bot not in guild {GUILD_ID}. Invite it first!")
            log.error("Generate invite URL at: https://discord.com/developers/applications")
            log.error("Required scopes: bot, applications.commands")
            log.error("Required permissions: Send Messages, Use Slash Commands, Connect, Speak")
            await self.close()
            return

        # Check backend health
        await self.check_backend_health()

        # Check backend status (provisioning)
        await self.check_backend_status()

        # Print instructions for manual tests
        self.print_manual_test_instructions()

        # Print summary
        self.print_test_summary()

        # Keep bot running for manual tests (Ctrl+C to exit)
        log.info("\n🤖 Bot is running. Run slash commands in Discord to test.")
        log.info("Press Ctrl+C to stop.\n")

    async def check_backend_health(self):
        """Verify backend is reachable."""
        try:
            async with aiohttp.ClientSession() as session:
                # Try the web endpoint health check
                health_url = f"{BACKEND_URL.replace(':9999', ':80')}/health"
                log.info(f"Checking health endpoint: {health_url}")
                async with session.get(health_url, timeout=10) as resp:
                    if resp.status == 200:
                        log.info(f"✅ Backend health check passed ({health_url})")
                        self.test_results["backend_health"] = True
                    else:
                        log.error(f"❌ Backend health check failed: {resp.status}")
        except asyncio.TimeoutError:
            log.error("❌ Backend health check timeout")
        except Exception as e:
            log.error(f"❌ Backend unreachable: {e}")

    async def check_backend_status(self):
        """Verify backend provisioning status."""
        try:
            async with aiohttp.ClientSession() as session:
                # Check admin status endpoint
                status_url = f"{BACKEND_URL}/status"
                log.info(f"Checking status endpoint: {status_url}")
                async with session.get(status_url, timeout=10) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        provisioned = data.get('provisioned', False)
                        if provisioned:
                            log.info(f"✅ Bot is provisioned with Discord token")
                            self.test_results["backend_status"] = True
                        else:
                            log.warning(f"⚠️  Bot not provisioned yet. Run: cargo run -p admin-cli -- provision")
                    else:
                        log.error(f"❌ Status check failed: {resp.status}")
        except asyncio.TimeoutError:
            log.error("❌ Status check timeout")
        except Exception as e:
            log.error(f"❌ Status check failed: {e}")

    def print_manual_test_instructions(self):
        """Print instructions for manual testing."""
        guild = self.get_guild(GUILD_ID)
        if not guild:
            return

        log.info("\n" + "="*60)
        log.info("MANUAL TEST INSTRUCTIONS")
        log.info("="*60)
        log.info(f"1. Open Discord and go to guild: {guild.name}")
        log.info("2. In any text channel, type: /ping")
        log.info("   Expected: Bot responds with 'Pong! Bot is alive.'")
        log.info("3. Test translation: /translate text:hello target:es")
        log.info("   Expected: Bot responds with Spanish translation")
        log.info("="*60 + "\n")

    def print_test_summary(self):
        """Print test results."""
        log.info("\n" + "="*60)
        log.info("AUTOMATED TEST SUMMARY")
        log.info("="*60)
        for test, passed in self.test_results.items():
            status = "✅ PASS" if passed else "❌ FAIL"
            log.info(f"{test.replace('_', ' ').title()}: {status}")
        log.info("="*60)

        automated_tests = ["bot_online", "guild_joined", "backend_health", "backend_status", "commands_registered"]
        automated_passed = all(self.test_results[t] for t in automated_tests)

        if automated_passed:
            log.info("🎉 All automated tests passed! Bot is configured correctly.")
            log.info("   Run manual tests above to verify full functionality.")
        else:
            log.error("⚠️  Some automated tests failed. Check logs above.")

# ============================================================================
# Main Entry Point
# ============================================================================

async def main():
    client = BotTester()
    async with client:
        await client.start(BOT_TOKEN)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        log.info("\n\n🛑 Bot stopped by user")
        sys.exit(0)
    except Exception as e:
        log.error(f"Fatal error: {e}")
        sys.exit(1)

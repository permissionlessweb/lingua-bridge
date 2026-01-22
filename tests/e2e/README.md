# Linguabridge E2E Tests

End-to-end tests for the Linguabridge Discord translation bot and web interface.

## Quick Start

```bash
# Install dependencies and run all tests
cd tests/e2e
make install
make test
```

## Setup

### 1. Install Dependencies

```bash
cd tests/e2e
pip install -r requirements.txt
playwright install chromium
```

Or use the Makefile:

```bash
make install
```

### 2. Configure Discord (for Discord tests)

Create a test Discord server and bot:
- Create a test Discord server
- Create a test bot and add it to the server
- Create channels: `#test-translations` (text), `Test Voice` (voice)
- Set bot permissions for voice and text

Set environment variables:

```bash
export TEST_DISCORD_TOKEN="your_bot_token"
export TEST_DISCORD_GUILD_ID="your_guild_id"
```

### 3. Configure Web Interface (for web tests)

```bash
export TEST_WEB_BASE_URL="http://localhost:9999"
```

## Running Tests

### Single Command (Recommended)

```bash
# Run all tests with automatic mock service management
python run_tests.py

# Or using make
make test
```

### Test Type Selection

```bash
# Web interface tests only
python run_tests.py --test-type web
# or: make test-web

# Discord workflow tests only
python run_tests.py --test-type discord
# or: make test-discord
```

### Advanced Options

```bash
# Run specific tests by pattern
python run_tests.py -k "test_basic"
# or: make test-filter FILTER="test_basic"

# Stop on first failure
python run_tests.py -x
# or: make test-fail-fast

# Collect tests without running (verify discovery)
python run_tests.py --collect-only
# or: make test-collect

# Run without starting mock services (they're already running)
python run_tests.py --no-mocks
# or: make test-external

# Provide Discord credentials via CLI
python run_tests.py --discord-token "token" --guild-id "id"
```

### Run All Available Targets

```bash
make help
```

## Test Structure

```
tests/e2e/
├── conftest.py              # Shared fixtures and configuration
├── pytest.ini               # Pytest configuration
├── run_tests.py             # Test runner with mock orchestration
├── Makefile                 # Make targets for common operations
├── requirements.txt         # Python dependencies
├── discord/                 # Discord bot workflow tests
│   ├── test_text_translation.py
│   ├── test_voice_channel.py
│   └── test_user_preferences.py
├── web/                     # Browser interface tests
│   └── test_voice_webview.py
├── mocks/                   # Mock inference services
│   ├── text_inference_mock.py   # FastAPI mock (port 8000)
│   └── voice_inference_mock.py  # WebSocket mock (port 8001)
└── fixtures/                # Test data
    └── translation_scenarios.json
```

## Mock Services

The test runner automatically starts mock inference services:

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| Text Inference | 8000 | HTTP | Translation & language detection |
| Voice Inference | 8001 | WebSocket | STT, translation, TTS |

### Manual Mock Service Control

For development, you can start mocks manually:

```bash
# Text inference mock (blocks terminal)
make mock-text

# Voice inference mock (blocks terminal)
make mock-voice
```

Then run tests with `--no-mocks`:

```bash
python run_tests.py --no-mocks
```

## Writing Tests

### Discord Tests

Use the `discord_bot` fixture for bot interactions:

```python
@pytest.mark.asyncio
async def test_my_feature(discord_bot, test_config):
    guild = discord_bot.get_guild(int(test_config["discord_guild_id"]))
    channel = discord.utils.get(guild.text_channels, name="test-channel")
    await channel.send("test message")

    response = await wait_for_discord_message(discord_bot, channel.id, timeout=10)
    assert "expected" in response.content
```

### Web Tests

Use Playwright fixtures for browser testing:

```python
@pytest.mark.asyncio
async def test_web_feature(page, test_config):
    await page.goto(f"{test_config['web_base_url']}/some-path")
    await expect(page.locator(".element")).to_be_visible()
```

### Mock Integration

Add test scenarios to `fixtures/translation_scenarios.json`:

```json
{
  "my_scenario": {
    "input": "Hello",
    "source_lang": "en",
    "target_lang": "es",
    "expected": "Hola"
  }
}
```

## CI/CD Integration

Tests can run in CI pipelines:

```yaml
# GitHub Actions example
- name: Run E2E Tests
  env:
    TEST_DISCORD_TOKEN: ${{ secrets.TEST_DISCORD_TOKEN }}
    TEST_DISCORD_GUILD_ID: ${{ secrets.TEST_DISCORD_GUILD_ID }}
  run: |
    cd tests/e2e
    pip install -r requirements.txt
    playwright install chromium
    python run_tests.py
```

## Troubleshooting

### Port Already in Use

If mock ports (8000, 8001) are already in use, the test runner will skip starting those mocks and assume external services are running.

### Playwright Browser Issues

```bash
# Reinstall browsers
playwright install chromium --with-deps
```

### Import Errors

Ensure you're running from the `tests/e2e` directory:

```bash
cd tests/e2e
python run_tests.py
```

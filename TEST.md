# Running Tests

## Quick Start

Run all tests (Rust + Python e2e) with:

```bash
just test
```

Or run each suite separately:

```bash
# Rust unit/property tests
just test-rust

# Python e2e tests
just test-python

# E2e tests directly (auto-creates venv, installs deps, starts mocks)
./tests/e2e/test.sh
```

## Test Suites

| Suite | Tests | Framework | Description |
|-------|-------|-----------|-------------|
| Rust unit | ~70 | cargo test | Unit tests for core modules |
| Rust property | ~13 | proptest | Property-based tests |
| Python e2e (web) | 7 | pytest + Playwright | Browser-based web interface tests |
| Python e2e (Discord) | 18 | pytest + discord.py | Discord bot integration tests |

## Code Coverage

### Run with coverage

```bash
# Both Rust and Python coverage
just test-coverage

# Rust only
just coverage-rust

# Python only
just coverage-python
# or
./tests/e2e/test.sh --coverage
```

### Coverage reports

| Language | Report Location | Threshold |
|----------|----------------|-----------|
| Rust | `target/coverage/tarpaulin-report.html` | - |
| Rust (LCOV) | `target/coverage/lcov.info` | - |
| Python | `tests/e2e/coverage_html/index.html` | 70% |

### Install coverage tools

```bash
just install-test-deps
```

This installs `cargo-tarpaulin` and Python test dependencies.

## Justfile Commands

| Command | Description |
|---------|-------------|
| `just test` | Run all tests (Rust + Python) |
| `just test-rust` | Run Rust tests |
| `just test-rust-verbose` | Run Rust tests with output |
| `just test-python` | Run Python e2e tests |
| `just test-coverage` | Run all tests with coverage |
| `just coverage-rust` | Rust coverage only |
| `just coverage-python` | Python coverage only |
| `just check` | Quick compile + clippy check |
| `just install-test-deps` | Install all test tooling |

## E2E Test Options

```bash
# Run only web tests (no Discord setup needed)
./tests/e2e/test.sh --test-type web

# Run only Discord tests
./tests/e2e/test.sh --test-type discord

# Run tests matching a pattern
./tests/e2e/test.sh -k "test_translation"

# Stop on first failure
./tests/e2e/test.sh -x

# Fresh environment (delete and recreate venv)
./tests/e2e/test.sh --clean

# Run with Python coverage
./tests/e2e/test.sh --coverage
```

## Discord Test Setup

Discord tests require a real Discord bot and test server. Without configuration, these tests are skipped.

### Step 1: Create a Test Discord Bot

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application" and name it (e.g., "LinguaBridge Test Bot")
3. Go to "Bot" section and click "Add Bot"
4. Under "Privileged Gateway Intents", enable:
   - Presence Intent
   - Server Members Intent
   - Message Content Intent
5. Click "Reset Token" and copy the token

### Step 2: Create a Test Discord Server

1. Create a new Discord server for testing
2. Create these channels:
   - Text channel: `test-translations`
   - Voice channel: `Test Voice`
3. Copy the Server ID (right-click server name → Copy Server ID)
   - Enable Developer Mode in Discord settings if you don't see this option

### Step 3: Invite the Bot to Your Test Server

1. In Developer Portal, go to "OAuth2" → "URL Generator"
2. Select scopes: `bot`, `applications.commands`
3. Select permissions: `Send Messages`, `Read Message History`, `Connect`, `Speak`
4. Copy the generated URL and open it to invite the bot

### Step 4: Set Environment Variables

```bash
export TEST_DISCORD_TOKEN="your_bot_token_here"
export TEST_DISCORD_GUILD_ID="your_server_id_here"
```

Or pass them directly:

```bash
./tests/e2e/test.sh --discord-token "token" --guild-id "id"
```

### Step 5: Run Discord Tests

```bash
./tests/e2e/test.sh --test-type discord
```

## Rust Test Structure

Unit and property tests live alongside source code in `#[cfg(test)]` modules:

| Module | Tests | Type |
|--------|-------|------|
| `src/translation/language.rs` | 17 | Unit + Property |
| `src/translation/cache.rs` | 14 | Unit + Property |
| `src/voice/buffer.rs` | 16 | Unit + Property |
| `src/error.rs` | 16 | Unit |
| `src/db/queries.rs` | 24 | Unit (in-memory SQLite) |
| `src/db/models.rs` | 16 | Unit |
| `src/admin/transport.rs` | 7 | Unit |
| `src/admin/crypto.rs` | 2 | Unit |
| `src/web/routes.rs` | 7 | Unit |

Property tests use the `proptest` crate to generate randomized inputs and verify invariants hold across many cases.

## Test Data

All test data is centralized in `tests/e2e/fixtures/testdata.json`:

- Translation scenarios (text and voice)
- Language detection samples
- Discord command responses
- Web interface messages

## Troubleshooting

### Tests hanging or timing out

```bash
# Use fresh environment
./tests/e2e/test.sh --clean
```

### Playwright browser issues

```bash
cd tests/e2e
source .venv/bin/activate
playwright install chromium --with-deps
```

### Port conflicts

The test runner auto-detects if ports are in use:

- 8000: Text inference mock
- 8001: Voice inference mock
- 9999: Web server mock

If a port is in use, the runner assumes an external service is running.

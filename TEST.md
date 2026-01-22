# Running Tests

## E2E Tests

Run the full end-to-end test suite with a single command:

```bash
./tests/e2e/test.sh
```

This automatically:
- Creates an isolated Python virtual environment
- Installs all dependencies
- Starts mock services (text inference, voice inference, web server)
- Runs all tests
- Cleans up when done

### Options

```bash
# Run only web interface tests
./tests/e2e/test.sh --test-type web

# Run only Discord workflow tests
./tests/e2e/test.sh --test-type discord

# Run tests matching a pattern
./tests/e2e/test.sh -k "test_translation"

# Stop on first failure
./tests/e2e/test.sh -x

# Fresh environment (delete and recreate venv)
./tests/e2e/test.sh --clean
```

### Discord Tests

Discord tests require credentials. Set these environment variables before running:

```bash
export TEST_DISCORD_TOKEN="your_bot_token"
export TEST_DISCORD_GUILD_ID="your_guild_id"
./tests/e2e/test.sh --test-type discord
```

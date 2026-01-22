# Linguabridge E2E Tests Makefile
# Run with: make [target]

.PHONY: all install test test-web test-discord test-collect clean help

PYTHON ?= python3
PYTEST ?= $(PYTHON) -m pytest

# Default target
all: test

# Install dependencies
install:
	$(PYTHON) -m pip install -r requirements.txt
	playwright install chromium

# Run all e2e tests
test:
	$(PYTHON) run_tests.py

# Run only web interface tests
test-web:
	$(PYTHON) run_tests.py --test-type web

# Run only Discord workflow tests
test-discord:
	$(PYTHON) run_tests.py --test-type discord

# Run tests without starting mock services
test-external:
	$(PYTHON) run_tests.py --no-mocks

# Collect tests without running (verify test discovery)
test-collect:
	$(PYTHON) run_tests.py --collect-only

# Run tests with verbose output
test-verbose:
	$(PYTHON) run_tests.py -v

# Run tests, stop on first failure
test-fail-fast:
	$(PYTHON) run_tests.py -x

# Run a specific test by pattern
# Usage: make test-filter FILTER="test_basic"
test-filter:
	$(PYTHON) run_tests.py -k "$(FILTER)"

# Start mock services manually (for development)
mock-text:
	$(PYTHON) -c "from mocks.text_inference_mock import run_mock_text_server_sync; run_mock_text_server_sync()"

mock-voice:
	$(PYTHON) -c "import asyncio; from mocks.voice_inference_mock import mock_voice_server; asyncio.run(mock_voice_server())"

# Clean up Python cache files
clean:
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	find . -type d -name ".pytest_cache" -exec rm -rf {} + 2>/dev/null || true

# Show help
help:
	@echo "Linguabridge E2E Test Suite"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  install       Install Python dependencies and Playwright browsers"
	@echo "  test          Run all e2e tests (default)"
	@echo "  test-web      Run only web interface tests"
	@echo "  test-discord  Run only Discord workflow tests"
	@echo "  test-external Run tests without starting mock services"
	@echo "  test-collect  Collect tests without running them"
	@echo "  test-verbose  Run tests with verbose output"
	@echo "  test-fail-fast Stop on first test failure"
	@echo "  test-filter   Run tests matching pattern (FILTER='pattern')"
	@echo "  mock-text     Start text inference mock server manually"
	@echo "  mock-voice    Start voice inference mock server manually"
	@echo "  clean         Remove Python cache files"
	@echo "  help          Show this help message"
	@echo ""
	@echo "Environment variables:"
	@echo "  TEST_DISCORD_TOKEN   Discord bot token for testing"
	@echo "  TEST_DISCORD_GUILD_ID Discord server ID for testing"
	@echo "  TEST_WEB_BASE_URL    Web interface URL (default: http://localhost:9999)"

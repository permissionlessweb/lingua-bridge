#!/usr/bin/env bash
#
# Linguabridge E2E Test Runner
#
# Usage: ./test.sh [options]
#
# Options are passed directly to run_tests.py:
#   --test-type web|discord|all   Run specific test type
#   -k "pattern"                  Run tests matching pattern
#   -x                            Stop on first failure
#   --no-mocks                    Don't start mock services
#   --collect-only                Just list tests, don't run
#
# Examples:
#   ./test.sh                     Run all tests
#   ./test.sh --test-type web     Run web tests only
#   ./test.sh -k "test_basic"     Run tests matching pattern
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_DIR="$SCRIPT_DIR/.venv"
PYTHON="${PYTHON:-python3}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() { echo -e "${GREEN}[e2e]${NC} $1"; }
warn() { echo -e "${YELLOW}[e2e]${NC} $1"; }
error() { echo -e "${RED}[e2e]${NC} $1"; }

# Create virtual environment if it doesn't exist
setup_venv() {
    if [ ! -d "$VENV_DIR" ]; then
        log "Creating virtual environment..."
        $PYTHON -m venv "$VENV_DIR"
    fi

    # Activate venv
    source "$VENV_DIR/bin/activate"

    # Upgrade pip quietly
    pip install --upgrade pip -q
}

# Install dependencies
install_deps() {
    log "Installing dependencies..."
    pip install -r "$SCRIPT_DIR/requirements.txt" -q

    # Install playwright browsers if needed
    if ! playwright install chromium --dry-run &>/dev/null 2>&1; then
        log "Installing Playwright browsers..."
        playwright install chromium
    fi
}

# Check if deps are already installed
deps_installed() {
    python -c "import pytest, fastapi, websockets, playwright, pytest_cov" 2>/dev/null
}

# Handle --clean and --coverage flags
CLEAN=0
COVERAGE=0
ARGS=()
for arg in "$@"; do
    if [ "$arg" = "--clean" ]; then
        CLEAN=1
    elif [ "$arg" = "--coverage" ]; then
        COVERAGE=1
    else
        ARGS+=("$arg")
    fi
done

if [ $CLEAN -eq 1 ] && [ -d "$VENV_DIR" ]; then
    warn "Removing existing virtual environment..."
    rm -rf "$VENV_DIR"
fi

# Main
main() {
    cd "$SCRIPT_DIR"

    setup_venv

    if ! deps_installed; then
        install_deps
    else
        log "Dependencies already installed"
    fi

    log "Running e2e tests..."
    echo ""

    # Add coverage args if requested
    if [ $COVERAGE -eq 1 ]; then
        log "Coverage reporting enabled"
        ARGS+=(--coverage)
    fi

    # Run tests, passing arguments through (excluding --clean/--coverage)
    python run_tests.py "${ARGS[@]}"
    exit_code=$?

    if [ $COVERAGE -eq 1 ] && [ $exit_code -eq 0 ]; then
        echo ""
        log "Coverage report: tests/e2e/coverage_html/index.html"
    fi

    if [ $exit_code -eq 0 ]; then
        echo ""
        log "All tests passed!"
    else
        echo ""
        error "Some tests failed (exit code: $exit_code)"
    fi

    exit $exit_code
}

main "$@"

#!/usr/bin/env python3
"""
Linguabridge E2E Test Runner

Orchestrates e2e tests with automatic mock service management.
Run with: python run_tests.py [options]
"""

import os
import sys
import subprocess
import argparse
import signal
import time
import socket
from pathlib import Path
from multiprocessing import Process
from contextlib import contextmanager
from typing import Optional, List


# Constants
TEST_DIR = Path(__file__).parent
TEXT_MOCK_PORT = 8000
VOICE_MOCK_PORT = 8001
WEB_MOCK_PORT = 9999


def is_port_in_use(port: int) -> bool:
    """Check if a port is already in use."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(("localhost", port)) == 0


def wait_for_port(port: int, timeout: int = 10) -> bool:
    """Wait for a port to become available."""
    start = time.time()
    while time.time() - start < timeout:
        if is_port_in_use(port):
            return True
        time.sleep(0.1)
    return False


def start_text_mock_server() -> None:
    """Entry point for text mock server process."""
    # Import inside function to avoid issues
    sys.path.insert(0, str(TEST_DIR))
    from mocks.text_inference_mock import run_mock_text_server_sync
    run_mock_text_server_sync(port=TEXT_MOCK_PORT)


def start_voice_mock_server() -> None:
    """Entry point for voice mock server process."""
    import asyncio
    sys.path.insert(0, str(TEST_DIR))
    from mocks.voice_inference_mock import mock_voice_server
    asyncio.run(mock_voice_server(port=VOICE_MOCK_PORT))


def start_web_mock_server() -> None:
    """Entry point for web mock server process."""
    sys.path.insert(0, str(TEST_DIR))
    from mocks.web_server_mock import run_mock_web_server_sync
    run_mock_web_server_sync(port=WEB_MOCK_PORT)


@contextmanager
def mock_servers():
    """Context manager to start/stop mock services."""
    processes: List[Process] = []

    # Check if ports are already in use
    text_port_used = is_port_in_use(TEXT_MOCK_PORT)
    voice_port_used = is_port_in_use(VOICE_MOCK_PORT)
    web_port_used = is_port_in_use(WEB_MOCK_PORT)

    if text_port_used:
        print(f"  Text mock port {TEXT_MOCK_PORT} already in use (external service assumed)")
    else:
        print(f"  Starting text mock server on port {TEXT_MOCK_PORT}...")
        p = Process(target=start_text_mock_server, daemon=True)
        p.start()
        processes.append(p)

    if voice_port_used:
        print(f"  Voice mock port {VOICE_MOCK_PORT} already in use (external service assumed)")
    else:
        print(f"  Starting voice mock server on port {VOICE_MOCK_PORT}...")
        p = Process(target=start_voice_mock_server, daemon=True)
        p.start()
        processes.append(p)

    if web_port_used:
        print(f"  Web mock port {WEB_MOCK_PORT} already in use (external service assumed)")
    else:
        print(f"  Starting web mock server on port {WEB_MOCK_PORT}...")
        p = Process(target=start_web_mock_server, daemon=True)
        p.start()
        processes.append(p)

    # Wait for servers to be ready
    if not text_port_used and not wait_for_port(TEXT_MOCK_PORT):
        print(f"  Warning: Text mock server didn't start on port {TEXT_MOCK_PORT}")
    if not voice_port_used and not wait_for_port(VOICE_MOCK_PORT):
        print(f"  Warning: Voice mock server didn't start on port {VOICE_MOCK_PORT}")
    if not web_port_used and not wait_for_port(WEB_MOCK_PORT):
        print(f"  Warning: Web mock server didn't start on port {WEB_MOCK_PORT}")

    try:
        yield processes
    finally:
        # Terminate all started processes
        for p in processes:
            if p.is_alive():
                p.terminate()
                p.join(timeout=2)
                if p.is_alive():
                    p.kill()


def ensure_playwright_browsers():
    """Install Playwright browsers if not already installed."""
    try:
        result = subprocess.run(
            ["playwright", "install", "chromium"],
            capture_output=True,
            text=True,
            timeout=300
        )
        if result.returncode != 0:
            print(f"Warning: Playwright install returned: {result.stderr}")
    except FileNotFoundError:
        print("Warning: playwright CLI not found. Install with: pip install playwright")
    except subprocess.TimeoutExpired:
        print("Warning: Playwright browser installation timed out")


def run_pytest(
    test_type: str = "all",
    markers: Optional[List[str]] = None,
    extra_args: Optional[List[str]] = None,
    verbose: bool = True
) -> int:
    """Run pytest with specified configuration."""
    # Use python -m pytest for reliability
    pytest_args = [sys.executable, "-m", "pytest"]

    if verbose:
        pytest_args.append("-v")

    pytest_args.extend(["--tb=short"])

    # Add markers filter
    if markers:
        pytest_args.extend(["-m", " or ".join(markers)])

    # Determine test path
    if test_type == "discord":
        pytest_args.append(str(TEST_DIR / "discord"))
    elif test_type == "web":
        pytest_args.append(str(TEST_DIR / "web"))
    elif test_type == "mocks":
        # Run only mock unit tests (if any)
        pytest_args.append(str(TEST_DIR / "mocks"))
    else:
        pytest_args.append(str(TEST_DIR))

    # Add any extra arguments
    if extra_args:
        pytest_args.extend(extra_args)

    print(f"\nRunning: {' '.join(pytest_args)}\n")
    result = subprocess.run(pytest_args, cwd=str(TEST_DIR))
    return result.returncode


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Linguabridge E2E Test Runner",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python run_tests.py                     # Run all e2e tests
  python run_tests.py --test-type web     # Run only web tests
  python run_tests.py --test-type discord # Run only Discord tests
  python run_tests.py --no-mocks          # Run tests without starting mocks
  python run_tests.py -k "test_basic"     # Run tests matching pattern
        """
    )
    parser.add_argument(
        "--test-type", "-t",
        choices=["all", "discord", "web", "mocks"],
        default="all",
        help="Type of tests to run (default: all)"
    )
    parser.add_argument(
        "--no-mocks",
        action="store_true",
        help="Don't start mock services (assume they're running)"
    )
    parser.add_argument(
        "--discord-token",
        help="Discord bot token for testing (or set TEST_DISCORD_TOKEN env var)"
    )
    parser.add_argument(
        "--guild-id",
        help="Discord guild ID for testing (or set TEST_DISCORD_GUILD_ID env var)"
    )
    parser.add_argument(
        "--web-url",
        default="http://localhost:9999",
        help="Base URL for web interface (default: http://localhost:9999)"
    )
    parser.add_argument(
        "--no-browser-install",
        action="store_true",
        help="Skip Playwright browser installation check"
    )
    parser.add_argument(
        "-k",
        dest="filter_expr",
        help="Pytest filter expression (e.g., 'test_basic')"
    )
    parser.add_argument(
        "-x", "--exitfirst",
        action="store_true",
        help="Stop on first failure"
    )
    parser.add_argument(
        "--collect-only",
        action="store_true",
        help="Only collect tests, don't run them"
    )
    parser.add_argument(
        "-q", "--quiet",
        action="store_true",
        help="Decrease verbosity"
    )

    args = parser.parse_args()

    # Set environment variables
    if args.discord_token:
        os.environ["TEST_DISCORD_TOKEN"] = args.discord_token
    if args.guild_id:
        os.environ["TEST_DISCORD_GUILD_ID"] = args.guild_id
    os.environ["TEST_WEB_BASE_URL"] = args.web_url
    os.environ["TEST_INFERENCE_TEXT_URL"] = f"http://localhost:{TEXT_MOCK_PORT}"
    os.environ["TEST_INFERENCE_VOICE_URL"] = f"ws://localhost:{VOICE_MOCK_PORT}"

    # Install Playwright browsers if needed
    if not args.no_browser_install and args.test_type in ["all", "web"]:
        print("Checking Playwright browsers...")
        ensure_playwright_browsers()

    # Build extra pytest args
    extra_args = []
    if args.filter_expr:
        extra_args.extend(["-k", args.filter_expr])
    if args.exitfirst:
        extra_args.append("-x")
    if args.collect_only:
        extra_args.append("--collect-only")

    # Run tests
    if args.no_mocks:
        print("\nRunning tests (mock services should be running externally)...\n")
        return run_pytest(
            test_type=args.test_type,
            extra_args=extra_args,
            verbose=not args.quiet
        )
    else:
        print("\nStarting mock services...")
        with mock_servers():
            print("Mock services ready.\n")
            return run_pytest(
                test_type=args.test_type,
                extra_args=extra_args,
                verbose=not args.quiet
            )


if __name__ == "__main__":
    sys.exit(main())

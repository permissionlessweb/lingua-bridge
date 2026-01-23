# Linguabridge test and coverage commands

# Default recipe
default: test

# Run all tests (Rust + Python e2e)
test: test-rust test-python

# Run Rust tests
test-rust:
    cargo test --workspace

# Run Rust tests with verbose output
test-rust-verbose:
    cargo test --workspace -- --nocapture

# Run Python e2e tests
test-python:
    ./tests/e2e/test.sh --test-type web

# Run all tests with coverage reports
test-coverage: coverage-rust coverage-python
    @echo ""
    @echo "=== Coverage Reports ==="
    open target/coverage/tarpaulin-report.html
    open tests/e2e/coverage_html/index.html

# Rust coverage (requires cargo-tarpaulin)
coverage-rust:
    cargo tarpaulin --lib --skip-clean --out Html --out Lcov --output-dir target/coverage --ignore-tests --exclude-files "admin-cli/*" "src/bot/*"
    open target/coverage/tarpaulin-report.html

# Python coverage
coverage-python:
    ./tests/e2e/test.sh --test-type web --coverage
    open tests/e2e/coverage_html/index.html

# Install tarpaulin if not present
install-tarpaulin:
    cargo install cargo-tarpaulin

# Install all test dependencies
install-test-deps:
    cargo install cargo-tarpaulin
    cd tests/e2e && pip install -r requirements.txt

# Quick check (fast feedback)
check:
    cargo chec
    cargo clippy --workspace

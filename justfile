# antlers development tasks
# https://github.com/casey/just

set shell := ["bash", "-uc"]

# Default recipe: show available commands
default:
    @just --list

# =============================================================================
# Build & Test
# =============================================================================

# Build all crates in debug mode
build:
    cargo build --workspace

# Build in release mode
build-release:
    cargo build --workspace --release

# Run all tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets

# Run clippy and fix auto-fixable issues
lint-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Format code
fmt:
    cargo fmt --all

# Check formatting without changes
fmt-check:
    cargo fmt --all -- --check

# Run all checks (format, lint, test)
check: fmt-check lint test

# Build documentation
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Open documentation in browser
doc-open:
    cargo doc --workspace --no-deps --open

# =============================================================================
# Installation
# =============================================================================

# Install from local source (debug)
install:
    cargo install --path crates/antlers-cli

# Install from local source (release)
install-release:
    cargo install --path crates/antlers-cli --release

# Install or upgrade nightly from Homebrew tap
install-nightly-brew:
    brew tap albertocavalcante/tap
    brew install antlers-nightly || brew upgrade antlers-nightly

# Upgrade nightly from Homebrew
upgrade-nightly-brew:
    brew upgrade antlers-nightly || echo "Already up to date or not installed"

# Reinstall nightly from Homebrew (force update)
reinstall-nightly-brew:
    brew reinstall antlers-nightly

# Uninstall nightly from Homebrew
uninstall-nightly-brew:
    brew uninstall antlers-nightly

# Install or upgrade stable from Homebrew tap (when available)
install-brew:
    brew tap albertocavalcante/tap
    brew install antlers || brew upgrade antlers

# =============================================================================
# Development Helpers
# =============================================================================

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and run tests
watch:
    cargo watch -x test

# Watch for changes and run clippy
watch-lint:
    cargo watch -x 'clippy --workspace --all-targets'

# Run a quick sanity check (fast compile check)
quick-check:
    cargo check --workspace --all-targets

# =============================================================================
# CI Simulation
# =============================================================================

# Run pre-commit checks locally
pre-commit:
    lefthook run pre-commit

# Run full CI pipeline locally
ci: fmt-check lint test doc
    @echo "All CI checks passed!"

# =============================================================================
# Release
# =============================================================================

# Trigger nightly build on GitHub
trigger-nightly:
    gh workflow run nightly.yml -f force=true

# Show nightly build status
nightly-status:
    gh run list --workflow=nightly.yml --limit 5

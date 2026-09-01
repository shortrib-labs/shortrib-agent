set export

# Show available tasks
default:
    @just --list

# First-time project setup: configure git signing and activate hooks
init: hooks
    @echo "Checking GPG signing configuration..."
    @if ! git config --global user.signingkey > /dev/null 2>&1; then \
        echo "⚠️  No GPG signing key configured."; \
        echo "   Run: git config --global user.signingkey <KEY_ID>"; \
        echo "   Run: git config --global commit.gpgsign true"; \
    else \
        git config --global commit.gpgsign true; \
        echo "✓ GPG signing enabled (key: $$(git config --global user.signingkey))"; \
    fi
    @echo "✓ Project initialized"

# Activate git hooks from .githooks/
hooks:
    git config core.hooksPath .githooks
    @echo "✓ Git hooks activated from .githooks/"

# Deactivate custom git hooks (restore default .git/hooks path)
unhooks:
    git config --unset core.hooksPath
    @echo "✓ Git hooks path restored to default"

# Run validation (format, lint, test) — mirrors CI
check: lint test

# Format the code
fmt:
    cargo fmt --all

# Run formatter check and clippy
lint:
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
test:
    cargo test --all-features

# Build the project
build:
    cargo build --release --locked

# Verify the public Calendar MCP OAuth metadata without credentials
verify-calendar-metadata:
    scripts/verify-google-calendar-mcp.sh

# Run the project locally
dev:
    cargo run

# Remove build artifacts
clean:
    cargo clean

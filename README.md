# Bitwarden CLI (Rust)

A secure and free password manager for all of your devices.

This is the Rust implementation of the Bitwarden CLI, providing improved performance and reduced binary size compared to the TypeScript version.

## Prerequisites

- Rust 1.85.0 or later (Edition 2024)
- Bitwarden SDK (cloned at `../sdk-internal/`)

## Building

```bash
# Build debug binary
cargo build

# Build optimized release binary
cargo build --release

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features

# Format code
cargo fmt
```

## Installation

```bash
# Install from local build
cargo install --path crates/bw-cli

# Or copy binary directly
cp target/release/bw /usr/local/bin/
```

## Usage

```bash
# Show help
bw --help

# Show version
bw --version

# Login with email/password
bw login

# Unlock vault (after login)
bw unlock

# Sync vault from server
bw sync

# List items
bw list items

# Get specific item
bw get item <id>

# Generate TOTP code
bw get totp <id>

# Check status
bw status --response
```

## Global Flags

- `--session <KEY>` - Session authentication key (env: BW_SESSION)
- `--quiet` - Suppress all output (env: BW_QUIET)
- `--response` - Return JSON formatted response (env: BW_RESPONSE)
- `--raw` - Return raw output (env: BW_RAW)
- `--pretty` - Format JSON with indentation (env: BW_PRETTY)
- `--nointeraction` - Disable interactive prompts (env: BW_NOINTERACTION)
- `--cleanexit` - Exit with code 0 even on errors (env: BW_CLEANEXIT)

## Development Status

This project is in active development. Currently implemented:
- ✅ Project structure and build configuration
- ✅ CLI parsing with all commands
- ✅ Global flags and environment variables
- ✅ Response formatting system
- ✅ SDK integration (real Bitwarden SDK crypto)
- ✅ Storage layer (TypeScript CLI compatible)
- ✅ API client (server communication)
- ✅ Authentication (login, unlock, lock, logout)
- ✅ Vault sync from server
- ✅ Vault read commands (list, get, TOTP)
- ✅ Password/passphrase generation
- ✅ Vault write commands (create, edit, delete, restore, move)
- ✅ Attachments (create, get, delete)
- 🚧 Send commands
- 🚧 Import/export


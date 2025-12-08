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

# Login (stub - not yet implemented)
bw login

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

This project is in early development. Currently implemented:
- ✅ Project structure and build configuration
- ✅ CLI parsing with all commands
- ✅ Global flags and environment variables
- ✅ Response formatting system
- ✅ SDK integration (real Bitwarden SDK crypto)
- ✅ Storage layer (encrypted local state)
- ✅ API client (server communication)
- ✅ Authentication infrastructure (login/unlock/logout)
- 🚧 Command implementations (in progress)

All commands currently return "Not yet implemented". See the [enhancement plan](enhancements/) for implementation roadmap.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

GPL-3.0 - see [LICENSE](LICENSE) for details.

# Cross-CLI Compatibility Guide

This guide explains how the Rust Bitwarden CLI achieves compatibility with the official TypeScript Bitwarden CLI, enabling seamless switching between implementations.

## Overview

The Rust CLI uses the same storage format as the TypeScript CLI, allowing both to share:

- Authentication tokens (access and refresh)
- Account registry
- Device identification
- Encryption keys

This means you can login with one CLI and use the session with the other.

## Prerequisites

- TypeScript CLI data.json at state version 73+
- Both CLIs accessing the same storage location

### Storage Locations

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/Bitwarden CLI/data.json` |
| Windows | `%AppData%\Bitwarden CLI\data.json` |
| Linux | `~/.config/Bitwarden CLI/data.json` |

## Checking Your State Version

To verify your data.json is compatible:

```bash
# macOS/Linux
cat ~/Library/Application\ Support/Bitwarden\ CLI/data.json | grep stateVersion
# Should show: "stateVersion": 73

# Or use jq if installed
jq '.stateVersion' ~/Library/Application\ Support/Bitwarden\ CLI/data.json
```

If your state version is below 73, run `bw login` with the TypeScript CLI to upgrade.

## Usage Scenarios

### Scenario 1: Login with TypeScript CLI, Use Rust CLI

```bash
# Login with TypeScript CLI (creates session)
npx @bitwarden/cli login

# Export session (if needed)
export BW_SESSION="..."

# Rust CLI can now use the same session
./bw sync
./bw list items
./bw get item "GitHub"
```

### Scenario 2: Login with Rust CLI, Use TypeScript CLI

```bash
# Login with Rust CLI
./bw login

# Export session
export BW_SESSION="..."

# TypeScript CLI can now use the same session
npx @bitwarden/cli list items
npx @bitwarden/cli sync
```

### Scenario 3: Logout and Re-login

```bash
# Either CLI can logout
./bw logout
# or
npx @bitwarden/cli logout

# Either CLI can login again
./bw login
# or
npx @bitwarden/cli login
```

## Storage Format Details

The shared storage uses namespaced keys for multi-account support:

### Global Keys

| Key | Purpose |
|-----|---------|
| `stateVersion` | Format version (must be 73+) |
| `global_applicationId_appId` | Application instance UUID |
| `global_account_accounts` | Account registry |
| `global_account_activeAccountId` | Currently active user |

### User Keys (per account)

All user data is namespaced with the user's ID:

| Pattern | Purpose |
|---------|---------|
| `user_{id}_token_accessToken` | OAuth access token |
| `user_{id}_token_refreshToken` | OAuth refresh token |
| `user_{id}_crypto_privateKey` | Encrypted private key |
| `user_{id}_crypto_userKey` | Encrypted user key |

## Behavior Differences

### Logout

Both CLIs handle logout the same way:

1. Access token set to `null` (not removed)
2. Refresh token set to `null` (not removed)
3. Active account ID set to `null`
4. Account remains in registry

This allows quick re-login without re-entering the email address.

### Multi-Account Support

The storage format supports multiple accounts, but the CLI only works with one account at a time. The `global_account_activeAccountId` determines which account is active.

## Troubleshooting

### Error: "Unsupported state version"

```
Error: Unsupported state version 50. This CLI requires version 73+.
Run the TypeScript CLI to upgrade your data.
```

**Solution**: Run `bw login` with the TypeScript CLI to upgrade your data.json to the latest format.

### Error: "No active account"

```
Error: No active account. Please log in first.
```

**Solution**: Run `bw login` to create a session.

### Error: "Session expired or invalid"

```
Error: Session expired or invalid
```

**Solution**: Run `bw unlock` to unlock your vault, or `bw login` if you've been logged out.

### Data Not Syncing Between CLIs

1. Verify both CLIs use the same storage path
2. Check file permissions on data.json
3. Ensure no CLI process is locking the file
4. Try running `bw sync` to refresh data

### Token Issues After Switching CLIs

If tokens seem invalid after switching:

1. Run `bw logout` to clear the session
2. Run `bw login` to create a fresh session
3. Verify both CLIs are using the same API server

## Best Practices

1. **Don't modify data.json manually** - Let the CLIs manage the file
2. **Use the same BW_SESSION** - When switching between CLIs in the same terminal session
3. **Sync after switching** - Run `bw sync` after switching CLIs to ensure data consistency
4. **Keep CLIs updated** - Use recent versions for best compatibility

## Technical Notes

### Unknown Key Preservation

The Rust CLI preserves all keys it doesn't understand. This ensures forward compatibility when the TypeScript CLI adds new features.

### Device ID Migration

The Rust CLI supports migrating device IDs from the legacy `deviceId` key to the new `global_deviceId` format. It reads both formats but writes only the new format.

### Session Key (BW_SESSION)

The `BW_SESSION` environment variable contains the encryption key for your vault. Both CLIs use the same format, so a session key from one CLI works with the other.

## Limitations

1. **State migrations not implemented** - Rust CLI cannot upgrade old data.json formats
2. **Multi-account commands not exposed** - Infrastructure exists but CLI commands are limited
3. **Secure storage not integrated** - Tokens stored in data.json, not system keychain

## Getting Help

- File issues at: https://github.com/anthropics/claude-code/issues
- TypeScript CLI docs: https://bitwarden.com/help/cli/

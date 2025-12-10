# User Guide: Vault Encryption and Decryption

This guide explains how the Bitwarden Rust CLI handles vault encryption and decryption, including session key management and best practices for secure usage.

## Overview

The Bitwarden CLI uses end-to-end encryption to protect your vault data. When you log in or unlock your vault, the CLI generates a session key that is used to securely store your encryption keys locally. This session key must be provided for all vault operations.

## Understanding Session Keys

### What is a Session Key?

A session key is a randomly generated 64-byte encryption key that:
- Is created during login or unlock
- Protects your user encryption key while stored on disk
- Must be provided for any vault operation (list, get, etc.)
- Is unique to each login/unlock session

### Why Session Keys?

Session keys provide an additional layer of security:
1. Your master password is never stored
2. Your encryption key is encrypted at rest
3. Only the holder of the session key can decrypt vault data
4. Session keys can be invalidated by locking the vault

## Basic Workflow

### 1. Login

When you log in, the CLI:
1. Authenticates with the Bitwarden server
2. Retrieves your encrypted vault
3. Derives your master key from your password
4. Decrypts your user encryption key
5. Generates a new session key
6. Encrypts your user key with the session key
7. Stores the encrypted user key locally
8. Returns the session key for you to export

```bash
$ bw login user@example.com
? Master password: [hidden]

You are logged in!

To unlock your vault, use the `unlock` command. ex:
$ bw unlock

You can also export the session key to use with other commands:
$ export BW_SESSION="<your-session-key>"
```

### 2. Export Session Key

After login or unlock, you must export the session key:

```bash
# Using export (recommended for interactive sessions)
export BW_SESSION="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# Or add to your shell profile for persistence across terminals
echo 'export BW_SESSION="..."' >> ~/.bashrc
```

### 3. Use Vault Commands

Once the session is set, you can use vault commands:

```bash
# List all items
bw list items

# Get a specific item
bw get item "github"

# Get a password
bw get password "github"

# Get a username
bw get username "github"

# Generate TOTP code
bw get totp "github"
```

### 4. Lock Vault

When done, lock your vault:

```bash
bw lock
```

This clears the encrypted user key from local storage. You'll need to unlock again before using vault commands.

### 5. Unlock

To resume using the vault after locking:

```bash
$ bw unlock
? Master password: [hidden]

Your vault is now unlocked!

$ export BW_SESSION="<new-session-key>"
```

Note: Each unlock generates a new session key. Previous session keys are invalidated.

## Alternative: Using --session Flag

Instead of exporting `BW_SESSION`, you can pass the session key directly:

```bash
# Pass session key as argument
bw list items --session "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# Store in a variable for convenience
SESSION="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
bw list items --session "$SESSION"
```

This is useful for:
- Scripts that manage multiple sessions
- Automation where environment variables aren't practical
- Temporary commands without modifying environment

## Error Messages and Solutions

### "Session key required"

```
Error: Session key required. Export BW_SESSION or use --session flag.
Run 'bw unlock' to get a new session key.
```

**Solution:** Run `bw unlock` and export the session key.

### "No active user"

```
Error: No active user. Run 'bw login' first.
```

**Solution:** Run `bw login` to log in.

### "User key not found"

```
Error: User key not found. Run 'bw unlock' first.
```

**Solution:** The vault is locked. Run `bw unlock` to unlock it.

### "Invalid session key"

```
Error: Invalid session key: Failed to parse session key
```

**Solution:** The session key is malformed. Run `bw unlock` to get a new valid session key.

### "Failed to decrypt user key"

```
Error: Failed to decrypt user key: Decryption failed
```

**Solution:** The session key doesn't match the stored encrypted user key. This can happen if:
- You're using a session key from a different login session
- The stored data was corrupted
- Run `bw unlock` to get a fresh session key

## Security Best Practices

### Do's

1. **Clear session on logout:**
   ```bash
   bw lock
   unset BW_SESSION
   ```

2. **Use environment variables in interactive sessions:**
   ```bash
   export BW_SESSION="..."
   ```

3. **Use --session for one-off commands in scripts:**
   ```bash
   bw get password "github" --session "$SESSION"
   ```

4. **Lock vault when not in use:**
   ```bash
   bw lock
   ```

### Don'ts

1. **Don't share session keys** - They grant access to your vault data

2. **Don't log session keys** - Avoid printing them in logs or error messages

3. **Don't store session keys in files** - Use environment variables instead

4. **Don't hardcode session keys** - They change on each unlock

## Scripting Examples

### Basic Script

```bash
#!/bin/bash
# Script that retrieves a password

# Unlock vault (user will be prompted for password)
SESSION=$(bw unlock --raw)

# Get the password
PASSWORD=$(bw get password "my-service" --session "$SESSION")

# Use the password
echo "Password retrieved: ${PASSWORD:0:4}****"

# Lock vault when done
bw lock
```

### Script with Error Handling

```bash
#!/bin/bash
set -e

# Check if already unlocked
if [ -z "$BW_SESSION" ]; then
    echo "Vault is locked. Unlocking..."
    export BW_SESSION=$(bw unlock --raw)
fi

# Verify session is valid
if ! bw list items --session "$BW_SESSION" > /dev/null 2>&1; then
    echo "Session invalid. Re-unlocking..."
    export BW_SESSION=$(bw unlock --raw)
fi

# Get items
bw list items --session "$BW_SESSION"
```

### Using with jq

```bash
# Get all login items as JSON and parse with jq
bw list items --session "$BW_SESSION" | jq '.[] | select(.type == 1) | .name'

# Get specific field
bw get item "github" --session "$BW_SESSION" | jq -r '.login.username'
```

## Compatibility with TypeScript CLI

The Rust CLI is fully compatible with the official TypeScript Bitwarden CLI:

### Session Key Compatibility

Session keys generated by either CLI can be used with the other:

```bash
# Login with TypeScript CLI
npx @bitwarden/cli login
export BW_SESSION="..."

# Use session with Rust CLI
bw list items  # Works!
```

```bash
# Login with Rust CLI
./bw login
export BW_SESSION="..."

# Use session with TypeScript CLI
npx @bitwarden/cli list items  # Works!
```

### Storage Compatibility

Both CLIs use the same storage format:
- Same data directory location
- Same file format for vault data
- Same protected storage key format

This means you can switch between CLIs without re-syncing your vault.

## Troubleshooting

### Session Key Lost

If you lose your session key:
1. Run `bw unlock` to generate a new one
2. Export the new session key

### Vault Won't Unlock

If unlock fails:
1. Verify your master password is correct
2. Check your internet connection (needed for prelogin)
3. Try `bw logout` then `bw login` to start fresh

### Commands Not Working After System Restart

Session keys don't persist across system restarts. After restarting:
1. Run `bw unlock`
2. Export the new session key

### Multiple Accounts

If you have multiple Bitwarden accounts:
1. Each login creates separate storage
2. Use `bw logout` before switching accounts
3. Each account has its own session key

# README.md Updates for Tool Commands Enhancement

This document contains the specific updates that should be made to the main README.md file.

## Section 1: Development Status

**Current content (lines 68-77):**

```markdown
## Development Status

This project is in early development. Currently implemented:
- ✅ Project structure and build configuration
- ✅ CLI parsing with all commands
- ✅ Global flags and environment variables
- ✅ Response formatting system
- ✅ SDK integration setup
- 🚧 Command implementations (stubs only)

All commands currently return "Not yet implemented". See the [enhancement plan](enhancements/) for implementation roadmap.
```

**Replace with:**

```markdown
## Development Status

This project is in active development. Currently implemented:
- ✅ Project structure and build configuration
- ✅ CLI parsing with all commands
- ✅ Global flags and environment variables
- ✅ Response formatting system
- ✅ SDK integration setup
- ✅ Storage layer (JSON-based)
- ✅ API client configuration
- ✅ Authentication commands (login, logout, unlock, lock, status)
- ✅ Vault read commands (list, get)
- ✅ Vault write commands (create, edit, delete)
- ✅ Tool commands (generate, encode, send template)
- 🚧 Send operations (CRUD) - deferred pending SDK integration
- 🚧 Import/Export - not yet implemented

See the [enhancement plan](enhancements/) for detailed implementation status.
```

## Section 2: Tool Commands (New Section)

**Insert after line 66 (after "Global Flags" section), before "Development Status":**

```markdown
## Tool Commands

### Password Generation

Generate cryptographically secure passwords with customizable options:

```bash
# Generate default 14-character password
bw generate

# Custom length password (5-128 characters)
bw generate --length 20

# Password with minimum character requirements
bw generate --length 16 --number 3 --special 2

# Password with specific character sets (set to 0 to disable)
bw generate --uppercase 0 --special 0  # lowercase and numbers only
```

**Available Options:**
- `--length N` - Password length (5-128, default: 14)
- `--lowercase N` - Minimum lowercase characters (default: 0)
- `--uppercase N` - Minimum uppercase characters (default: 0)
- `--number N` - Minimum numeric characters (default: 1)
- `--special N` - Minimum special characters (default: 1)

**Note:** By default, passwords include all character types. Set a minimum to 0 to exclude that character type entirely.

**Security:** All passwords are generated using a cryptographically secure random number generator (OsRng).

### Passphrase Generation

Generate memorable passphrases using the EFF long wordlist (7,776 words):

```bash
# Generate default 3-word passphrase
bw generate --passphrase
# Example output: "little-brook-variable"

# Custom word count (3-20 words)
bw generate --passphrase --words 5

# Capitalized words with number
bw generate --passphrase --capitalize --includeNumber
# Example output: "Hamster-Unvarying-Jokester-Fragrant-Eggplant-8472"

# Custom separator
bw generate --passphrase --separator " "
# Example output: "abruptly dynamic zeppelin"
```

**Available Options:**
- `--passphrase` - Generate passphrase instead of password
- `--words N` - Number of words (3-20, default: 3)
- `--separator STR` - Word separator (default: "-")
- `--capitalize` - Capitalize first letter of each word
- `--includeNumber` - Add random 4-digit number suffix

**Note:** Passphrases use the [EFF long wordlist](https://www.eff.org/deeplinks/2016/07/new-wordlists-random-passphrases) for maximum entropy and memorability.

### Base64 Encoding

Encode text data to base64 format:

```bash
# Encode string
bw encode "Hello World"
# Output: "SGVsbG8gV29ybGQ="

# Encode with special characters
bw encode "test data 123!@#"
# Output: "dGVzdCBkYXRhIDEyMyFAIw=="

# With JSON response format
bw encode "test data" --response
# Output: {"success":true,"data":{"data":"dGVzdCBkYXRh"}}
```

**Use Case:** Encode data for safe transmission or storage in text-based formats.

### Send Templates

Generate JSON templates for creating Sends (secure temporary shares):

```bash
# Text Send template (default)
bw send template
# Outputs JSON template for text Send

# File Send template
bw send template file
# Outputs JSON template for file Send

# Explicit text template
bw send template text
```

**Template Structure:**

Text Send template includes:
- `name` - Display name for the Send
- `text` - Text content and visibility settings
- `deletionDate` - When Send will be deleted (optional)
- `expirationDate` - When Send stops accepting access (optional)
- `maxAccessCount` - Maximum number of accesses (optional)
- `password` - Password protection (optional)
- `hideEmail` - Hide email from recipients
- `disabled` - Disable the Send

File Send template includes similar fields with file metadata instead of text content.

**Note:** Send CRUD operations (create, list, get, edit, delete) and the receive command are not yet implemented. They will be added in a future update pending SDK integration for encryption and API operations.

```

## Section 3: Usage Examples (Update)

**Current content (lines 42-55):**

```markdown
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
```

**Replace with:**

```markdown
## Usage

```bash
# Show help
bw --help

# Show version
bw --version

# Login
bw login

# Check status
bw status

# Generate a password
bw generate

# Generate a passphrase
bw generate --passphrase

# Encode data
bw encode "my data"

# Get Send template
bw send template
```
```

## Implementation Notes

### Changes Summary

1. **Development Status section**: Updated to reflect all completed enhancements
2. **Tool Commands section**: New comprehensive section documenting password generation, passphrase generation, base64 encoding, and Send templates
3. **Usage Examples section**: Updated to show working commands instead of stubs

### File Location

These updates should be applied to: `/Users/bgentry/Source/repos/bwcli-rs/README.md`

### Application Method

You can apply these updates in one of two ways:

1. **Manual editing**: Copy the new content and replace the specified sections
2. **Using Edit tool**: Use the Edit tool to replace the old content with the new content

### Testing After Updates

After updating README.md, verify:

1. All command examples work as documented
2. All links are valid
3. Formatting renders correctly on GitHub
4. Section ordering is logical
5. No typos or formatting errors

### Maintenance

When Send CRUD operations are implemented (Phase 3-6):

1. Remove the "Note:" disclaimers about unimplemented Send features
2. Add comprehensive Send command examples
3. Add receive command documentation
4. Update the Development Status to show Send operations as complete

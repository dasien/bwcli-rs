# Generate Command User Guide

The `bw generate` command creates secure passwords and passphrases using the Bitwarden SDK.

## Quick Start

```bash
# Generate a password
bw generate

# Generate a passphrase
bw generate --passphrase
```

## Password Generation

### Basic Usage

By default, `bw generate` creates a 16-character password containing lowercase letters, uppercase letters, numbers, and special characters.

```bash
$ bw generate
wKTlS%F!hnfEZ9RI
```

### Custom Length

Use `--length` to specify password length (4-128 characters):

```bash
$ bw generate --length 24
Prcu881#5J*^N7SiQXgP1234

$ bw generate --length 8
xK4#mP2Q
```

### Character Set Control

Control which character types are included using minimum count flags. Setting a minimum to `0` disables that character set:

```bash
# Alphanumeric only (no special characters)
$ bw generate --special 0
e0eIt0tRBqEbu88F

# Letters only (no numbers or special)
$ bw generate --number 0 --special 0
nOxXOPnkQumpqsYZ

# Numbers and special only
$ bw generate --lowercase 0 --uppercase 0
47!7&5@0*3^^1412
```

### Minimum Character Requirements

Ensure a minimum number of specific character types:

```bash
# At least 4 of each character type
$ bw generate --lowercase 4 --uppercase 4 --number 4 --special 4 --length 20
aB3$cD4%eF5^gH6&iJ7*

# Strong password with guaranteed complexity
$ bw generate --lowercase 2 --uppercase 2 --number 2 --special 2
```

## Passphrase Generation

### Basic Usage

Generate memorable passphrases using the `--passphrase` flag:

```bash
$ bw generate --passphrase
blatancy-overlying-ocean
```

### Word Count

Use `--words` to specify the number of words (3-20):

```bash
$ bw generate --passphrase --words 5
demote-correct-morbidly-bulldozer-attest

$ bw generate --passphrase --words 4
usual-cleft-opponent-dance
```

### Custom Separator

Change the word separator with `--separator`:

```bash
$ bw generate --passphrase --separator "."
commence.transpose.hastily

$ bw generate --passphrase --separator "_"
ozone_tidal_release

$ bw generate --passphrase --separator ""
ozonetidalrelease
```

### Capitalization and Numbers

Make passphrases more complex:

```bash
# Capitalize first letter of each word
$ bw generate --passphrase --capitalize
Denim-Job-Patrol

# Add a number to a random word
$ bw generate --passphrase --includeNumber
job9-patrol-denim

# Both options combined
$ bw generate --passphrase --capitalize --includeNumber
Denim-Job9-Patrol
```

## Output Formats

### Raw Output (Default)

By default, the generated value is printed directly:

```bash
$ bw generate
wKTlS%F!hnfEZ9RI
```

### JSON Response

Use `--response` for JSON output:

```bash
$ bw generate --response
{
  "success": true,
  "data": {
    "data": "wKTlS%F!hnfEZ9RI"
  }
}
```

### Pretty JSON

Combine with `--pretty` for formatted JSON:

```bash
$ bw generate --response --pretty
{
  "success": true,
  "data": {
    "data": "wKTlS%F!hnfEZ9RI"
  }
}
```

## Error Handling

### No Character Sets Enabled

```bash
$ bw generate --lowercase 0 --uppercase 0 --number 0 --special 0
No character sets enabled. Enable at least one of: lowercase, uppercase, numbers, or special characters
```

### Invalid Password Length

```bash
$ bw generate --length 3
Invalid password length. Length must be at least 4 and greater than the sum of minimum character requirements

$ bw generate --length 4 --lowercase 2 --uppercase 2 --number 2
Invalid password length. Length must be at least 4 and greater than the sum of minimum character requirements
```

### Invalid Word Count

```bash
$ bw generate --passphrase --words 2
Invalid word count. Number of words must be between 3 and 20

$ bw generate --passphrase --words 25
Invalid word count. Number of words must be between 3 and 20
```

## Common Use Cases

### Secure Master Password

```bash
# Long password with all character types
bw generate --length 24 --lowercase 3 --uppercase 3 --number 3 --special 3
```

### Wi-Fi Password

```bash
# Alphanumeric, easy to type
bw generate --length 16 --special 0
```

### Memorable Passphrase

```bash
# 4-word capitalized passphrase
bw generate --passphrase --words 4 --capitalize
```

### API Key Style

```bash
# Long alphanumeric string
bw generate --length 32 --special 0
```

### Script Integration

```bash
# Capture password in variable
PASSWORD=$(bw generate)

# Use in script with JSON
RESULT=$(bw generate --response)
PASSWORD=$(echo "$RESULT" | jq -r '.data.data')
```

## Technical Notes

### Random Number Generation

Passwords and passphrases are generated using the Bitwarden SDK's cryptographically secure random number generator:

- **Algorithm**: ChaCha12 CSPRNG
- **Seeding**: Operating system entropy (OsRng)
- **Security**: Equivalent to hardware RNG for cryptographic purposes

### Passphrase Wordlist

The SDK uses the EFF Large Wordlist containing 7,776 common English words, providing approximately:

- 12.9 bits of entropy per word
- 38.7 bits for a 3-word passphrase
- 51.7 bits for a 4-word passphrase
- 64.6 bits for a 5-word passphrase

## Option Reference

### Password Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--length` | number | 16 | Password length (4-128) |
| `--lowercase` | number | enabled | Min lowercase letters (0 disables) |
| `--uppercase` | number | enabled | Min uppercase letters (0 disables) |
| `--number` | number | enabled | Min numeric characters (0 disables) |
| `--special` | number | enabled | Min special characters (0 disables) |

### Passphrase Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--passphrase` | flag | false | Generate passphrase mode |
| `--words` | number | 3 | Number of words (3-20) |
| `--separator` | string | `-` | Word separator |
| `--capitalize` | flag | false | Capitalize words |
| `--includeNumber` | flag | false | Add number to word |

### Output Options

| Option | Type | Description |
|--------|------|-------------|
| `--response` | flag | Output as JSON |
| `--pretty` | flag | Format JSON with indentation |

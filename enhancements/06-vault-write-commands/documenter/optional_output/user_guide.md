# Vault Write Operations - User Guide

> **Note**: This guide documents the service layer implementation. CLI commands are planned for Enhancement 07. This guide will be useful once the command layer is implemented.

## Overview

Bitwarden CLI's vault write operations allow you to create, update, delete, restore, and move vault items and folders directly from the command line. This guide covers all write operations available in the Rust implementation.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Item Operations](#item-operations)
  - [Creating Items](#creating-items)
  - [Updating Items](#updating-items)
  - [Deleting Items](#deleting-items)
  - [Restoring Items](#restoring-items)
  - [Moving Items](#moving-items)
- [Folder Operations](#folder-operations)
  - [Creating Folders](#creating-folders)
  - [Updating Folders](#updating-folders)
  - [Deleting Folders](#deleting-folders)
- [Item Types Reference](#item-types-reference)
- [Validation Rules](#validation-rules)
- [Error Handling](#error-handling)
- [Tips and Best Practices](#tips-and-best-practices)

## Prerequisites

Before using vault write operations:

1. **Authentication Required**: You must be logged in with a valid session
2. **Vault Synced**: Ensure your vault is synced (`bw sync`)
3. **Valid JSON**: Item data must be properly formatted JSON
4. **Permissions**: You must have write access to the vault

## Item Operations

### Creating Items

Create a new vault item by providing a JSON structure with the required fields.

#### Basic Login Item

```bash
# Create a login item (type=1)
echo '{
  "type": 1,
  "name": "GitHub Account",
  "login": {
    "username": "user@example.com",
    "password": "secure-password-here",
    "uris": [
      {
        "uri": "https://github.com",
        "match": null
      }
    ]
  },
  "notes": "My GitHub credentials"
}' | bw create item
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "organizationId": null,
  "folderId": null,
  "type": 1,
  "name": "2.encrypted_GitHub Account",
  "notes": "2.encrypted_My GitHub credentials",
  "favorite": false,
  "login": {
    "username": "2.encrypted_user@example.com",
    "password": "2.encrypted_secure-password-here",
    "uris": [
      {
        "uri": "2.encrypted_https://github.com",
        "match": null
      }
    ],
    "totp": null
  },
  "creationDate": "2025-12-05T12:00:00Z",
  "revisionDate": "2025-12-05T12:00:00Z",
  "deletedDate": null
}
```

#### Login with TOTP

```bash
echo '{
  "type": 1,
  "name": "AWS Console",
  "login": {
    "username": "admin@company.com",
    "password": "aws-password",
    "totp": "otpauth://totp/AWS:admin@company.com?secret=JBSWY3DPEHPK3PXP&issuer=AWS",
    "uris": [
      {
        "uri": "https://console.aws.amazon.com"
      }
    ]
  }
}' | bw create item
```

#### Secure Note

```bash
# Create a secure note (type=2)
echo '{
  "type": 2,
  "name": "API Keys",
  "secureNote": {
    "type": 0
  },
  "notes": "Production API Key: sk-prod-abc123\nStaging API Key: sk-stag-xyz789"
}' | bw create item
```

#### Credit Card

```bash
# Create a card item (type=3)
echo '{
  "type": 3,
  "name": "Business Credit Card",
  "card": {
    "cardholderName": "John Doe",
    "number": "4111111111111111",
    "brand": "Visa",
    "expMonth": "12",
    "expYear": "2025",
    "code": "123"
  }
}' | bw create item
```

#### Identity

```bash
# Create an identity item (type=4)
echo '{
  "type": 4,
  "name": "Passport Information",
  "identity": {
    "title": "Mr",
    "firstName": "John",
    "middleName": "Q",
    "lastName": "Doe",
    "address1": "123 Main St",
    "city": "New York",
    "state": "NY",
    "postalCode": "10001",
    "country": "US",
    "phone": "+1-555-1234",
    "email": "john.doe@example.com",
    "ssn": "123-45-6789",
    "passportNumber": "X12345678"
  }
}' | bw create item
```

#### Item in Folder

```bash
# Create item in a specific folder
echo '{
  "type": 1,
  "name": "Development DB",
  "folderId": "a7c2f4e1-3b5d-4f8e-9a2c-1d4e7f8a9b2c",
  "login": {
    "username": "dbadmin",
    "password": "db-password",
    "uris": [
      {
        "uri": "postgresql://localhost:5432/devdb"
      }
    ]
  }
}' | bw create item
```

### Updating Items

Update an existing item by providing its ID and the modified JSON structure.

```bash
# Get the current item
bw get item <item-id> > item.json

# Edit the JSON file
# (e.g., change password, add a URI, update notes)

# Update the item
cat item.json | bw edit item <item-id>
```

**Example: Change password**
```bash
# Retrieve item
bw get item 550e8400-e29b-41d4-a716-446655440000 | \
  jq '.login.password = "new-secure-password"' | \
  bw edit item 550e8400-e29b-41d4-a716-446655440000
```

**Example: Add a URI**
```bash
bw get item 550e8400-e29b-41d4-a716-446655440000 | \
  jq '.login.uris += [{"uri": "https://github.com/login"}]' | \
  bw edit item 550e8400-e29b-41d4-a716-446655440000
```

**Example: Update notes**
```bash
bw get item 550e8400-e29b-41d4-a716-446655440000 | \
  jq '.notes = "Updated notes with new information"' | \
  bw edit item 550e8400-e29b-41d4-a716-446655440000
```

### Deleting Items

#### Soft Delete (Move to Trash)

```bash
# Move item to trash (can be restored)
bw delete item <item-id>
```

The item is marked with a `deletedDate` timestamp but remains in the vault. You can restore it later.

#### Permanent Delete

```bash
# Permanently delete (cannot be undone)
bw delete item <item-id> --permanent
```

**Interactive Confirmation:**
```
Are you sure you want to permanently delete this item? This action cannot be undone.
Type 'yes' to confirm: yes
Item permanently deleted.
```

**Skip Confirmation (for scripts):**
```bash
bw delete item <item-id> --permanent --nointeraction
```

⚠️ **Warning**: Permanent deletion cannot be undone. The item is removed from the server and cannot be recovered.

### Restoring Items

Restore an item that was moved to trash (soft deleted).

```bash
# Restore a deleted item
bw restore item <item-id>
```

**Example:**
```bash
# Delete an item
bw delete item 550e8400-e29b-41d4-a716-446655440000

# List items in trash
bw list items --trash

# Restore the item
bw restore item 550e8400-e29b-41d4-a716-446655440000
```

**Note**: You can only restore items that were soft-deleted. Permanently deleted items cannot be restored.

### Moving Items

Move an item to a different folder.

```bash
# Move item to a folder
bw move <item-id> <folder-id>

# Remove item from folder (move to root)
bw move <item-id> null
```

**Example:**
```bash
# Create a folder first
FOLDER_ID=$(bw create folder "Work Accounts" | jq -r '.id')

# Move an item to the folder
bw move 550e8400-e29b-41d4-a716-446655440000 $FOLDER_ID
```

## Folder Operations

### Creating Folders

Create a new folder to organize your vault items.

```bash
# Create a folder
bw create folder "Work"

# Create a folder with special characters
bw create folder "Personal/Banking"
```

**Response:**
```json
{
  "id": "a7c2f4e1-3b5d-4f8e-9a2c-1d4e7f8a9b2c",
  "name": "2.encrypted_Work",
  "revisionDate": "2025-12-05T12:00:00Z"
}
```

**Folder Naming:**
- Maximum 1000 characters
- Cannot be empty
- Can include spaces and special characters
- Case-sensitive

### Updating Folders

Rename an existing folder.

```bash
# Rename a folder
bw edit folder <folder-id> "New Folder Name"
```

**Example:**
```bash
# Get folder ID
FOLDER_ID=$(bw list folders | jq -r '.[] | select(.name == "Work") | .id')

# Rename the folder
bw edit folder $FOLDER_ID "Work Accounts"
```

### Deleting Folders

Delete a folder. Items in the folder are not deleted—they become unfoldered.

```bash
# Delete a folder
bw delete folder <folder-id>
```

**Example:**
```bash
# List folders
bw list folders

# Delete a folder
bw delete folder a7c2f4e1-3b5d-4f8e-9a2c-1d4e7f8a9b2c
```

**Note**: Deleting a folder does not delete the items inside it. The items will remain in your vault without a folder assignment.

## Item Types Reference

### Type Values

| Type | Value | Description |
|------|-------|-------------|
| Login | 1 | Username/password with URIs and TOTP |
| SecureNote | 2 | Encrypted notes |
| Card | 3 | Credit/debit card information |
| Identity | 4 | Personal identity information |

### Login Fields (type=1)

**Required:**
- `type`: 1
- `name`: Item name (max 1000 chars)
- `login`: Login object

**Login Object:**
- `username`: Username or email
- `password`: Password
- `uris`: Array of URI objects
  - `uri`: The URL (max 10000 chars)
  - `match`: Match type (optional)
- `totp`: TOTP key in `otpauth://` format (optional)

**Optional:**
- `notes`: Encrypted notes (max 10000 chars)
- `folderId`: UUID of folder
- `favorite`: Boolean
- `fields`: Array of custom fields

### SecureNote Fields (type=2)

**Required:**
- `type`: 2
- `name`: Note title (max 1000 chars)
- `secureNote`: SecureNote object
  - `type`: Note type (0 = generic)

**Optional:**
- `notes`: Note content (max 10000 chars)
- `folderId`: UUID of folder
- `favorite`: Boolean

### Card Fields (type=3)

**Required:**
- `type`: 3
- `name`: Card name (max 1000 chars)
- `card`: Card object

**Card Object:**
- `cardholderName`: Name on card
- `number`: Card number
- `brand`: Card brand (Visa, Mastercard, etc.)
- `expMonth`: Expiration month (MM)
- `expYear`: Expiration year (YYYY)
- `code`: CVV/CVC code

**Optional:**
- `notes`: Additional notes (max 10000 chars)
- `folderId`: UUID of folder
- `favorite`: Boolean

### Identity Fields (type=4)

**Required:**
- `type`: 4
- `name`: Identity name (max 1000 chars)
- `identity`: Identity object

**Identity Object (all optional):**
- `title`: Title (Mr, Mrs, Dr, etc.)
- `firstName`: First name
- `middleName`: Middle name
- `lastName`: Last name
- `address1`: Address line 1
- `address2`: Address line 2
- `address3`: Address line 3
- `city`: City
- `state`: State/Province
- `postalCode`: Postal/ZIP code
- `country`: Country code
- `company`: Company name
- `email`: Email address
- `phone`: Phone number
- `ssn`: Social Security Number
- `username`: Username
- `passportNumber`: Passport number
- `licenseNumber`: License number

## Validation Rules

The CLI validates all input before submitting to the API. Here are the validation rules:

### Field Constraints

| Field | Constraint | Error Message |
|-------|-----------|---------------|
| `name` | Required, ≤1000 chars | "Required field 'name' missing" or "Field 'name' too long (max 1000)" |
| `notes` | Optional, ≤10000 chars | "Field 'notes' too long (max 10000)" |
| `type` | Required, 1-4 | "Invalid cipher type: {type}" |
| `login` | Required if type=1 | "Type mismatch: cipher type Login requires login" |
| `secureNote` | Required if type=2 | "Type mismatch: cipher type SecureNote requires secure_note" |
| `card` | Required if type=3 | "Type mismatch: cipher type Card requires card" |
| `identity` | Required if type=4 | "Type mismatch: cipher type Identity requires identity" |
| `folderId` | Valid UUID if present | "Invalid UUID format for 'folderId'" |
| `organizationId` | Valid UUID if present | "Invalid UUID format for 'organizationId'" |
| `totp` | otpauth:// URI if present | "Invalid format for 'totp': expected otpauth:// URI" |
| `uris[].uri` | ≤10000 chars each | "Field 'uri' too long (max 10000)" |

### UUID Format

UUIDs must be in the standard format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`

**Valid examples:**
- `550e8400-e29b-41d4-a716-446655440000`
- `a7c2f4e1-3b5d-4f8e-9a2c-1d4e7f8a9b2c`

**Invalid examples:**
- `invalid-uuid`
- `123456`
- `550e8400e29b41d4a716446655440000` (missing hyphens)

### TOTP Format

TOTP keys must use the `otpauth://` URI scheme.

**Valid format:**
```
otpauth://totp/Service:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Service
```

**Components:**
- `otpauth://totp/` - Required prefix
- `Service:user@example.com` - Label (service and account)
- `?secret=JBSWY3DPEHPK3PXP` - Base32 encoded secret (required)
- `&issuer=Service` - Issuer (recommended)

## Error Handling

### Common Errors

#### Validation Errors

**Missing Required Field:**
```
Error: Required field 'name' missing

Solution: Ensure the 'name' field is present in your JSON.
```

**Field Too Long:**
```
Error: Field 'notes' too long (max 10000 characters)

Solution: Reduce the length of the notes field.
```

**Type Mismatch:**
```
Error: Type mismatch: cipher type Login requires login

Solution: Add the 'login' object to your JSON when creating a login item (type=1).
```

**Invalid UUID:**
```
Error: Invalid UUID format for 'folderId'

Solution: Use a valid UUID format (e.g., 550e8400-e29b-41d4-a716-446655440000).
```

#### Operation Errors

**Item Not Found:**
```
Error: Cipher not found: 550e8400-e29b-41d4-a716-446655440000

Solution: Verify the item ID is correct. Sync your vault with 'bw sync'.
```

**Folder Not Found:**
```
Error: Folder not found: a7c2f4e1-3b5d-4f8e-9a2c-1d4e7f8a9b2c

Solution: Verify the folder ID is correct. List folders with 'bw list folders'.
```

**Authentication Required:**
```
Error: You are not logged in.

Solution: Log in with 'bw login' and set the session key.
```

**Operation Cancelled:**
```
Error: Operation cancelled by user

Context: Occurs when you decline a permanent delete confirmation.
```

#### Network Errors

**API Error:**
```
Error: API request failed: 400 Bad Request

Solution: Check your input data format. Ensure all required fields are present.
```

**Connection Error:**
```
Error: Failed to connect to Bitwarden server

Solution: Check your internet connection. Verify the server URL is correct.
```

### Error Response Format

Errors are returned in JSON format when using `--response`:

```json
{
  "success": false,
  "message": "Field 'name' too long (max 1000)",
  "data": {
    "field": "name",
    "constraint": "max_length",
    "limit": 1000
  }
}
```

## Tips and Best Practices

### Security Best Practices

1. **Use Strong Passwords**: Generate passwords with `bw generate`
2. **Enable TOTP**: Add two-factor authentication keys when available
3. **Confirm Destructive Operations**: Always review before confirming permanent delete
4. **Use Folders**: Organize items into folders for better security and usability
5. **Regular Backups**: Export your vault regularly with `bw export`

### Workflow Tips

#### Batch Create Items

```bash
# Create multiple items from a JSON array
cat items.json | jq -c '.[]' | while read item; do
  echo "$item" | bw create item
done
```

#### Update Multiple Items

```bash
# Update all items in a folder
bw list items --folderid <folder-id> | jq -c '.[]' | while read item; do
  echo "$item" | jq '.favorite = true' | bw edit item "$(echo "$item" | jq -r '.id')"
done
```

#### Safe Delete Pattern

```bash
# Soft delete first, then permanently delete after verification
bw delete item <item-id>
bw list items --trash  # Verify it's in trash
bw delete item <item-id> --permanent --nointeraction
```

#### Move Items to New Folder

```bash
# Create folder and move items in one workflow
FOLDER_ID=$(bw create folder "New Folder" | jq -r '.id')
bw list items --folderid null | jq -r '.[].id' | while read id; do
  bw move "$id" "$FOLDER_ID"
done
```

### Scripting Tips

#### Use --nointeraction Flag

Disable interactive prompts for automated scripts:

```bash
bw delete item <item-id> --permanent --nointeraction
```

#### Check Exit Codes

```bash
if bw create folder "Test"; then
  echo "Folder created successfully"
else
  echo "Failed to create folder"
  exit 1
fi
```

#### Use --response for JSON Output

Parse responses in scripts:

```bash
ITEM_ID=$(bw create item < item.json | jq -r '.id')
if [ -z "$ITEM_ID" ]; then
  echo "Failed to create item"
  exit 1
fi
```

#### Environment Variables

Set environment variables to avoid repeating flags:

```bash
export BW_SESSION="your-session-key"
export BW_RESPONSE=true
export BW_NOINTERACTION=true

bw create folder "Automated"
```

### JSON Template Tips

#### Use Templates

Generate templates for item types:

```bash
# Future feature: Template generation
bw get template item.login > login_template.json
bw get template item.card > card_template.json
```

#### Validate JSON Before Submitting

```bash
# Validate JSON syntax
cat item.json | jq empty && echo "Valid JSON" || echo "Invalid JSON"

# Check required fields
cat item.json | jq 'has("name") and has("type")'
```

#### Pretty Print for Editing

```bash
# Get item in readable format
bw get item <item-id> --pretty > item.json

# Edit in your favorite editor
$EDITOR item.json

# Submit changes
cat item.json | bw edit item <item-id>
```

### Performance Tips

1. **Batch Operations**: Use scripts to batch multiple operations
2. **Cache Locally**: Store item IDs locally instead of listing repeatedly
3. **Use --quiet**: Suppress output for faster script execution
4. **Sync Periodically**: Don't sync after every operation; sync once at the end

### Troubleshooting

#### Item Not Appearing After Creation

```bash
# Force a sync
bw sync --force

# Verify item exists
bw get item <item-id>
```

#### JSON Parsing Errors

```bash
# Validate JSON syntax
cat item.json | jq .

# Check for common issues:
# - Missing commas
# - Extra commas (trailing)
# - Unquoted strings
# - Incorrect nesting
```

#### Encryption Errors

```bash
# Ensure you're logged in with a valid session
bw login
export BW_SESSION="..."

# Verify vault is unlocked
bw unlock
```

## Advanced Examples

### Create Login with Custom Fields

```bash
echo '{
  "type": 1,
  "name": "Database Connection",
  "login": {
    "username": "dbadmin",
    "password": "secure-password"
  },
  "fields": [
    {
      "name": "Server",
      "value": "db.example.com",
      "type": 0
    },
    {
      "name": "Port",
      "value": "5432",
      "type": 0
    },
    {
      "name": "Database",
      "value": "production",
      "type": 0
    }
  ]
}' | bw create item
```

### Update Only Specific Fields

```bash
# Update only the password
bw get item <item-id> | \
  jq '.login.password = "new-password"' | \
  bw edit item <item-id>

# Add a note without changing other fields
bw get item <item-id> | \
  jq '.notes = "Important: Password expires in 90 days"' | \
  bw edit item <item-id>

# Mark as favorite
bw get item <item-id> | \
  jq '.favorite = true' | \
  bw edit item <item-id>
```

### Organize Items by Type

```bash
# Create folders for each item type
LOGINS=$(bw create folder "Logins" | jq -r '.id')
NOTES=$(bw create folder "Secure Notes" | jq -r '.id')
CARDS=$(bw create folder "Payment Cards" | jq -r '.id')

# Move items to appropriate folders
bw list items --response | jq -r '.[] | select(.type == 1) | .id' | \
  while read id; do bw move "$id" "$LOGINS"; done

bw list items --response | jq -r '.[] | select(.type == 2) | .id' | \
  while read id; do bw move "$id" "$NOTES"; done

bw list items --response | jq -r '.[] | select(.type == 3) | .id' | \
  while read id; do bw move "$id" "$CARDS"; done
```

### Backup Before Bulk Changes

```bash
# Export vault before making bulk changes
bw export --format json --output backup_$(date +%Y%m%d).json

# Make your changes
# ...

# If something goes wrong, restore from backup
# (Manual restore process via web vault or import)
```

## Future Features

The following features are planned for future releases:

- **Template Generation**: `bw get template item.login`
- **Attachment Support**: `bw create attachment`, `bw delete attachment`
- **Organization Features**: `bw share`, `bw confirm`
- **Collection Management**: `bw create org-collection`
- **Batch Operations**: Native support for bulk operations
- **Interactive Editors**: Edit items in your $EDITOR

## Additional Resources

- [Bitwarden CLI Documentation](https://bitwarden.com/help/cli/)
- [Bitwarden API Reference](https://bitwarden.com/help/api/)
- [GitHub Repository](https://github.com/bitwarden/clients)
- [Community Forum](https://community.bitwarden.com/)

## Getting Help

If you encounter issues:

1. Check the error message for specific guidance
2. Verify your input JSON is valid
3. Ensure you're authenticated (`bw login`)
4. Sync your vault (`bw sync`)
5. Check the [troubleshooting guide](#troubleshooting)
6. Visit the [community forum](https://community.bitwarden.com/)
7. Report bugs on [GitHub](https://github.com/bitwarden/clients/issues)

---

**Last Updated**: 2025-12-05
**Version**: 1.0 (Service Layer Complete, Commands Pending)

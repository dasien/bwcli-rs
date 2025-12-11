# Vault Write Commands Guide

This guide covers creating, editing, deleting, restoring, and moving vault items using the Bitwarden CLI.

## Prerequisites

Before using vault write commands, you must:

1. **Login** to your Bitwarden account:
   ```bash
   bw login
   ```

2. **Unlock** your vault:
   ```bash
   bw unlock
   ```

3. **Set your session key** either via environment variable or `--session` flag:
   ```bash
   # Using environment variable
   export BW_SESSION="your-session-key"

   # Or using --session flag with each command
   bw create item <json> --session "your-session-key"
   ```

## Getting Started with Templates

The easiest way to create items is to start with a template:

```bash
# Get a login item template
bw get template item.login

# Get a secure note template
bw get template item.secureNote

# Get a card template
bw get template item.card

# Get an identity template
bw get template item.identity

# Get a folder template
bw get template folder
```

### Available Template Types

| Template Type | Command | Description |
|--------------|---------|-------------|
| Login | `item` or `item.login` | Passwords and credentials |
| Secure Note | `item.secureNote` | Encrypted text notes |
| Card | `item.card` | Payment card information |
| Identity | `item.identity` | Personal identification info |
| Folder | `folder` | Organize items in folders |
| Custom Field | `item.field` | Add custom fields to items |
| URI | `item.login.uri` | Login URI entry |

## Creating Items

### Create a Login Item

1. Get the template:
   ```bash
   bw get template item.login > login.json
   ```

2. Edit `login.json` with your data:
   ```json
   {
     "organizationId": null,
     "folderId": null,
     "type": 1,
     "name": "My Website Login",
     "notes": "Created via CLI",
     "favorite": false,
     "login": {
       "uris": [
         {
           "match": null,
           "uri": "https://mywebsite.com"
         }
       ],
       "username": "myusername",
       "password": "mysecurepassword",
       "totp": null
     }
   }
   ```

3. Create the item using one of these methods:

   **Raw JSON:**
   ```bash
   bw create item '{"type":1,"name":"My Login","login":{"username":"user","password":"pass"}}'
   ```

   **Base64 encoded (TypeScript CLI compatible):**
   ```bash
   bw create item "$(cat login.json | base64)"
   ```

   **Stdin:**
   ```bash
   cat login.json | bw create item -
   ```

### Create a Secure Note

```bash
bw create item '{
  "type": 2,
  "name": "My Secret Note",
  "notes": "This is the secret content of my note.",
  "secureNote": {"type": 0}
}'
```

### Create a Card

```bash
bw create item '{
  "type": 3,
  "name": "My Credit Card",
  "card": {
    "cardholderName": "John Doe",
    "brand": "visa",
    "number": "4111111111111111",
    "expMonth": "12",
    "expYear": "2025",
    "code": "123"
  }
}'
```

### Create an Identity

```bash
bw create item '{
  "type": 4,
  "name": "Personal Identity",
  "identity": {
    "firstName": "John",
    "lastName": "Doe",
    "email": "john@example.com",
    "phone": "555-1234"
  }
}'
```

### Create a Folder

```bash
bw create folder '{"name":"My New Folder"}'
```

## Editing Items

To edit an item, you need its ID and the updated JSON:

```bash
# Get the item ID first
bw list items --search "My Website Login"

# Edit the item
bw edit item abc12345-6789-def0-1234-567890abcdef '{
  "type": 1,
  "name": "My Website Login (Updated)",
  "login": {
    "username": "newusername",
    "password": "newpassword"
  }
}'
```

### Merge Behavior

When editing, the CLI **merges** your changes with the existing item:
- Fields you specify are updated
- Fields you omit are preserved
- This allows partial updates without re-specifying everything

### Edit a Folder

```bash
bw edit folder abc12345-6789-def0-1234-567890abcdef '{"name":"Renamed Folder"}'
```

## Deleting Items

### Soft Delete (Move to Trash)

```bash
# Delete an item (moves to trash)
bw delete item abc12345-6789-def0-1234-567890abcdef
```

### Permanent Delete

```bash
# Permanently delete an item
bw delete item abc12345-6789-def0-1234-567890abcdef --permanent
```

### Delete a Folder

```bash
# Delete a folder (items in folder are moved to "No Folder")
bw delete folder abc12345-6789-def0-1234-567890abcdef
```

## Restoring Items

Restore an item from trash:

```bash
bw restore abc12345-6789-def0-1234-567890abcdef
```

## Moving Items

### Move to a Folder

```bash
# Get folder ID
bw list folders

# Move item to folder
bw move abc12345-item-id fghij67890-folder-id
```

### Remove from Folder

To remove an item from its current folder (move to "No Folder"):

```bash
bw move abc12345-item-id null
```

## Input Formats

The CLI accepts JSON input in three formats:

### 1. Raw JSON

JSON starting with `{` is parsed directly:

```bash
bw create item '{"type":1,"name":"Test","login":{"username":"user"}}'
```

### 2. Base64 Encoded

For compatibility with the TypeScript CLI:

```bash
# Encode your JSON
echo '{"type":1,"name":"Test"}' | base64

# Use the encoded string
bw create item "eyJ0eXBlIjoxLCJuYW1lIjoiVGVzdCJ9"
```

### 3. Stdin

Use `-` to read from stdin:

```bash
# From a file
cat myitem.json | bw create item -

# From a pipeline
jq '.name = "Updated Name"' myitem.json | bw create item -
```

## Scripting Examples

### Create Multiple Items

```bash
#!/bin/bash
# Create multiple logins from a CSV

while IFS=, read -r name url username password; do
  bw create item "{
    \"type\": 1,
    \"name\": \"$name\",
    \"login\": {
      \"uris\": [{\"uri\": \"$url\"}],
      \"username\": \"$username\",
      \"password\": \"$password\"
    }
  }"
done < logins.csv
```

### Update All Items in a Folder

```bash
#!/bin/bash
FOLDER_ID="abc123"

# Get all items in folder
bw list items --folderid "$FOLDER_ID" | jq -r '.[].id' | while read -r id; do
  # Get current item
  item=$(bw get item "$id")

  # Modify and update (example: add a note)
  echo "$item" | jq '.notes = "Updated via script"' | bw edit item "$id" -
done
```

### Export and Re-import

```bash
# Export an item
bw get item abc123 > backup.json

# Re-create it (remove the ID first)
jq 'del(.id)' backup.json | bw create item -
```

## Error Handling

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "Vault is locked" | Session key not set or expired | Run `bw unlock` |
| "Invalid base64 encoding" | Malformed base64 input | Check encoding or use raw JSON |
| "Invalid JSON" | Malformed JSON syntax | Validate your JSON |
| "Missing required field: name" | Item name not provided | Add "name" field |
| "Item not found" | Invalid item ID | Check the ID with `bw list items` |
| "Cannot edit items in trash" | Trying to edit deleted item | Restore first with `bw restore` |
| "Item is not in trash" | Trying to restore non-deleted item | Item is already active |
| "Folder not found" | Invalid folder ID | Check ID with `bw list folders` |

### Validation Rules

Items must satisfy these rules:
- `name`: Required, max 1000 characters
- `notes`: Optional, max 10000 characters
- `uri` (in login): Max 10000 characters each
- `folderId`: Must be valid UUID format if present
- `totp`: Must be `otpauth://` URI if present

## Best Practices

1. **Use templates**: Start with `bw get template` for correct structure
2. **Validate JSON**: Use `jq` or similar to validate before creating
3. **Test with `--raw`**: Add `--raw` flag to see full output without formatting
4. **Sync after changes**: Run `bw sync` to ensure server consistency
5. **Script safely**: Use `set -e` in scripts to stop on errors
6. **Protect passwords**: Avoid putting passwords directly in command line history
   ```bash
   # Better approach - read from secure source
   PASSWORD=$(pass show myservice) bw create item "{...}"
   ```

## Troubleshooting

### Command Not Working

1. Check vault is unlocked: `bw status`
2. Verify session key is set: `echo $BW_SESSION`
3. Try with `--session` flag explicitly
4. Check JSON syntax with `jq`

### Item Not Appearing

1. Run `bw sync` to refresh local cache
2. Check if item is in trash: `bw list items --trash`
3. Verify folder filter if using `--folderid`

### Changes Not Syncing

1. Check internet connection
2. Run `bw sync` manually
3. Check for API errors in output

## Related Commands

| Command | Purpose |
|---------|---------|
| `bw list items` | List all vault items |
| `bw get item <id>` | Get a specific item |
| `bw list folders` | List all folders |
| `bw get folder <id>` | Get a specific folder |
| `bw sync` | Sync vault with server |
| `bw status` | Check login/lock status |

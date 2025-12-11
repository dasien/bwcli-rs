# Error Scenarios and Messages

This document defines the expected error scenarios and messages for vault create/edit commands.

## Input Parsing Errors

| Scenario | Command | Error Message |
|----------|---------|---------------|
| Invalid base64 | `bw create item abc!@#` | "Error: Invalid base64 encoding" |
| Invalid JSON | `bw create item <valid_b64_of_invalid_json>` | "Error: Invalid JSON: {parse_error}" |
| Missing JSON arg | `bw create item` | "Error: Missing required argument: JSON" |
| Empty JSON | `bw create item ""` | "Error: JSON input cannot be empty" |

## Validation Errors

| Scenario | Field | Error Message |
|----------|-------|---------------|
| Missing required field | `name` | "Error: Field 'name' is required" |
| Empty required field | `name` | "Error: Field 'name' cannot be empty" |
| Field too long | `name` (>1000) | "Error: Field 'name' exceeds maximum length of 1000 characters (actual: {len})" |
| Field too long | `notes` (>10000) | "Error: Field 'notes' exceeds maximum length of 10000 characters (actual: {len})" |
| Field too long | `uri` (>10000) | "Error: Field 'uri' exceeds maximum length of 10000 characters (actual: {len})" |
| Invalid UUID | `folderId` | "Error: Invalid UUID format for 'folderId': {value}" |
| Invalid UUID | `organizationId` | "Error: Invalid UUID format for 'organizationId': {value}" |
| Invalid TOTP format | `totp` | "Error: Invalid TOTP format. Expected 'otpauth://' URI" |
| Type mismatch | Login cipher without login | "Error: Field 'login' is required for cipher type Login" |
| Type mismatch | Card cipher without card | "Error: Field 'card' is required for cipher type Card" |
| Type mismatch | Identity without identity | "Error: Field 'identity' is required for cipher type Identity" |
| Type mismatch | SecureNote without secureNote | "Error: Field 'secureNote' is required for cipher type SecureNote" |
| Invalid cipher type | `type: 99` | "Error: Unknown cipher type: 99" |

## Authentication Errors

| Scenario | Error Message |
|----------|---------------|
| No session provided | "Error: Vault is locked. Run 'bw unlock' and set BW_SESSION environment variable." |
| Invalid session | "Error: Invalid session key. Please unlock your vault." |
| Session expired | "Error: Session expired. Please unlock your vault again." |

## Resource Errors

| Scenario | Command | Error Message |
|----------|---------|---------------|
| Item not found | `bw edit item <id>` | "Error: Item not found: {id}" |
| Item not found | `bw delete item <id>` | "Error: Item not found: {id}" |
| Item not found | `bw restore item <id>` | "Error: Item not found: {id}" |
| Item not found | `bw move <id> <folder>` | "Error: Item not found: {id}" |
| Folder not found | `bw edit folder <id>` | "Error: Folder not found: {id}" |
| Folder not found | `bw delete folder <id>` | "Error: Folder not found: {id}" |
| Folder not found | `bw move <item> <id>` | "Error: Folder not found: {id}" |
| Folder not found | Create with invalid folderId | "Error: Folder not found: {folderId}" |

## State Errors

| Scenario | Command | Error Message |
|----------|---------|---------------|
| Edit deleted item | `bw edit item <id>` | "Error: Cannot edit deleted item. Restore it first or delete permanently." |
| Restore non-deleted | `bw restore item <id>` | "Error: Item is not in trash" |
| Vault not synced | Any write command | "Error: Vault not synced. Run 'bw sync' first." |

## API Errors

| Scenario | Error Message |
|----------|---------------|
| Network error | "Error: Network error: {details}" |
| API error 400 | "Error: Bad request: {api_message}" |
| API error 401 | "Error: Unauthorized. Please log in again." |
| API error 403 | "Error: Forbidden. You don't have permission for this operation." |
| API error 404 | "Error: Resource not found on server." |
| API error 500 | "Error: Server error. Please try again later." |
| Timeout | "Error: Request timed out. Please try again." |

## Template Errors

| Scenario | Command | Error Message |
|----------|---------|---------------|
| Unknown template | `bw get template foo` | "Error: Unknown template type: 'foo'. Valid types: item.login, item.secureNote, item.card, item.identity, folder" |

## Error Response Format

All errors should follow the standard CLI response format:

```json
{
  "success": false,
  "message": "Error: {error_message}"
}
```

With `--raw` flag, only the error message is output (no JSON wrapper).

## Error Codes (Future Consideration)

For scripting purposes, consider adding error codes:

| Code | Category |
|------|----------|
| 1 | General error |
| 2 | Authentication error |
| 3 | Validation error |
| 4 | Resource not found |
| 5 | State error (e.g., item deleted) |
| 6 | Network/API error |
| 7 | Parse error (JSON/base64) |

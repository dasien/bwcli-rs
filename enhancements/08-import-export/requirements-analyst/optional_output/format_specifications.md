# Import/Export Format Specifications

## Overview

This document provides detailed specifications for each import and export format supported by the Bitwarden CLI. This is reference material for the architecture and implementation phases.

## Export Formats

### 1. CSV Format (Bitwarden)

**Purpose:** Simple, human-readable export format compatible with spreadsheet applications.

**Structure:**
```csv
folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp
```

**Field Specifications:**
- `folder`: Folder name (empty if no folder)
- `favorite`: Boolean (0 or 1)
- `type`: Cipher type (login=1, note=2, card=3, identity=4)
- `name`: Item name (required)
- `notes`: Item notes (optional, multi-line escaped)
- `fields`: Custom fields (JSON array as string)
- `reprompt`: Master password reprompt (0 or 1)
- `login_uri`: Login URL (login type only)
- `login_username`: Username (login type only)
- `login_password`: Password (login type only)
- `login_totp`: TOTP secret (login type only)

**Special Handling:**
- Multi-line notes: Quoted with escaped newlines
- Commas in fields: Entire field quoted
- Quotes in fields: Escaped with double quotes (`""`)
- UTF-8 encoding required
- CSV header row required

**Example:**
```csv
folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp
Work,1,1,GitHub,"Work GitHub account",,0,https://github.com,user@example.com,password123,TOTP_SECRET
,0,2,Note,"Simple note",,0,,,,,
```

**Compatibility Notes:**
- Must match TypeScript CLI output exactly
- Excel/Google Sheets compatible
- Can be imported by many password managers

### 2. JSON Format (Bitwarden)

**Purpose:** Structured format preserving all Bitwarden vault data and metadata.

**Structure:**
```json
{
  "encrypted": false,
  "folders": [
    {
      "id": "uuid",
      "name": "folder-name"
    }
  ],
  "items": [
    {
      "id": "uuid",
      "organizationId": "uuid-or-null",
      "folderId": "uuid-or-null",
      "type": 1,
      "reprompt": 0,
      "name": "item-name",
      "notes": "item-notes",
      "favorite": false,
      "fields": [],
      "login": {
        "uris": [{"match": null, "uri": "https://example.com"}],
        "username": "user@example.com",
        "password": "password123",
        "totp": "TOTP_SECRET"
      },
      "collectionIds": []
    }
  ]
}
```

**Field Specifications:**

**Top-level:**
- `encrypted`: Boolean, false for unencrypted export
- `folders`: Array of folder objects
- `items`: Array of cipher objects

**Folder Object:**
- `id`: UUID string
- `name`: Folder name string

**Cipher Object:**
- `id`: UUID string (required)
- `organizationId`: UUID string or null
- `folderId`: UUID string or null
- `type`: Integer (1=login, 2=note, 3=card, 4=identity)
- `reprompt`: Integer (0=disabled, 1=enabled)
- `name`: String (required)
- `notes`: String or null
- `favorite`: Boolean
- `fields`: Array of custom field objects
- `login`/`secureNote`/`card`/`identity`: Type-specific data object
- `collectionIds`: Array of UUID strings

**Type-Specific Objects:**

**Login:**
```json
{
  "uris": [{"match": null, "uri": "string"}],
  "username": "string-or-null",
  "password": "string-or-null",
  "totp": "string-or-null"
}
```

**Secure Note:**
```json
{
  "type": 0
}
```

**Card:**
```json
{
  "cardholderName": "string-or-null",
  "brand": "string-or-null",
  "number": "string-or-null",
  "expMonth": "string-or-null",
  "expYear": "string-or-null",
  "code": "string-or-null"
}
```

**Identity:**
```json
{
  "title": "string-or-null",
  "firstName": "string-or-null",
  "middleName": "string-or-null",
  "lastName": "string-or-null",
  "address1": "string-or-null",
  "address2": "string-or-null",
  "address3": "string-or-null",
  "city": "string-or-null",
  "state": "string-or-null",
  "postalCode": "string-or-null",
  "country": "string-or-null",
  "company": "string-or-null",
  "email": "string-or-null",
  "phone": "string-or-null",
  "ssn": "string-or-null",
  "username": "string-or-null",
  "passportNumber": "string-or-null",
  "licenseNumber": "string-or-null"
}
```

**Custom Field Object:**
```json
{
  "name": "field-name",
  "value": "field-value",
  "type": 0
}
```
- `type`: 0=text, 1=hidden, 2=boolean

**Compatibility Notes:**
- Must match TypeScript CLI JSON structure exactly
- Pretty-print with 2-space indentation (when --pretty flag used)
- Minified JSON (no whitespace) by default
- UTF-8 encoding required

### 3. Encrypted JSON Format (Bitwarden)

**Purpose:** Secure export format protected with user password.

**Structure:**
```json
{
  "encrypted": true,
  "encKeyValidation_DO_NOT_EDIT": "base64-string",
  "data": "encrypted-base64-string"
}
```

**Field Specifications:**
- `encrypted`: Boolean, always true
- `encKeyValidation_DO_NOT_EDIT`: Encryption key validation string
- `data`: Base64-encoded encrypted JSON (same structure as unencrypted JSON)

**Encryption Process:**
1. User provides password
2. Derive encryption key from password using PBKDF2 or Argon2
3. Encrypt JSON data with AES-256-CBC or AES-256-GCM
4. Include KDF parameters in encrypted data
5. Base64-encode encrypted data

**Decryption Process:**
1. User provides password
2. Extract KDF parameters from encrypted data
3. Derive decryption key using same KDF
4. Decrypt data
5. Parse decrypted JSON

**Security Requirements:**
- Use strong KDF (PBKDF2 with 100,000+ iterations or Argon2)
- Use AES-256 for encryption
- Include random salt in KDF
- Include random IV in encryption
- Validate encryption key before full decryption (encKeyValidation)

**Compatibility Notes:**
- Must use same encryption scheme as TypeScript CLI
- Must be compatible with Bitwarden SDK encryption
- Encryption parameters must be included in output

## Import Formats

### 1. Bitwarden CSV

**Detection:** Header row matches Bitwarden CSV format exactly.

**Required Headers:** At minimum `type` and `name` columns.

**Transformation:**
- Map CSV columns to cipher fields
- Parse type as integer
- Create folders from unique folder names
- Parse custom fields from JSON string
- Handle multi-line notes

**Validation:**
- Require `name` field
- Require `type` field (1, 2, 3, or 4)
- Validate login_uri is valid URL (if present)
- Validate favorite/reprompt are 0 or 1
- Validate UTF-8 encoding

### 2. Bitwarden JSON

**Detection:** JSON object with `items` array.

**Required Fields:**
- `items` array must exist
- Each item must have `type` and `name`

**Transformation:**
- Parse JSON into cipher objects
- Validate UUIDs
- Create folders from `folders` array
- Map item fields directly (already in Bitwarden format)

**Validation:**
- Validate JSON syntax
- Validate required fields present
- Validate types are correct (1-4)
- Validate UUIDs are valid format
- Validate type-specific data matches type

### 3. Bitwarden Encrypted JSON

**Detection:** JSON object with `"encrypted": true`.

**Required Fields:**
- `encrypted` must be true
- `data` must be present
- `encKeyValidation_DO_NOT_EDIT` must be present

**Decryption:**
- Prompt user for password (if not provided)
- Decrypt data using password
- Validate decryption succeeded
- Parse decrypted JSON
- Process as regular Bitwarden JSON import

**Validation:**
- Validate encryption key with encKeyValidation
- Validate decrypted data is valid JSON
- Validate decrypted data has correct structure

### 4. LastPass CSV

**Detection:** Header row contains `url`, `username`, `password`.

**Header Format:**
```csv
url,username,password,extra,name,grouping,fav
```

**Transformation:**
- `name` → cipher name
- `url` → login URI
- `username` → login username
- `password` → login password
- `extra` → notes
- `grouping` → folder name
- `fav` → favorite (convert to boolean)

**Special Handling:**
- LastPass uses `grouping` for folders (can include subfolders with `\` separator)
- Handle subfolder hierarchy (create nested folders or flatten)
- `extra` field may contain multi-line notes
- Handle special characters in passwords

**Validation:**
- Require `name` field
- Validate `url` is valid URL (if not empty)
- Handle empty fields gracefully

### 5. 1Password 1PIF Format

**Detection:** File extension `.1pif` or JSON array of 1Password items.

**Structure:**
```json
[
  {
    "uuid": "uuid",
    "title": "item-name",
    "securityLevel": "SL5",
    "typeName": "webforms.WebForm",
    "secureContents": {
      "fields": [],
      "URLs": [],
      "notesPlain": "notes"
    },
    "openContents": {}
  }
]
```

**Transformation:**
- `title` → cipher name
- `typeName` → cipher type (map to Bitwarden types)
- `secureContents.fields` → login username/password
- `secureContents.URLs` → login URIs
- `secureContents.notesPlain` → notes
- `folderUuid` → folder (if present)

**Type Mapping:**
- `webforms.WebForm` → login (type 1)
- `passwords.Password` → login (type 1)
- `securenotes.SecureNote` → note (type 2)
- `wallet.financial.CreditCard` → card (type 3)
- `identities.Identity` → identity (type 4)

**Validation:**
- Validate JSON structure
- Handle missing fields gracefully
- Validate type mapping is supported

### 6. Chrome Passwords CSV

**Detection:** Header row matches Chrome format.

**Header Format:**
```csv
name,url,username,password
```

**Transformation:**
- `name` → cipher name
- `url` → login URI
- `username` → login username
- `password` → login password
- All imports are type 1 (login)

**Special Handling:**
- Chrome doesn't export folders (all items at root level)
- Chrome doesn't export notes or custom fields
- Simple 1:1 mapping

**Validation:**
- Require all four columns
- Validate `url` is valid URL
- Require `name` field

### 7. KeePass CSV

**Detection:** Header row contains `Account`, `Login Name`, `Password`, `Web Site`.

**Header Format:**
```csv
"Account","Login Name","Password","Web Site","Comments","Group"
```

**Transformation:**
- `Account` → cipher name
- `Login Name` → login username
- `Password` → login password
- `Web Site` → login URI
- `Comments` → notes
- `Group` → folder name

**Special Handling:**
- KeePass uses `Group` for folders (can be hierarchical with `/` separator)
- Handle folder hierarchy
- Handle special characters in fields

**Validation:**
- Require `Account` field
- Handle missing fields gracefully

### 8. Dashlane CSV

**Detection:** Header row contains `title`, `username`, `password`, `url`.

**Header Format:**
```csv
title,username,password,url,note,category
```

**Transformation:**
- `title` → cipher name
- `username` → login username
- `password` → login password
- `url` → login URI
- `note` → notes
- `category` → folder name

**Special Handling:**
- Dashlane categories map to folders
- Handle multi-line notes
- Handle special characters

**Validation:**
- Require `title` field
- Validate `url` is valid URL (if present)

## Format Detection Algorithm

### Detection Order

1. **Check file extension** (if provided)
   - `.json` → JSON-based format (Bitwarden or 1Password)
   - `.csv` → CSV-based format (multiple possibilities)
   - `.1pif` → 1Password format

2. **Parse file format**
   - Try parsing as JSON
   - Try parsing as CSV
   - Fail if neither works

3. **Analyze structure** (JSON)
   - Check for `"encrypted": true` → Bitwarden Encrypted JSON
   - Check for `items` array → Bitwarden JSON
   - Check for array of objects with `uuid` and `typeName` → 1Password 1PIF

4. **Analyze headers** (CSV)
   - Check for exact Bitwarden header → Bitwarden CSV
   - Check for LastPass header (url, username, password, extra) → LastPass
   - Check for 1Password header → 1Password CSV
   - Check for Chrome header (name, url, username, password) → Chrome
   - Check for KeePass header (Account, Login Name, Password) → KeePass
   - Check for Dashlane header (title, username, password, url) → Dashlane

5. **Explicit format parameter** (overrides detection)
   - If user provides `--format` flag, use that format
   - Skip detection entirely

### Detection Edge Cases

- **Ambiguous formats:** If detection is ambiguous, fail with error asking user to specify format
- **Invalid format:** If format cannot be detected, list supported formats and ask user to specify
- **Empty files:** Fail with clear error message
- **Malformed files:** Fail with clear error indicating parsing failure

## Validation Rules

### Common Validation Rules

1. **File Existence:** File must exist and be readable
2. **File Size:** Warn if file is very large (> 10MB)
3. **Encoding:** File must be valid UTF-8
4. **Structure:** File must match format specification
5. **Required Fields:** All required fields must be present for each format

### Cipher-Specific Validation

**Login Items (type 1):**
- At least one of username, password, or URI must be present
- URI must be valid URL format (if present)
- TOTP must be valid base32 (if present)

**Note Items (type 2):**
- Name is required
- Notes field optional but typically used

**Card Items (type 3):**
- At least card number or cardholder name should be present
- Expiration format validation (MM/YYYY)
- Card number format validation (numeric, 13-19 digits)

**Identity Items (type 4):**
- At least one identity field should be present
- Email validation (if present)
- Phone number format validation (if present)

### Data Integrity Validation

1. **Encoding:** All text must be valid UTF-8
2. **Length Limits:** Fields must not exceed reasonable lengths (e.g., name < 1000 chars)
3. **Type Safety:** Numeric fields must be numeric, booleans must be boolean
4. **Reference Integrity:** Folder IDs must reference existing folders
5. **Format Compliance:** Data must match schema for format

### Security Validation

1. **No Malicious Content:** Check for potential injection attacks
2. **Password Validation:** Warn if importing weak passwords (optional)
3. **Encryption Validation:** For encrypted imports, validate encryption is correct
4. **File Path Validation:** Ensure output paths are safe (no directory traversal)

## Error Handling Patterns

### Validation Errors

**Format:** `Validation error at line {line}: {error message}`

**Examples:**
- `Validation error at line 15: Missing required field 'name'`
- `Validation error at line 23: Invalid type value '5' (must be 1-4)`
- `Validation error: Invalid JSON syntax at position 1234`

### Import Errors

**Format:** `Import failed: {error message}`

**Examples:**
- `Import failed: Could not authenticate with API`
- `Import failed: Folder creation failed for 'Work'`
- `Import failed: Item creation failed at line 42`

### Export Errors

**Format:** `Export failed: {error message}`

**Examples:**
- `Export failed: Not authenticated`
- `Export failed: Could not write to file '/path/to/file.csv'`
- `Export failed: Organization 'uuid' not found`

## Performance Considerations

### Export Performance

1. **Batch Reading:** Read vault data in batches (100-1000 items)
2. **Streaming Output:** Write to file incrementally, not all at once
3. **Memory Management:** Don't hold entire vault in memory
4. **Progress Tracking:** Update progress every 50-100 items

### Import Performance

1. **Batch Parsing:** Parse import file in chunks
2. **Batch API Calls:** Create items in batches (if API supports)
3. **Parallel Processing:** Process multiple items concurrently (if safe)
4. **Progress Tracking:** Update progress every 50-100 items

### Memory Optimization

1. **Streaming:** Use streaming parsers for large files
2. **Incremental Processing:** Process data incrementally
3. **Resource Cleanup:** Clear sensitive data from memory after use
4. **Buffering:** Use appropriate buffer sizes (4KB-64KB)

## Testing Recommendations

### Unit Tests

1. **Parser Tests:** Test each format parser with sample data
2. **Validation Tests:** Test all validation rules
3. **Transformation Tests:** Test data transformation logic
4. **Error Handling Tests:** Test all error paths

### Integration Tests

1. **Round-Trip Tests:** Export then import, verify data integrity
2. **Cross-Compatibility Tests:** Import TypeScript CLI exports
3. **Format Detection Tests:** Test auto-detection for each format
4. **Large Dataset Tests:** Test with 1,000+ items

### Sample Data Requirements

Create sample data files for each format:
- Bitwarden CSV (10-item sample)
- Bitwarden JSON (10-item sample)
- Bitwarden Encrypted JSON (10-item sample)
- LastPass CSV (10-item sample)
- 1Password 1PIF (10-item sample)
- Chrome CSV (10-item sample)
- KeePass CSV (10-item sample)
- Dashlane CSV (10-item sample)

Each sample should include:
- Multiple item types (login, note, card, identity)
- Special characters (unicode, quotes, commas)
- Multi-line notes
- Empty/null fields
- Folders/groups

## References

- TypeScript CLI: `apps/cli/src/tools/import/importers/`
- CSV RFC: https://tools.ietf.org/html/rfc4180
- JSON RFC: https://tools.ietf.org/html/rfc8259
- Bitwarden Export Format: https://bitwarden.com/help/export-your-data/
- LastPass Export: https://support.lastpass.com/help/how-do-i-export-stored-data-from-lastpass
- 1Password Export: https://support.1password.com/export/
- Chrome Export: chrome://settings/passwords (Export button)
- KeePass Export: Database → Export menu
- Dashlane Export: File → Export menu

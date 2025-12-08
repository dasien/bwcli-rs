# Import/Export Format Specifications

This document provides detailed format specifications for all supported import and export formats, including field mappings, parsing rules, and example data.

## Table of Contents

1. [Bitwarden CSV Format](#bitwarden-csv-format)
2. [Bitwarden JSON Format](#bitwarden-json-format)
3. [Encrypted JSON Format](#encrypted-json-format)
4. [LastPass CSV Format](#lastpass-csv-format)
5. [1Password CSV Format](#1password-csv-format)
6. [Chrome Passwords CSV Format](#chrome-passwords-csv-format)

---

## Bitwarden CSV Format

### Specification

**File Extension:** `.csv`

**Character Encoding:** UTF-8

**Field Separator:** `,` (comma)

**Quote Character:** `"` (double quote)

**Line Terminator:** `\n` (LF) or `\r\n` (CRLF)

### Header Row

```csv
folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp,card_cardholderName,card_brand,card_number,card_expMonth,card_expYear,card_code,identity_title,identity_firstName,identity_middleName,identity_lastName,identity_address1,identity_address2,identity_address3,identity_city,identity_state,identity_postalCode,identity_country,identity_company,identity_email,identity_phone,identity_ssn,identity_username,identity_passportNumber,identity_licenseNumber
```

### Field Definitions

| Field | Type | Required | Description | Example |
|-------|------|----------|-------------|---------|
| `folder` | string | No | Folder name | "Personal" |
| `favorite` | integer | No | 1 = favorite, 0 = not favorite | 1 |
| `type` | string | Yes | Item type: login, note, card, identity | "login" |
| `name` | string | Yes | Item name | "Example Site" |
| `notes` | string | No | Notes (can contain newlines) | "My secure notes" |
| `fields` | string | No | Custom fields, format: "name: value" separated by newlines | "API Key: abc123" |
| `reprompt` | integer | No | 1 = reprompt, 0 = no reprompt | 0 |
| `login_uri` | string | No | Login URI(s), multiple URIs separated by newlines | "https://example.com" |
| `login_username` | string | No | Login username | "user@example.com" |
| `login_password` | string | No | Login password | "password123" |
| `login_totp` | string | No | TOTP secret | "otpauth://totp/..." |

**Card Fields (type=card):**

| Field | Description | Example |
|-------|-------------|---------|
| `card_cardholderName` | Cardholder name | "John Doe" |
| `card_brand` | Card brand | "Visa" |
| `card_number` | Card number | "4111111111111111" |
| `card_expMonth` | Expiration month | "12" |
| `card_expYear` | Expiration year | "2025" |
| `card_code` | CVV/CVC code | "123" |

**Identity Fields (type=identity):**

| Field | Description |
|-------|-------------|
| `identity_title` | Title (Mr., Ms., etc.) |
| `identity_firstName` | First name |
| `identity_middleName` | Middle name |
| `identity_lastName` | Last name |
| `identity_address1` | Address line 1 |
| `identity_address2` | Address line 2 |
| `identity_address3` | Address line 3 |
| `identity_city` | City |
| `identity_state` | State/Province |
| `identity_postalCode` | Postal code |
| `identity_country` | Country |
| `identity_company` | Company |
| `identity_email` | Email |
| `identity_phone` | Phone |
| `identity_ssn` | SSN |
| `identity_username` | Username |
| `identity_passportNumber` | Passport number |
| `identity_licenseNumber` | License number |

### Example Data

```csv
folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp
"Personal",1,"login","GitHub","My work account","API Token: ghp_abc123",0,"https://github.com","user@example.com","password123",""
"Work",0,"login","AWS Console","Production account","",0,"https://console.aws.amazon.com","admin","SecurePass!23",""
"",0,"note","Secure Note","This is a secure note
with multiple lines","",0,"","","",""
"Personal",1,"card","Visa Card","Primary card","",0,"","","","","John Doe","Visa","4111111111111111","12","2025","123"
```

### Parsing Rules

1. **Quotes:** Fields containing commas, newlines, or quotes must be quoted
2. **Escape Quotes:** Quotes within quoted fields are escaped by doubling (`""`)
3. **Empty Fields:** Empty fields are represented as `""` or nothing between commas
4. **Newlines:** Newlines within `notes` and `fields` are preserved within quotes
5. **Multiple URIs:** Multiple URIs in `login_uri` are separated by actual newlines within quotes

### Implementation Notes

- Use `csv` crate with custom configuration:
  ```rust
  let mut csv_reader = csv::ReaderBuilder::new()
      .has_headers(true)
      .flexible(false)  // Strict: all rows must have same number of fields
      .from_reader(reader);
  ```

- Handle newlines in fields:
  ```rust
  // Multiple URIs example
  let uris: Vec<String> = login_uri
      .lines()
      .filter(|s| !s.is_empty())
      .map(|s| s.to_string())
      .collect();
  ```

---

## Bitwarden JSON Format

### Specification

**File Extension:** `.json`

**Character Encoding:** UTF-8

**Format:** JSON (pretty-printed with 2-space indent)

### Root Structure

```json
{
  "encrypted": false,
  "folders": [...],
  "items": [...]
}
```

### Folder Object

```json
{
  "id": "uuid-string",
  "name": "Folder Name"
}
```

### Item Object (Login)

```json
{
  "id": "uuid-string",
  "organizationId": null,
  "folderId": "folder-uuid",
  "type": 1,
  "reprompt": 0,
  "name": "Item Name",
  "notes": "Item notes",
  "favorite": true,
  "fields": [
    {
      "name": "Field Name",
      "value": "Field Value",
      "type": 0
    }
  ],
  "login": {
    "username": "user@example.com",
    "password": "password123",
    "totp": "otpauth://totp/...",
    "uris": [
      {
        "match": null,
        "uri": "https://example.com"
      }
    ]
  },
  "collectionIds": [],
  "revisionDate": "2025-12-05T00:00:00.000Z",
  "creationDate": "2025-12-05T00:00:00.000Z",
  "deletedDate": null
}
```

### Field Type Mapping

| Field | JSON Key | Type | Values |
|-------|----------|------|--------|
| Item Type | `type` | integer | 1=Login, 2=SecureNote, 3=Card, 4=Identity |
| Reprompt | `reprompt` | integer | 0=No, 1=Yes |
| Field Type | `fields[].type` | integer | 0=Text, 1=Hidden, 2=Boolean |
| URI Match | `login.uris[].match` | integer? | null=Default, 0=Domain, 1=Host, 2=StartsWith, 3=Exact, 4=RegEx, 5=Never |

### Complete Example

```json
{
  "encrypted": false,
  "folders": [
    {
      "id": "4c2869dd-0e1c-499f-b116-a824016df251",
      "name": "Personal"
    },
    {
      "id": "7e4bb8a1-3c5d-4e7f-9c1a-f4ab8e7c2b61",
      "name": "Work"
    }
  ],
  "items": [
    {
      "id": "bf22e4b4-1e3a-4c7c-8e5f-2f8a7d9e4a12",
      "organizationId": null,
      "folderId": "4c2869dd-0e1c-499f-b116-a824016df251",
      "type": 1,
      "reprompt": 0,
      "name": "GitHub",
      "notes": "My work account",
      "favorite": true,
      "fields": [
        {
          "name": "API Token",
          "value": "ghp_abc123xyz",
          "type": 1
        }
      ],
      "login": {
        "username": "user@example.com",
        "password": "password123",
        "totp": "otpauth://totp/GitHub:user@example.com?secret=ABC123&issuer=GitHub",
        "uris": [
          {
            "match": null,
            "uri": "https://github.com"
          }
        ]
      },
      "collectionIds": [],
      "revisionDate": "2025-12-05T12:30:45.123Z",
      "creationDate": "2025-01-15T08:00:00.000Z",
      "deletedDate": null
    },
    {
      "id": "a3f5e7c9-2d4b-4e8f-9a1c-7f6e5d4c3b2a",
      "organizationId": null,
      "folderId": "4c2869dd-0e1c-499f-b116-a824016df251",
      "type": 2,
      "reprompt": 0,
      "name": "Secure Note",
      "notes": "This is a secure note with sensitive information.\nIt can span multiple lines.",
      "favorite": false,
      "fields": [],
      "secureNote": {
        "type": 0
      },
      "collectionIds": [],
      "revisionDate": "2025-12-05T12:30:45.123Z",
      "creationDate": "2025-01-15T08:00:00.000Z",
      "deletedDate": null
    },
    {
      "id": "c8e2f4a6-1d3b-4e5f-8c7a-9f2e1d4c3b5a",
      "organizationId": null,
      "folderId": "4c2869dd-0e1c-499f-b116-a824016df251",
      "type": 3,
      "reprompt": 1,
      "name": "Visa Card",
      "notes": "Primary credit card",
      "favorite": true,
      "fields": [],
      "card": {
        "cardholderName": "John Doe",
        "brand": "Visa",
        "number": "4111111111111111",
        "expMonth": "12",
        "expYear": "2025",
        "code": "123"
      },
      "collectionIds": [],
      "revisionDate": "2025-12-05T12:30:45.123Z",
      "creationDate": "2025-01-15T08:00:00.000Z",
      "deletedDate": null
    }
  ]
}
```

### Parsing Rules

1. **UUIDs:** All IDs must be valid UUIDs (or generated during import)
2. **Dates:** All dates in ISO 8601 format with milliseconds
3. **Optional Fields:** Missing optional fields should be treated as null/empty
4. **Type Validation:** Ensure `type` field matches the presence of `login`/`card`/`identity`/`secureNote`

### Implementation Notes

```rust
// Deserialize with serde_json
#[derive(Deserialize)]
struct BitwardenJsonExport {
    encrypted: bool,
    folders: Vec<FolderExport>,
    items: Vec<ItemExport>,
}

// Validate structure
if !export.encrypted {
    // Process as unencrypted
} else {
    return Err(ImportError::ParseError(
        "Use encrypted_json format for encrypted exports".to_string()
    ));
}
```

---

## Encrypted JSON Format

### Specification

**File Extension:** `.json`

**Character Encoding:** UTF-8

**Encryption:** AES-256-CBC

**KDF:** PBKDF2-SHA256 (100,000 iterations)

### Root Structure

```json
{
  "encrypted": true,
  "encKeyValidation_DO_NOT_EDIT": "2.iv|ciphertext|mac",
  "data": "2.iv|ciphertext|mac"
}
```

### Field Definitions

| Field | Description |
|-------|-------------|
| `encrypted` | Always `true` |
| `encKeyValidation_DO_NOT_EDIT` | Encrypted known value for password verification |
| `data` | Encrypted vault data (same structure as Bitwarden JSON) |

### EncString Format

Bitwarden EncString format: `type.iv|ciphertext|mac`

Where:
- `type`: Encryption type (2 = AES-256-CBC with HMAC-SHA256)
- `iv`: Base64-encoded initialization vector (16 bytes)
- `ciphertext`: Base64-encoded encrypted data
- `mac`: Base64-encoded HMAC-SHA256 (32 bytes)

### Example

```json
{
  "encrypted": true,
  "encKeyValidation_DO_NOT_EDIT": "2.dGVzdGluZ3RoaXNvdXQ=|8J+YgOKAnOKAjeKAjeKAjQ==|dGhpc2lzYXRlc3RtYWN0aGlzaXNhdGVzdG1hY3Rlc3RtYWM=",
  "data": "2.aXZpc3JhbmRvbTE2Ynl0ZXM=|eyJlbmNyeXB0ZWQiOmZhbHNlLCJmb2xkZXJzIjpbXSwiaXRlbXMiOltdfQ==|bWFjaXNyYW5kb20zMmJ5dGVzbWFjaXNyYW5kb20zMmJ5dGVz"
}
```

### Encryption Process

1. **Serialize Data:** Convert vault to JSON (Bitwarden JSON format without `encrypted` field)
2. **Derive Key:** Use PBKDF2-SHA256 with user password
   - Salt: Random 16 bytes (stored in derived data structure)
   - Iterations: 100,000
   - Output: 32 bytes (256 bits)
3. **Generate IV:** Random 16 bytes
4. **Encrypt:** AES-256-CBC
5. **Compute MAC:** HMAC-SHA256 of (IV + ciphertext)
6. **Format:** Create EncString: `2.base64(iv)|base64(ciphertext)|base64(mac)`
7. **Validation:** Encrypt known string with same key for password verification

### Decryption Process

1. **Parse JSON:** Extract `encKeyValidation_DO_NOT_EDIT` and `data`
2. **Prompt Password:** Get password from user (or `--password` flag)
3. **Derive Key:** Use same PBKDF2 parameters
4. **Verify Password:** Decrypt `encKeyValidation_DO_NOT_EDIT` and check known value
5. **Parse EncString:** Extract type, IV, ciphertext, MAC from `data`
6. **Verify MAC:** Compute HMAC-SHA256 and compare
7. **Decrypt:** AES-256-CBC with derived key and IV
8. **Parse JSON:** Parse decrypted data as Bitwarden JSON

### Implementation Notes

```rust
// Use Bitwarden SDK for all crypto operations
use bitwarden_crypto::{derive_key, encrypt_aes256, decrypt_aes256, EncString};

// Derive key from password
let kdf = Kdf::Pbkdf2 {
    iterations: 100_000,
};
let key = derive_key(&password, &email, &kdf)?;

// Encrypt data
let enc_string = encrypt_aes256(&json_data, &key)?;

// Format as EncString
let enc_string_formatted = format!("2.{}|{}|{}",
    base64::encode(&iv),
    base64::encode(&ciphertext),
    base64::encode(&mac)
);
```

---

## LastPass CSV Format

### Specification

**File Extension:** `.csv`

**Character Encoding:** UTF-8

**Source:** LastPass CSV export

### Header Row

```csv
url,username,password,extra,name,grouping,fav
```

### Field Definitions

| Field | Description | Maps To |
|-------|-------------|---------|
| `url` | Website URL | `login.uris[0].uri` |
| `username` | Login username | `login.username` |
| `password` | Login password | `login.password` |
| `extra` | Notes | `notes` |
| `name` | Item name | `name` |
| `grouping` | Folder/category | Folder name |
| `fav` | Favorite (1 or 0) | `favorite` |

### Example Data

```csv
url,username,password,extra,name,grouping,fav
https://github.com,user@example.com,password123,My work account,GitHub,Work,1
https://gmail.com,myemail@gmail.com,emailpass,Personal email,Gmail,Personal,0
http://sn,,,This is a secure note,Secure Note,Personal,0
```

### Special Cases

**Secure Notes:**
- URL is `http://sn`
- Username and password are empty
- Content is in `extra` field
- Type becomes `SecureNote` instead of `Login`

### Parsing Rules

1. **Type Detection:** If URL is `http://sn`, treat as secure note
2. **Folder Creation:** Create folder from `grouping` field if not empty
3. **Favorite:** Convert "1" to `true`, anything else to `false`
4. **Empty Fields:** Treat empty strings as `None`

### Implementation Notes

```rust
// Detect secure note
if record.url == "http://sn" {
    return ImportItem {
        item_type: ImportItemType::SecureNote,
        name: record.name,
        notes: Some(record.extra),
        // ... other fields
    };
}

// Regular login
ImportItem {
    item_type: ImportItemType::Login,
    name: record.name,
    notes: if record.extra.is_empty() { None } else { Some(record.extra) },
    folder_name: if record.grouping.is_empty() { None } else { Some(record.grouping) },
    favorite: record.fav == "1",
    login: Some(ImportLogin {
        username: if record.username.is_empty() { None } else { Some(record.username) },
        password: if record.password.is_empty() { None } else { Some(record.password) },
        uris: vec![record.url],
        totp: None,
    }),
    // ... other fields
}
```

---

## 1Password CSV Format

### Specification

**File Extension:** `.csv`

**Character Encoding:** UTF-8

**Source:** 1Password CSV export

### Header Row

```csv
Title,Website,Username,Password,Notes,Type,Folder
```

### Field Definitions

| Field | Description | Maps To |
|-------|-------------|---------|
| `Title` | Item title | `name` |
| `Website` | Website URL | `login.uris[0].uri` |
| `Username` | Login username | `login.username` |
| `Password` | Login password | `login.password` |
| `Notes` | Notes | `notes` |
| `Type` | Item type | Cipher type |
| `Folder` | Folder name | Folder name |

### Type Mapping

| 1Password Type | Bitwarden Type |
|----------------|----------------|
| `Login` | Login (type=1) |
| `Secure Note` | SecureNote (type=2) |
| `Credit Card` | Card (type=3) |
| `Identity` | Identity (type=4) |
| (empty) | Login (default) |

### Example Data

```csv
Title,Website,Username,Password,Notes,Type,Folder
GitHub,https://github.com,user@example.com,password123,My work account,Login,Work
Gmail,https://gmail.com,myemail@gmail.com,emailpass,Personal email,Login,Personal
Important Info,,,This is a secure note,Secure Note,Personal
```

### Parsing Rules

1. **Type Mapping:** Map 1Password types to Bitwarden cipher types
2. **Default Type:** If type is empty, default to Login
3. **Empty URLs:** If website is empty and type is "Secure Note", treat as note
4. **Folder Creation:** Create folder from `Folder` field

### Implementation Notes

```rust
fn map_1password_type(type_str: &str) -> ImportItemType {
    match type_str.to_lowercase().as_str() {
        "login" | "" => ImportItemType::Login,
        "secure note" => ImportItemType::SecureNote,
        "credit card" => ImportItemType::Card,
        "identity" => ImportItemType::Identity,
        _ => ImportItemType::Login, // Default to login
    }
}
```

---

## Chrome Passwords CSV Format

### Specification

**File Extension:** `.csv`

**Character Encoding:** UTF-8

**Source:** Chrome password export (chrome://settings/passwords)

### Header Row

```csv
name,url,username,password
```

### Field Definitions

| Field | Description | Maps To |
|-------|-------------|---------|
| `name` | Site name | `name` |
| `url` | Website URL | `login.uris[0].uri` |
| `username` | Login username | `login.username` |
| `password` | Login password | `login.password` |

### Example Data

```csv
name,url,username,password
github.com,https://github.com/login,user@example.com,password123
gmail.com,https://accounts.google.com,myemail@gmail.com,emailpass
example.com,https://example.com,testuser,testpass
```

### Special Cases

- **All items are Logins:** Chrome only exports passwords, no notes or cards
- **No folders:** Chrome doesn't have folder concept
- **No favorites:** All items have `favorite = false`
- **No TOTP:** Chrome doesn't export TOTP

### Parsing Rules

1. **Type:** All items are type Login
2. **Folder:** All items have no folder (`folder_id = null`)
3. **Name:** Use `name` field as item name
4. **URI:** Use `url` field as login URI

### Implementation Notes

```rust
// Simple mapping - all fields present
ImportItem {
    item_type: ImportItemType::Login,
    name: record.name,
    notes: None,
    folder_name: None,
    favorite: false,
    login: Some(ImportLogin {
        username: Some(record.username),
        password: Some(record.password),
        uris: vec![record.url],
        totp: None,
    }),
    // ... other fields default
}
```

---

## Format Detection Logic

### Auto-Detection Algorithm

For Bitwarden formats only (user must specify format for other password managers):

```rust
pub fn detect_bitwarden_format(data: &[u8]) -> Option<String> {
    // 1. Try JSON parse
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data) {
        // Check for encrypted JSON
        if json.get("encrypted") == Some(&serde_json::Value::Bool(true)) {
            return Some("encrypted_json".to_string());
        }

        // Check for Bitwarden JSON structure
        if json.get("items").is_some() && json.get("folders").is_some() {
            return Some("bitwardenjson".to_string());
        }
    }

    // 2. Try CSV parse
    if let Ok(csv_str) = std::str::from_utf8(data) {
        let first_line = csv_str.lines().next()?;

        // Check for Bitwarden CSV header
        if first_line.contains("login_uri") && first_line.contains("login_username") {
            return Some("bitwardencsv".to_string());
        }
    }

    None
}
```

### Detection Priority

1. **Encrypted JSON** - Check for `"encrypted": true`
2. **Bitwarden JSON** - Check for `items` and `folders` keys
3. **Bitwarden CSV** - Check for `login_uri` in header

### Usage

```rust
// In import command
let format = if cmd.format.starts_with("bitwarden") {
    // Auto-detect
    let data = std::fs::read(&cmd.file)?;
    detect_bitwarden_format(&data)
        .unwrap_or_else(|| cmd.format.clone())
} else {
    // User specified non-Bitwarden format
    cmd.format.clone()
};
```

---

## Testing Recommendations

### Test Data Sets

Create comprehensive test files for each format:

1. **Small Dataset:** 5-10 items covering all field types
2. **Large Dataset:** 1000+ items for performance testing
3. **Edge Cases:** Special characters, unicode, empty fields, maximum lengths
4. **Real Exports:** Actual exports from each password manager

### Cross-Format Tests

Verify data integrity across formats:

```rust
#[test]
fn test_csv_json_equivalence() {
    // Same data exported as CSV and JSON should parse to same items
    let csv_items = parse_csv("test_export.csv");
    let json_items = parse_json("test_export.json");
    assert_eq!(csv_items, json_items);
}
```

### Validation Tests

Test validation catches errors:

```rust
#[test]
fn test_validator_catches_empty_name() {
    let invalid_item = ImportItem {
        name: "".to_string(),
        // ...
    };
    let result = validator.validate_item(&invalid_item);
    assert!(result.is_err());
}
```

---

## Reference Files

For implementation, reference these TypeScript CLI files:

- Export: `apps/cli/src/tools/commands/export.command.ts`
- Import: `apps/cli/src/tools/commands/import.command.ts`
- Importers: `apps/cli/src/tools/import/importers/`
  - `bitwarden-csv-importer.ts`
  - `bitwarden-json-importer.ts`
  - `lastpass-csv-importer.ts`
  - `onepassword-importer.ts`
  - `chrome-csv-importer.ts`

Ensure byte-for-byte compatibility with TypeScript CLI exports.

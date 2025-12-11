# Detailed User Stories: Vault Create/Edit CLI Commands

This document provides detailed user stories with comprehensive acceptance criteria for each command in the vault create/edit enhancement.

## US-1: Create Login Item

**Story:**
As a CLI user, I want to create a login item using JSON input, so that I can add new passwords to my vault programmatically.

**Acceptance Criteria:**

1. **Given** valid JSON with type=1 and login data
   **When** I run `bw create item <base64_json>`
   **Then** a new login item is created in my vault
   **And** the decrypted item JSON is returned

2. **Given** JSON missing the required `name` field
   **When** I run `bw create item <base64_json>`
   **Then** an error is returned: "name is required"

3. **Given** JSON with type=1 but missing `login` object
   **When** I run `bw create item <base64_json>`
   **Then** an error is returned: "login data required for Login type"

4. **Given** valid JSON with a `folderId` that doesn't exist
   **When** I run `bw create item <base64_json>`
   **Then** an error is returned: "Folder not found"

5. **Given** valid JSON
   **When** I run `bw create item <base64_json>` without a valid session
   **Then** an error is returned: "Vault is locked"

**Example Input:**
```json
{
  "type": 1,
  "name": "Example Login",
  "notes": null,
  "favorite": false,
  "login": {
    "username": "user@example.com",
    "password": "mypassword123",
    "uris": [
      {"uri": "https://example.com", "match": null}
    ],
    "totp": null
  }
}
```

---

## US-2: Create Secure Note Item

**Story:**
As a CLI user, I want to create a secure note using JSON input, so that I can store sensitive text securely.

**Acceptance Criteria:**

1. **Given** valid JSON with type=2 and secureNote data
   **When** I run `bw create item <base64_json>`
   **Then** a new secure note is created
   **And** the notes field contains my text

2. **Given** JSON with type=2 but missing `secureNote` object
   **When** I run `bw create item <base64_json>`
   **Then** an error is returned: "secureNote data required for SecureNote type"

**Example Input:**
```json
{
  "type": 2,
  "name": "My Secret Note",
  "notes": "This is the secret content of my note.",
  "secureNote": {"type": 0}
}
```

---

## US-3: Create Card Item

**Story:**
As a CLI user, I want to create a card item using JSON input, so that I can store credit card information securely.

**Acceptance Criteria:**

1. **Given** valid JSON with type=3 and card data
   **When** I run `bw create item <base64_json>`
   **Then** a new card item is created
   **And** card details are encrypted and stored

2. **Given** JSON with type=3 but missing `card` object
   **When** I run `bw create item <base64_json>`
   **Then** an error is returned: "card data required for Card type"

**Example Input:**
```json
{
  "type": 3,
  "name": "My Credit Card",
  "card": {
    "cardholderName": "John Doe",
    "brand": "Visa",
    "number": "4111111111111111",
    "expMonth": "12",
    "expYear": "2025",
    "code": "123"
  }
}
```

---

## US-4: Create Identity Item

**Story:**
As a CLI user, I want to create an identity item using JSON input, so that I can store personal information securely.

**Acceptance Criteria:**

1. **Given** valid JSON with type=4 and identity data
   **When** I run `bw create item <base64_json>`
   **Then** a new identity item is created

2. **Given** JSON with type=4 but missing `identity` object
   **When** I run `bw create item <base64_json>`
   **Then** an error is returned: "identity data required for Identity type"

**Example Input:**
```json
{
  "type": 4,
  "name": "Personal Identity",
  "identity": {
    "title": "Mr",
    "firstName": "John",
    "lastName": "Doe",
    "email": "john.doe@example.com",
    "phone": "555-1234"
  }
}
```

---

## US-5: Create Folder

**Story:**
As a CLI user, I want to create a folder using JSON input, so that I can organize my vault items.

**Acceptance Criteria:**

1. **Given** valid JSON with a `name` field
   **When** I run `bw create folder <base64_json>`
   **Then** a new folder is created
   **And** the folder JSON is returned with the new ID

2. **Given** JSON with an empty `name`
   **When** I run `bw create folder <base64_json>`
   **Then** an error is returned: "name cannot be empty"

3. **Given** JSON with `name` exceeding 1000 characters
   **When** I run `bw create folder <base64_json>`
   **Then** an error is returned: "name exceeds maximum length of 1000 characters"

**Example Input:**
```json
{
  "name": "Work Passwords"
}
```

---

## US-6: Edit Item

**Story:**
As a CLI user, I want to edit an existing vault item, so that I can update passwords and other details.

**Acceptance Criteria:**

1. **Given** a valid item ID and JSON with changes
   **When** I run `bw edit item <id> <base64_json>`
   **Then** the item is updated with the new values
   **And** unchanged fields are preserved
   **And** the updated decrypted item is returned

2. **Given** an item ID that doesn't exist
   **When** I run `bw edit item <id> <base64_json>`
   **Then** an error is returned: "Item not found"

3. **Given** an item that is in trash (deleted)
   **When** I run `bw edit item <id> <base64_json>`
   **Then** an error is returned: "Cannot edit deleted item"

4. **Given** JSON that only changes `password` field
   **When** I run `bw edit item <id> <base64_json>`
   **Then** only the password is updated
   **And** username, URIs, and other fields remain unchanged

**Merge Behavior:**
- Non-null fields in input overwrite existing values
- Null fields in input preserve existing values
- Arrays replace entirely (URIs, fields, etc.)

---

## US-7: Edit Folder

**Story:**
As a CLI user, I want to edit a folder name, so that I can reorganize my vault.

**Acceptance Criteria:**

1. **Given** a valid folder ID and JSON with new name
   **When** I run `bw edit folder <id> <base64_json>`
   **Then** the folder name is updated
   **And** items in the folder remain associated

2. **Given** a folder ID that doesn't exist
   **When** I run `bw edit folder <id> <base64_json>`
   **Then** an error is returned: "Folder not found"

---

## US-8: Delete Item (Soft)

**Story:**
As a CLI user, I want to move an item to trash, so that I can remove it while keeping the option to restore.

**Acceptance Criteria:**

1. **Given** a valid item ID
   **When** I run `bw delete item <id>`
   **Then** the item is moved to trash
   **And** `deletedDate` is set on the item
   **And** success message is returned

2. **Given** an item ID that doesn't exist
   **When** I run `bw delete item <id>`
   **Then** an error is returned: "Item not found"

3. **Given** an item already in trash
   **When** I run `bw delete item <id>`
   **Then** behavior TBD (verify with TypeScript CLI)

---

## US-9: Delete Item (Permanent)

**Story:**
As a CLI user, I want to permanently delete an item, so that sensitive data is completely removed.

**Acceptance Criteria:**

1. **Given** a valid item ID
   **When** I run `bw delete item <id> --permanent`
   **Then** the item is permanently deleted
   **And** item cannot be restored
   **And** success message is returned

2. **Given** the `--permanent` flag is not provided
   **When** I run `bw delete item <id>`
   **Then** item is only moved to trash (soft delete)

---

## US-10: Delete Folder

**Story:**
As a CLI user, I want to delete a folder, so that I can clean up my vault organization.

**Acceptance Criteria:**

1. **Given** a valid folder ID
   **When** I run `bw delete folder <id>`
   **Then** the folder is deleted
   **And** items previously in the folder become "unfiled"

2. **Given** a folder ID that doesn't exist
   **When** I run `bw delete folder <id>`
   **Then** an error is returned: "Folder not found"

---

## US-11: Restore Item from Trash

**Story:**
As a CLI user, I want to restore an item from trash, so that I can recover accidentally deleted items.

**Acceptance Criteria:**

1. **Given** an item ID for an item in trash
   **When** I run `bw restore item <id>`
   **Then** the item is restored
   **And** `deletedDate` is cleared
   **And** the restored item JSON is returned

2. **Given** an item ID for an item NOT in trash
   **When** I run `bw restore item <id>`
   **Then** an error is returned: "Item is not in trash"

3. **Given** an item ID that doesn't exist
   **When** I run `bw restore item <id>`
   **Then** an error is returned: "Item not found"

---

## US-12: Move Item to Folder

**Story:**
As a CLI user, I want to move an item to a different folder, so that I can organize my vault.

**Acceptance Criteria:**

1. **Given** a valid item ID and folder ID
   **When** I run `bw move <itemId> <folderId>`
   **Then** the item's `folderId` is updated
   **And** the updated item JSON is returned

2. **Given** `null` as the folder ID
   **When** I run `bw move <itemId> null`
   **Then** the item is removed from any folder
   **And** `folderId` becomes null

3. **Given** a folder ID that doesn't exist
   **When** I run `bw move <itemId> <folderId>`
   **Then** an error is returned: "Folder not found"

4. **Given** an item ID that doesn't exist
   **When** I run `bw move <itemId> <folderId>`
   **Then** an error is returned: "Item not found"

---

## US-13: Get Template - Login

**Story:**
As a CLI user, I want to get a JSON template for creating login items, so that I have a starting point.

**Acceptance Criteria:**

1. **Given** template type "item.login"
   **When** I run `bw get template item.login`
   **Then** a complete login template JSON is returned
   **And** template matches TypeScript CLI format exactly

**Expected Output:**
```json
{
  "organizationId": null,
  "collectionIds": null,
  "folderId": null,
  "type": 1,
  "name": "Item name",
  "notes": "Some notes about this item.",
  "favorite": false,
  "fields": [],
  "login": {
    "uris": [{"match": null, "uri": "https://example.com"}],
    "username": "jdoe",
    "password": "myp@ssword123",
    "totp": null
  },
  "reprompt": 0
}
```

---

## US-14: Get Template - Secure Note

**Story:**
As a CLI user, I want to get a JSON template for creating secure notes.

**Acceptance Criteria:**

1. **Given** template type "item.secureNote"
   **When** I run `bw get template item.secureNote`
   **Then** a complete secure note template JSON is returned

---

## US-15: Get Template - Card

**Story:**
As a CLI user, I want to get a JSON template for creating card items.

**Acceptance Criteria:**

1. **Given** template type "item.card"
   **When** I run `bw get template item.card`
   **Then** a complete card template JSON is returned

---

## US-16: Get Template - Identity

**Story:**
As a CLI user, I want to get a JSON template for creating identity items.

**Acceptance Criteria:**

1. **Given** template type "item.identity"
   **When** I run `bw get template item.identity`
   **Then** a complete identity template JSON is returned

---

## US-17: Get Template - Folder

**Story:**
As a CLI user, I want to get a JSON template for creating folders.

**Acceptance Criteria:**

1. **Given** template type "folder"
   **When** I run `bw get template folder`
   **Then** a folder template JSON is returned

**Expected Output:**
```json
{
  "name": "Folder name"
}
```

---

## US-18: Raw JSON Input

**Story:**
As a developer, I want to provide raw JSON without base64 encoding, for convenience during development.

**Acceptance Criteria:**

1. **Given** raw JSON starting with `{`
   **When** I run `bw create item '{"type":1,...}'`
   **Then** the JSON is parsed directly without base64 decoding

2. **Given** base64-encoded JSON (not starting with `{`)
   **When** I run `bw create item <base64>`
   **Then** the input is base64 decoded first

---

## US-19: Stdin Input

**Story:**
As a CLI user, I want to pipe JSON input from another command, for scripting convenience.

**Acceptance Criteria:**

1. **Given** JSON piped to stdin
   **When** I run `cat item.json | bw create item`
   **Then** the JSON is read from stdin
   **And** the item is created

2. **Given** base64-encoded JSON piped to stdin
   **When** I run `cat item.b64 | bw create item`
   **Then** the base64 is decoded and parsed

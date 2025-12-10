---
enhancement: 11-encrypt-decrypt
agent: requirements-analyst
document_type: user_stories
timestamp: 2025-12-09T12:00:00Z
---

# User Stories: Vault Encryption/Decryption

## Epic: Vault Decryption

**As a** CLI user
**I want** vault commands to return readable data instead of encrypted strings
**So that** I can actually use my vault data from the command line

---

## User Stories

### US-1: View Vault Items with Readable Names
**As a** CLI user
**I want** `bw list items` to show item names I can read
**So that** I can find the credential I'm looking for

**Acceptance Criteria**:
- [ ] Running `bw list items` shows human-readable item names (e.g., "GitHub", "Amazon")
- [ ] Item names are NOT in EncString format (e.g., "2.abc|def|ghi")
- [ ] Login usernames are decrypted in list output
- [ ] Notes preview (if shown) is decrypted

**Complexity**: Medium (3 story points)

---

### US-2: Retrieve Password for Scripting
**As a** CLI user
**I want** `bw get password <id>` to return my actual password
**So that** I can use it in scripts or copy it to clipboard

**Acceptance Criteria**:
- [ ] Running `bw get password <item-id>` returns the decrypted password
- [ ] With `--raw` flag, returns just the password without JSON wrapper
- [ ] Returns appropriate error if item not found
- [ ] Returns appropriate error if item has no password field

**Complexity**: Small (2 story points)

---

### US-3: Retrieve Username for Scripting
**As a** CLI user
**I want** `bw get username <id>` to return the actual username
**So that** I can use it in scripts or forms

**Acceptance Criteria**:
- [ ] Running `bw get username <item-id>` returns the decrypted username
- [ ] With `--raw` flag, returns just the username without JSON wrapper
- [ ] Returns appropriate error if item not found
- [ ] Returns appropriate error if item has no username field

**Complexity**: Small (2 story points)

---

### US-4: Generate TOTP Code
**As a** CLI user
**I want** `bw get totp <id>` to generate a TOTP code
**So that** I can complete two-factor authentication

**Acceptance Criteria**:
- [ ] Running `bw get totp <item-id>` returns a valid 6-digit code
- [ ] Code is generated from the decrypted TOTP secret
- [ ] With `--raw` flag, returns just the code without JSON wrapper
- [ ] Returns appropriate error if item has no TOTP secret

**Complexity**: Small (2 story points)

---

### US-5: View Complete Item Details
**As a** CLI user
**I want** `bw get item <id>` to show all decrypted fields
**So that** I can see the complete credential information

**Acceptance Criteria**:
- [ ] Running `bw get item <item-id>` returns full item JSON
- [ ] Item name is decrypted
- [ ] Login username/password are decrypted
- [ ] URIs are decrypted
- [ ] Notes are decrypted
- [ ] Custom fields are decrypted
- [ ] Card details are decrypted (for card items)
- [ ] Identity fields are decrypted (for identity items)

**Complexity**: Medium (3 story points)

---

### US-6: Session Persistence Across Commands
**As a** CLI user
**I want** to login once and have subsequent commands work
**So that** I don't have to enter my password for every command

**Acceptance Criteria**:
- [ ] After `bw login`, BW_SESSION environment variable is returned
- [ ] Exporting BW_SESSION allows subsequent commands to work
- [ ] Commands use session key to decrypt vault data
- [ ] Invalid/expired session key produces clear error message

**Complexity**: Medium (3 story points)

---

### US-7: Unlock After Lock
**As a** CLI user
**I want** `bw unlock` to restore decryption capability
**So that** I can use my vault again after locking it

**Acceptance Criteria**:
- [ ] After `bw lock`, running `bw unlock` with password works
- [ ] New BW_SESSION is returned
- [ ] Subsequent vault commands work with new session
- [ ] Wrong password produces clear error

**Complexity**: Small (2 story points)

---

### US-8: TypeScript CLI Compatibility
**As a** CLI user who uses both TypeScript and Rust CLI
**I want** sessions from one CLI to work with the other
**So that** I can migrate gradually or use either interchangeably

**Acceptance Criteria**:
- [ ] BW_SESSION from TypeScript CLI works with Rust CLI
- [ ] Storage format is compatible between CLIs
- [ ] `__PROTECTED__` storage keys match exactly

**Complexity**: Medium (3 story points)

---

### US-9: Secure Session Key Handling
**As a** security-conscious CLI user
**I want** my encryption keys handled securely
**So that** my vault remains protected

**Acceptance Criteria**:
- [ ] User key is encrypted at rest using session key
- [ ] User key never appears in logs or debug output
- [ ] Decrypted passwords are not cached unnecessarily
- [ ] Wrong session key returns error, not garbage data
- [ ] Error messages don't reveal cryptographic details

**Complexity**: Medium (3 story points)

---

## Story Dependencies

```
US-6 (Session Persistence)
    |
    +---> US-1 (List Items)
    |
    +---> US-2 (Get Password)
    |
    +---> US-3 (Get Username)
    |
    +---> US-4 (Get TOTP)
    |
    +---> US-5 (Get Item)
    |
    +---> US-7 (Unlock)

US-8 (Compatibility) - Independent, but critical
US-9 (Security) - Crosscutting concern for all stories
```

---

## Story Map

| Priority | User Story | Complexity |
|----------|------------|------------|
| P0 | US-6: Session Persistence | Medium |
| P0 | US-9: Security | Medium |
| P1 | US-1: List Items | Medium |
| P1 | US-2: Get Password | Small |
| P1 | US-3: Get Username | Small |
| P1 | US-4: Get TOTP | Small |
| P1 | US-5: Get Item | Medium |
| P2 | US-7: Unlock | Small |
| P2 | US-8: Compatibility | Medium |

---

## Total Effort Estimate

- Small stories (2 points): 4 stories = 8 points
- Medium stories (3 points): 5 stories = 15 points
- **Total**: 23 story points

Note: All stories are interdependent through the session/key management foundation (US-6, US-9).

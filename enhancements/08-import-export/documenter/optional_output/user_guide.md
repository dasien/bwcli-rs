# User Guide: Import/Export Commands

## Table of Contents

1. [Introduction](#introduction)
2. [Export Your Vault](#export-your-vault)
3. [Import Data](#import-data)
4. [Migration Guides](#migration-guides)
5. [Common Workflows](#common-workflows)
6. [Troubleshooting](#troubleshooting)
7. [Security Best Practices](#security-best-practices)

## Introduction

The Bitwarden CLI import/export functionality enables you to:

- **Export**: Create backups of your vault in various formats
- **Import**: Migrate data from other password managers or restore backups

**Status**: Service layer implementation complete. CLI commands pending integration.

## Export Your Vault

### Export Formats

#### CSV (Comma-Separated Values)

Best for: Data portability, spreadsheet analysis, simple backups

**Features**:
- Human-readable format
- Compatible with Excel, Google Sheets
- 34-column universal format
- Supports all cipher types (Login, Note, Card, Identity)

**Future CLI Command**:
```bash
bw export --output vault-backup.csv
```

**What Gets Exported**:
- All login items (username, password, URIs, TOTP)
- All secure notes
- All card items (card holder, number, expiration, CVV)
- All identity items (name, address, contact info)
- All folders
- Custom fields
- Favorite flags

**Not Included**:
- Attachments
- Organization membership (use --organizationid for org exports)

#### JSON (JavaScript Object Notation)

Best for: Complete backups, data inspection, programmatic access

**Features**:
- Structured format preserving all data
- Pretty-printed for readability
- Includes all metadata
- Compatible with Bitwarden JSON format

**Future CLI Command**:
```bash
bw export --format json --output vault-backup.json
```

**What Gets Exported**:
- Everything in CSV format, plus:
- Internal IDs
- Organization IDs
- Collection IDs
- Complete folder structure
- All cipher metadata

#### Encrypted JSON (Secure)

Best for: Secure backups with password protection

**Status**: ⚠️ Not yet functional (awaiting SDK integration)

**Features** (when available):
- AES-256-CBC encryption
- PBKDF2 key derivation
- Password-protected
- Secure storage

**Future CLI Command**:
```bash
bw export --format encrypted_json --password mypassword --output secure-backup.json
```

**Temporary Workaround**:
```bash
# Export as JSON and encrypt with GPG
bw export --format json | gpg --symmetric --armor > vault-backup.json.gpg

# Decrypt later
gpg --decrypt vault-backup.json.gpg | bw import bitwardenjson /dev/stdin
```

### Export Options

#### Output to File

**Future CLI Command**:
```bash
bw export --output /path/to/backup.csv
```

**Creates**: File at specified path

**Warning**: Will overwrite existing file (future: will prompt for confirmation)

#### Output to Stdout

**Future CLI Command**:
```bash
bw export --format csv
```

**Uses**:
- Pipe to other commands
- Process with text tools
- Stream to remote locations

**Examples**:
```bash
# Search exported data
bw export --format csv | grep "github"

# Count items
bw export --format csv | wc -l

# Upload to cloud storage
bw export --format csv | aws s3 cp - s3://my-bucket/backup.csv
```

### Export Examples

#### Basic CSV Export

**Future CLI Command**:
```bash
# Export entire vault to CSV
bw export --output vault-backup.csv

# Verify export
ls -lh vault-backup.csv
wc -l vault-backup.csv
```

#### JSON Export

**Future CLI Command**:
```bash
# Export to JSON for complete backup
bw export --format json --output vault-backup.json

# Pretty-print JSON for inspection
cat vault-backup.json | jq '.'
```

#### Encrypted Backup (Future)

**Future CLI Command**:
```bash
# Create password-protected backup
bw export --format encrypted_json --password "mySecurePassword" --output secure-backup.json

# Store password separately!
```

## Import Data

### Supported Formats

#### Bitwarden Formats

##### Bitwarden CSV

**Format ID**: `bitwardencsv`

**Source**: Exported from Bitwarden web vault or CLI

**Future CLI Command**:
```bash
bw import bitwardencsv backup.csv
```

**Features**:
- All cipher types supported
- Folder support
- Custom fields
- Multiple URIs per login

##### Bitwarden JSON

**Format ID**: `bitwardenjson`

**Source**: Exported from Bitwarden web vault or CLI

**Future CLI Command**:
```bash
bw import bitwardenjson backup.json
```

**Features**:
- Complete vault structure
- All metadata preserved
- Folder hierarchy
- Collection information

**Note**: Encrypted JSON must be decrypted before import

#### LastPass

**Format ID**: `lastpass`

**How to Export from LastPass**:

1. Log in to LastPass web vault
2. Go to "Account Settings" → "Advanced" → "Export"
3. Copy the CSV data or save as file
4. Save as `lastpass_export.csv`

**Future CLI Command**:
```bash
bw import lastpass lastpass_export.csv
```

**What Gets Imported**:
- All login items
- Groups → Folders
- Favorite flags
- Extra field → Notes

**Limitations**:
- Login items only (no cards or identities)
- No custom fields
- No TOTP secrets

#### 1Password

**Format ID**: `1password`

**How to Export from 1Password**:

1. Open 1Password
2. File → Export → CSV
3. Choose location and save
4. Save as `1password_export.csv`

**Future CLI Command**:
```bash
bw import 1password 1password_export.csv
```

**What Gets Imported**:
- Multiple item types (Login, Note, Card, etc.)
- Folder structure
- Notes field
- Type mapping

**Limitations**:
- CSV format has limited fields
- Some metadata may be lost

#### Chrome Passwords

**Format ID**: `chrome`

**How to Export from Chrome**:

1. Open Chrome
2. Go to chrome://settings/passwords
3. Click three dots (⋮) next to "Saved Passwords"
4. Click "Export passwords"
5. Save as `chrome_passwords.csv`

**Future CLI Command**:
```bash
bw import chrome chrome_passwords.csv
```

**What Gets Imported**:
- Login items only
- Name, URL, username, password

**Limitations**:
- Very simple format
- No folders (all items in root)
- No additional metadata

### Import Options

#### List Available Formats

**Future CLI Command**:
```bash
bw import --formats
```

**Output**:
```
Supported import formats:
  bitwardencsv    - Bitwarden CSV
  bitwardenjson   - Bitwarden JSON
  lastpass        - LastPass
  1password       - 1Password
  chrome          - Chrome Passwords
```

#### Specify Format

**Future CLI Command**:
```bash
# Format auto-detection not yet available
# Always specify format explicitly
bw import bitwardencsv backup.csv
```

### Import Process

1. **File Size Check**: Verifies file is under 100MB limit
2. **Parsing**: Reads and parses the file content
3. **Validation**: Checks all items for required fields
4. **Creation**: Creates folders and items in vault
5. **Confirmation**: Reports items and folders created

**Validation Rules**:
- Login items: Must have username OR password
- Card items: Must have card number
- All items: Must have a name
- URIs: Must be valid format

**Fail-Fast Strategy**: If any validation error occurs, no items are imported. Fix errors and try again.

### Import Examples

#### Import Bitwarden CSV

**Future CLI Command**:
```bash
# Import from CSV backup
bw import bitwardencsv vault-backup.csv

# Expected output:
# ✓ Successfully imported 247 items and 12 folders
```

#### Import from LastPass

**Future CLI Command**:
```bash
# Export from LastPass (manual step)
# Then import to Bitwarden
bw import lastpass lastpass_export.csv

# Expected output:
# ✓ Successfully imported 183 items and 8 folders
```

#### Import from 1Password

**Future CLI Command**:
```bash
# Export from 1Password (manual step)
# Then import to Bitwarden
bw import 1password 1password_export.csv

# Expected output:
# ✓ Successfully imported 156 items and 15 folders
```

## Migration Guides

### Migrating from LastPass

#### Step 1: Export from LastPass

1. Log in to your LastPass vault at lastpass.com
2. Click your profile icon → Account Settings
3. Navigate to Advanced → Export
4. Copy the CSV data or use the download option
5. Save to a file: `lastpass_export.csv`

**Security Note**: This file contains your passwords in plain text. Delete it after import.

#### Step 2: Import to Bitwarden

**Future CLI Command**:
```bash
# Import the LastPass export
bw import lastpass lastpass_export.csv
```

#### Step 3: Verify Import

**Future CLI Commands**:
```bash
# Count items
bw list items | jq '. | length'

# List folders
bw list folders | jq '.[] | .name'

# Search for specific item
bw get item "github"
```

#### Step 4: Cleanup

```bash
# Securely delete export file
shred -u lastpass_export.csv  # Linux
rm -P lastpass_export.csv     # macOS
```

#### What to Check

- **Item Count**: Verify count matches LastPass
- **Folders**: Verify all groups became folders
- **Favorites**: Check favorite items imported
- **Test Logins**: Test several random logins work

#### Known Differences

- LastPass groups → Bitwarden folders
- LastPass extra field → Bitwarden notes
- Only login items migrate (no secure notes or cards in LastPass CSV)

### Migrating from 1Password

#### Step 1: Export from 1Password

1. Open 1Password desktop application
2. Select vault to export
3. File → Export → CSV
4. Choose location and password (if prompted)
5. Save as `1password_export.csv`

**Security Note**: This file contains your passwords in plain text. Delete it after import.

#### Step 2: Import to Bitwarden

**Future CLI Command**:
```bash
# Import the 1Password export
bw import 1password 1password_export.csv
```

#### Step 3: Verify Import

- Check item count matches
- Verify folder structure preserved
- Test sample items
- Check item types (Login, Note, Card, etc.)

#### Step 4: Cleanup

```bash
# Securely delete export file
shred -u 1password_export.csv  # Linux
rm -P 1password_export.csv     # macOS
```

#### Known Differences

- 1Password vaults → Bitwarden folders
- 1Password categories map to Bitwarden cipher types
- Some 1Password fields may not have direct equivalents

### Migrating from Chrome

#### Step 1: Export from Chrome

1. Open Chrome browser
2. Navigate to chrome://settings/passwords
3. Click three dots (⋮) next to "Saved Passwords"
4. Click "Export passwords"
5. Authenticate if prompted
6. Save as `chrome_passwords.csv`

**Security Warning**: Chrome will warn you that passwords will be visible. Proceed with caution.

#### Step 2: Import to Bitwarden

**Future CLI Command**:
```bash
# Import Chrome passwords
bw import chrome chrome_passwords.csv
```

#### Step 3: Verify Import

- Check all passwords imported
- Test several random logins
- Verify URLs are correct

#### Step 4: Cleanup

```bash
# Securely delete export file
shred -u chrome_passwords.csv  # Linux
rm -P chrome_passwords.csv     # macOS
```

#### Known Limitations

- Chrome only exports basic login info
- No folders (all items imported to root)
- No notes or additional fields
- No cards, identities, or secure notes

### Migrating Between Bitwarden Accounts

#### Export from Source Account

**Future CLI Command**:
```bash
# Login to source account
bw login source@example.com

# Export vault
bw export --format json --output complete-backup.json

# Logout
bw logout
```

#### Import to Destination Account

**Future CLI Command**:
```bash
# Login to destination account
bw login destination@example.com

# Import backup
bw import bitwardenjson complete-backup.json

# Verify
bw sync
bw list items
```

## Common Workflows

### Regular Backup Routine

**Create a backup script**:

```bash
#!/bin/bash
# backup-vault.sh

# Configuration
BACKUP_DIR="$HOME/vault-backups"
DATE=$(date +%Y-%m-%d)
BACKUP_FILE="$BACKUP_DIR/vault-backup-$DATE.csv"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Export vault
bw export --output "$BACKUP_FILE"

# Verify backup created
if [ -f "$BACKUP_FILE" ]; then
    echo "✓ Backup created: $BACKUP_FILE"
    ls -lh "$BACKUP_FILE"
else
    echo "✗ Backup failed!"
    exit 1
fi

# Optional: Encrypt backup
gpg --symmetric --armor "$BACKUP_FILE"
rm "$BACKUP_FILE"

# Optional: Upload to cloud storage
# aws s3 cp "$BACKUP_FILE.asc" s3://my-bucket/backups/

# Cleanup old backups (keep last 30 days)
find "$BACKUP_DIR" -name "vault-backup-*.csv*" -mtime +30 -delete

echo "✓ Backup routine complete"
```

**Make executable and run**:
```bash
chmod +x backup-vault.sh
./backup-vault.sh
```

**Schedule with cron**:
```bash
# Run daily at 2 AM
0 2 * * * /path/to/backup-vault.sh
```

### Emergency Restore

**Scenario**: Need to restore vault from backup

**Steps**:

1. **Locate backup file**:
   ```bash
   ls -ltr ~/vault-backups/
   ```

2. **Decrypt if encrypted**:
   ```bash
   gpg --decrypt vault-backup-2024-01-15.csv.gpg > restore.csv
   ```

3. **Login to Bitwarden**:
   ```bash
   bw login
   ```

4. **Import backup**:
   ```bash
   bw import bitwardencsv restore.csv
   ```

5. **Verify restore**:
   ```bash
   bw list items | jq '. | length'
   ```

6. **Cleanup**:
   ```bash
   shred -u restore.csv
   ```

### Sharing Vault Data with Team

**Export specific folder**:

```bash
# Not yet supported - export entire vault
# Then manually filter items

# Export to JSON
bw export --format json --output full-vault.json

# Filter specific folder with jq
cat full-vault.json | jq '{
  encrypted: .encrypted,
  folders: [.folders[] | select(.name == "Team Shared")],
  items: [.items[] | select(.folderId == "folder-id-here")]
}' > team-shared.json

# Share team-shared.json with team
```

### Data Analysis

**Export for analysis**:

```bash
# Export to CSV
bw export --output vault-data.csv

# Analyze with command-line tools
cat vault-data.csv | csvlook
cat vault-data.csv | csvstat

# Find weak passwords (basic example)
cat vault-data.csv | grep -i "password" | awk -F',' '{print length($9), $9}' | sort -n

# Count items by type
cat vault-data.csv | awk -F',' '{print $3}' | sort | uniq -c
```

## Troubleshooting

### Export Issues

#### "Unsupported format: xml"

**Problem**: Invalid format specified

**Solution**: Use supported formats only:
- `csv` (default)
- `json`
- `encrypted_json` (when SDK available)

#### "Not authenticated"

**Problem**: No active session

**Solution**:
```bash
# Login first
bw login

# Or unlock if already logged in
bw unlock

# Use session key
export BW_SESSION="session-key-here"
```

#### Export file is empty

**Problem**: No items in vault

**Solution**: Verify vault has items:
```bash
bw list items
```

If vault is actually empty, export will succeed with header only (CSV) or empty items array (JSON).

#### "Permission denied" writing export file

**Problem**: Cannot write to output path

**Solution**:
- Check directory exists: `mkdir -p /path/to/directory`
- Check permissions: `ls -ld /path/to/directory`
- Try different location: `bw export --output ~/backup.csv`

### Import Issues

#### "File too large: 105906176 bytes (max: 104857600 bytes)"

**Problem**: Import file exceeds 100MB limit

**Solutions**:

1. **Split file**:
   ```bash
   # Split CSV into smaller files
   split -l 5000 large-export.csv import-part-

   # Import each part
   for file in import-part-*; do
       bw import bitwardencsv "$file"
   done
   ```

2. **Remove unnecessary data**:
   - Export only needed folders from source
   - Remove old/unused items before export

3. **Use JSON instead of CSV**:
   - JSON is more compact
   - Better for large datasets

#### "Validation failed" errors

**Problem**: Import data has validation errors

**Example Error**:
```
❌ Validation failed with 2 error(s):

  Line 5: login_username: Login must have username or password
  Line 12: name: Name is required

No items were imported. Please fix the errors and try again.
```

**Solution**:

1. **Read error message carefully** - includes line numbers and field names

2. **Edit source file** to fix issues:
   ```bash
   # Open CSV in editor
   nano import-file.csv

   # Jump to line 5, add username or password
   # Jump to line 12, add name field
   ```

3. **Common validation fixes**:
   - Add missing names (required for all items)
   - Add username OR password for login items
   - Remove empty rows
   - Fix malformed URIs

4. **Re-run import**:
   ```bash
   bw import bitwardencsv fixed-import.csv
   ```

#### "JsonError: missing field `revisionDate`"

**Problem**: JSON structure incomplete or corrupted

**Solution**:

1. **Verify JSON is valid**:
   ```bash
   cat import.json | jq '.'
   ```

2. **Check JSON came from Bitwarden**:
   - Only Bitwarden JSON exports have correct structure
   - Don't manually edit Bitwarden JSON

3. **Re-export from source**:
   - Export again from Bitwarden
   - Don't use modified JSON

#### "Unsupported format: keepass"

**Problem**: Format not yet implemented

**Solutions**:

1. **Check available formats**:
   ```bash
   bw import --formats
   ```

2. **Convert to supported format**:
   - Export from KeePass to CSV
   - Manually convert to Bitwarden CSV format
   - Or wait for KeePass parser implementation

3. **Request feature**:
   - File issue requesting KeePass support
   - Provide sample export file

#### Import succeeds but items not visible

**Problem**: Vault integration not yet complete (service layer only)

**Status**: Expected behavior - actual vault write pending

**Solution**: Wait for vault service integration

**Workaround**: None currently - feature in development

### Format-Specific Issues

#### LastPass: "grouping" field missing

**Problem**: Old LastPass export format

**Solution**: Re-export from LastPass using latest export feature

#### 1Password: Some items missing

**Problem**: 1Password CSV export is limited

**Solution**:
- Check 1Password export settings
- Some item types may not export to CSV
- Try exporting in smaller batches

#### Chrome: No folders

**Problem**: Chrome doesn't support folders

**Solution**: This is expected behavior
- Chrome only exports login items
- No folder support in Chrome
- Manually organize after import

## Security Best Practices

### Export Security

#### 1. Use Encrypted Exports When Possible

**Future**:
```bash
# Use encrypted JSON (when SDK available)
bw export --format encrypted_json --password "strong-password" --output backup.json
```

**Currently**:
```bash
# Encrypt CSV/JSON with GPG
bw export --format json | gpg --symmetric --armor > backup.json.gpg
```

#### 2. Secure Export Files Immediately

```bash
# Set restrictive permissions
chmod 600 vault-backup.csv

# Or encrypt immediately
bw export | gpg --symmetric > backup.gpg
```

#### 3. Store Backups Securely

**Good practices**:
- Encrypt before storing
- Use secure cloud storage
- Use offline/encrypted USB drives
- Don't email unencrypted exports
- Don't commit to git repositories

**Bad practices**:
- Leaving unencrypted exports on desktop
- Storing in Dropbox without encryption
- Sending via email
- Committing to version control

#### 4. Delete Exports After Use

```bash
# Securely delete export file
shred -u -n 3 vault-backup.csv  # Linux (3 passes)
rm -P vault-backup.csv          # macOS
srm vault-backup.csv            # macOS with srm installed

# Or just encrypt and delete original
bw export > backup.csv
gpg --symmetric backup.csv  # Creates backup.csv.gpg
shred -u backup.csv
```

### Import Security

#### 1. Verify Source of Import File

- Only import from trusted sources
- Verify file integrity (checksums)
- Scan for malware if from external source

#### 2. Review Import Data Before Importing

```bash
# Preview CSV
head -20 import-file.csv
cat import-file.csv | csvlook | less

# Preview JSON
cat import-file.json | jq '.' | less
cat import-file.json | jq '.items[] | .name' | less
```

#### 3. Delete Import Files After Use

```bash
# Securely delete
shred -u import-file.csv
```

#### 4. Review Imported Items

After import:
```bash
# List recent items
bw list items | jq '.[] | select(.revisionDate > "2024-01-01") | {name, login}'

# Check for suspicious items
bw list items | jq '.[] | select(.name | test("suspicious")) | .'
```

### General Security Tips

1. **Use Strong Master Password**: Protects encrypted exports
2. **Enable Two-Factor Authentication**: Protects account
3. **Regular Backups**: But secure them properly
4. **Audit Access**: Review who has access to backups
5. **Secure Backup Storage**: Encrypted cloud or offline storage
6. **Test Restores**: Verify backups work before you need them
7. **Clean Up Old Backups**: Don't leave old exports around

## Next Steps

After import/export:

1. **Verify Data**: Check items and folders imported correctly
2. **Test Logins**: Test several random items work
3. **Organize**: Organize items into folders if needed
4. **Clean Up**: Securely delete export files
5. **Enable 2FA**: Set up two-factor authentication
6. **Backup**: Create secure backup of new vault

## Getting Help

If you encounter issues:

1. **Check this guide** - Most common issues covered
2. **Review error messages** - They include helpful details
3. **Check logs** - Look for detailed error information
4. **File an issue** - Include error message and steps to reproduce
5. **Ask community** - Bitwarden forums and Reddit

## Appendix: CSV Format Specification

### Universal 34-Column Format

The Bitwarden CSV export uses a universal format supporting all cipher types:

**Headers**:
```
folder,favorite,type,name,notes,fields,reprompt,
login_uri,login_username,login_password,login_totp,
card_cardholderName,card_brand,card_number,card_expMonth,card_expYear,card_code,
identity_title,identity_firstName,identity_middleName,identity_lastName,
identity_address1,identity_address2,identity_address3,
identity_city,identity_state,identity_postalCode,identity_country,
identity_email,identity_phone,identity_ssn,identity_username,
identity_passportNumber,identity_licenseNumber
```

**Type Values**:
- `login` - Login credentials
- `note` - Secure notes
- `card` - Payment cards
- `identity` - Identity information

**Example Login Item**:
```csv
Work,0,login,github,Notes for github,,0,https://github.com,user@example.com,mypassword,,,,,,,,,,,,,,,,,,,,,,,,,
```

**Example Card Item**:
```csv
,0,card,Visa Card,,,0,,,,,John Doe,Visa,4111111111111111,12,2025,123,,,,,,,,,,,,,,,,,
```

**Example Identity Item**:
```csv
,0,identity,My Identity,,,0,,,,,,,,,,,Mr,John,Q,Public,123 Main St,,,Springfield,IL,62701,US,john@example.com,555-1234,123-45-6789,jqpublic,,
```

### Custom Fields Format

Custom fields are stored in the `fields` column in this format:

```
fieldName1: value1
fieldName2: value2
```

Example:
```csv
,0,login,example,,,0,https://example.com,user,pass,,"API Key: abc123
Security Question: blue",,,,,,,,,,,,,,,,,,,,,,
```

### Multiple URIs Format

Multiple URIs for a login item are separated by newlines within the quoted field:

```csv
,0,login,example,,,0,"https://example.com
https://www.example.com
https://app.example.com",user,pass,,,,,,,,,,,,,,,,,,,,,,,,,
```

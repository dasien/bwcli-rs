# Template API Specification

This document defines the exact JSON structure for item templates, ensuring TypeScript CLI compatibility.

## Template Types

| Template Name | Description |
|---------------|-------------|
| `item` | Alias for `item.login` |
| `item.login` | Login item template |
| `item.secureNote` | Secure note template |
| `item.card` | Payment card template |
| `item.identity` | Identity template |
| `folder` | Folder template |
| `item.field` | Custom field template |
| `item.login.uri` | URI entry template |

## Login Template (`item.login`)

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
    "uris": [
      {
        "match": null,
        "uri": "https://example.com"
      }
    ],
    "username": "jdoe",
    "password": "myp@ssword123",
    "totp": null
  },
  "secureNote": null,
  "card": null,
  "identity": null,
  "reprompt": 0
}
```

## Secure Note Template (`item.secureNote`)

```json
{
  "organizationId": null,
  "collectionIds": null,
  "folderId": null,
  "type": 2,
  "name": "Item name",
  "notes": "Some notes about this item.",
  "favorite": false,
  "fields": [],
  "login": null,
  "secureNote": {
    "type": 0
  },
  "card": null,
  "identity": null,
  "reprompt": 0
}
```

## Card Template (`item.card`)

```json
{
  "organizationId": null,
  "collectionIds": null,
  "folderId": null,
  "type": 3,
  "name": "Item name",
  "notes": "Some notes about this item.",
  "favorite": false,
  "fields": [],
  "login": null,
  "secureNote": null,
  "card": {
    "cardholderName": "John Doe",
    "brand": "visa",
    "number": "4242424242424242",
    "expMonth": "04",
    "expYear": "2025",
    "code": "123"
  },
  "identity": null,
  "reprompt": 0
}
```

## Identity Template (`item.identity`)

```json
{
  "organizationId": null,
  "collectionIds": null,
  "folderId": null,
  "type": 4,
  "name": "Item name",
  "notes": "Some notes about this item.",
  "favorite": false,
  "fields": [],
  "login": null,
  "secureNote": null,
  "card": null,
  "identity": {
    "title": "Mr",
    "firstName": "John",
    "middleName": "William",
    "lastName": "Doe",
    "address1": "123 Main St",
    "address2": "Apt 1",
    "address3": null,
    "city": "New York",
    "state": "NY",
    "postalCode": "10001",
    "country": "US",
    "company": "Acme Inc",
    "email": "jdoe@example.com",
    "phone": "555-123-4567",
    "ssn": "123-45-6789",
    "username": "jdoe",
    "passportNumber": "123456789",
    "licenseNumber": "D1234567"
  },
  "reprompt": 0
}
```

## Folder Template (`folder`)

```json
{
  "name": "Folder name"
}
```

## Custom Field Template (`item.field`)

```json
{
  "name": "Field name",
  "value": "Some value",
  "type": 0
}
```

**Field Types:**
- `0` = Text (visible)
- `1` = Hidden (password-like)
- `2` = Boolean

## URI Template (`item.login.uri`)

```json
{
  "match": null,
  "uri": "https://example.com"
}
```

**Match Types:**
- `null` = Default (uses global setting)
- `0` = Base domain
- `1` = Host
- `2` = Starts with
- `3` = Exact
- `4` = Regular expression
- `5` = Never

## Cipher Type Values

| Type | Value | Description |
|------|-------|-------------|
| Login | 1 | Website credentials |
| SecureNote | 2 | Encrypted note |
| Card | 3 | Payment card |
| Identity | 4 | Personal information |
| SshKey | 5 | SSH key (not in templates) |

## Reprompt Values

| Value | Description |
|-------|-------------|
| 0 | No reprompt |
| 1 | Master password required |

## Usage Examples

### Create Login Item

```bash
# Get template
bw get template item.login > item.json

# Edit item.json with your data
# Then encode and create
cat item.json | bw encode | xargs bw create item
```

### Create Using Raw JSON

```bash
bw create item '{"type":1,"name":"My Login","login":{"username":"user","password":"pass"}}'
```

### Create Folder

```bash
bw create folder '{"name":"Work"}'
```

## Notes

1. All templates use `camelCase` field names to match TypeScript CLI
2. The `type` field is numeric (1-5), not string
3. Templates include all fields even when null for documentation
4. `collectionIds` should be an array when set, not null
5. The `reprompt` field defaults to 0 if not specified

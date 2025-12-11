# Template Specifications

This document defines the exact JSON format for each template type, matching the TypeScript CLI for compatibility.

## Template: item.login (type=1)

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
  "reprompt": 0
}
```

## Template: item.secureNote (type=2)

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
  "secureNote": {
    "type": 0
  },
  "reprompt": 0
}
```

## Template: item.card (type=3)

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
  "card": {
    "cardholderName": "John Doe",
    "brand": "visa",
    "number": "4111111111111111",
    "expMonth": "12",
    "expYear": "2025",
    "code": "123"
  },
  "reprompt": 0
}
```

## Template: item.identity (type=4)

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
  "identity": {
    "title": "Mr",
    "firstName": "John",
    "middleName": null,
    "lastName": "Doe",
    "address1": "123 Main St",
    "address2": null,
    "address3": null,
    "city": "New York",
    "state": "NY",
    "postalCode": "10001",
    "country": "US",
    "company": null,
    "email": "john.doe@example.com",
    "phone": "555-123-4567",
    "ssn": null,
    "username": "johndoe",
    "passportNumber": null,
    "licenseNumber": null
  },
  "reprompt": 0
}
```

## Template: folder

```json
{
  "name": "Folder name"
}
```

## Template: item.field (for custom fields)

```json
{
  "name": "Field name",
  "value": "Field value",
  "type": 0
}
```

Field types:
- 0 = Text
- 1 = Hidden
- 2 = Boolean

## Template: item.field.linked (for linked custom fields)

```json
{
  "name": "Field name",
  "value": null,
  "type": 3,
  "linkedId": 100
}
```

Linked field IDs:
- 100 = Username
- 101 = Password

## Notes on Template Usage

### Workflow Example

1. Get template:
   ```bash
   bw get template item.login > login.json
   ```

2. Edit the template:
   ```bash
   # Edit login.json with your data
   ```

3. Encode and create:
   ```bash
   cat login.json | base64 | bw create item
   # OR for raw JSON support:
   cat login.json | bw create item
   ```

### TypeScript CLI Compatibility

These templates match the TypeScript CLI's `TemplateResponse` exactly. Key compatibility points:

1. **Field naming**: camelCase (e.g., `folderId` not `folder_id`)
2. **Null vs absent**: Explicit `null` values for optional fields
3. **Type values**: Integer type codes (1, 2, 3, 4)
4. **Arrays**: Empty arrays `[]` not `null`

### Implementation Notes

Templates should be:
1. Static JSON strings (not dynamically generated)
2. Stored as constants or embedded resources
3. Returned without modification (preserve exact formatting)

This ensures byte-for-byte compatibility with TypeScript CLI output for testing purposes.

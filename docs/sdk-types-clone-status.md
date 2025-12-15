# SDK Types Clone Status

This document tracks which Bitwarden SDK types implement `Clone` and which do not, along with workarounds used in the CLI.

## Types That Do NOT Implement Clone

| Type | Module | Notes |
|------|--------|-------|
| `FolderView` | `bitwarden_vault` | Use references (`&[FolderView]`) to avoid Clone requirement |
| `Collection` | `bitwarden_collections` | Not stored in cloneable structs |

## Types That DO Implement Clone

| Type | Module |
|------|--------|
| `CipherView` | `bitwarden_vault` |
| `Cipher` | `bitwarden_vault` |
| `CipherType` | `bitwarden_vault` |
| `FolderId` | `bitwarden_vault` |
| `CipherId` | `bitwarden_vault` |
| `CollectionId` | `bitwarden_collections` |
| `CollectionView` | `bitwarden_collections` |

## Workarounds Used

### JSON Export Struct Split

In `crates/bw-core/src/services/import_export/export/formatters/json.rs`, we use two separate structs to handle the Clone constraint:

```rust
// For serialization (export) - uses references to avoid Clone requirement
#[derive(Debug, Serialize)]
pub struct JsonExport<'a> {
    pub encrypted: bool,
    pub folders: &'a [FolderView],  // reference, no Clone needed
    pub items: &'a [CipherView],
}

// For deserialization (import) - owned values
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonExportOwned {
    pub encrypted: bool,
    pub folders: Vec<FolderView>,  // serde can deserialize directly
    pub items: Vec<CipherView>,
}
```

### ExportData Without Clone

In `crates/bw-core/src/services/import_export/export/mod.rs`, `ExportData` only derives `Debug` (not `Clone`) because it contains `Vec<FolderView>`:

```rust
#[derive(Debug)]  // No Clone - FolderView doesn't implement it
pub struct ExportData {
    pub folders: Vec<FolderView>,
    pub ciphers: Vec<CipherView>,
}
```

## General Guidelines

1. **When storing SDK types**: Check if they implement `Clone` before adding to structs that derive `Clone`
2. **For serialization**: Use references with lifetimes (`&'a [T]`) instead of owned `Vec<T>` when the type doesn't implement `Clone`
3. **For deserialization**: Serde can deserialize directly into owned types without needing `Clone`
4. **When passing around**: Prefer references or `Arc<T>` for non-Clone types that need to be shared

# SDK 3.0.0 Migration & Correctness Plan

Status doc for moving bwcli-rs onto the current Bitwarden SDK and fixing the
defects that analysis turned up along the way.

## Context

- bwcli-rs was written against SDK **1.0.0**; `Cargo.toml` is bumped to **2.0.0**
  (builds clean).
- Upstream `sdk-internal` main is at **3.0.0** — 582 commits / ~8 months ahead of
  the local checkout, with 22 new crates.
- Strategy: **adopt SDK crates, stay an independent CLI.** Upstream's own
  `crates/bw` is a reference implementation to crib from, not a destination.
- SDK consumption: **track upstream main.** Do each migration against a fixed
  commit (currently `9794da58`) so the target doesn't move mid-change, then
  resume tracking.

## Deferred decision: on-disk state format

TS-CLI compatibility and the SDK-native state path are **mutually exclusive**,
and the binding constraint is the *session* layer, not the vault layer. Real
drop-in interop needs TS-compatible vault data **and** tokens **and** session
simultaneously; partial compatibility buys nothing.

Deferred until the CLI is proven working end-to-end. Gates:
`bitwarden-sync`, vault CRUD via `CiphersClient`/`FoldersClient` (~1,200 LOC),
and `bitwarden-unlock` (~700 LOC).

If we later go SDK-native, migration from a TS-CLI install is feasible as a
one-time, non-destructive importer: carry over identity, base URLs, tokens and
account crypto state, then let `bw sync` rebuild the vault. The session cannot
carry over — users re-run `bw unlock` once for a new `BW_SESSION`. Unverified:
the exact TS `data.json` key holding the account private key.

## Findings that drive this plan

1. **The vault crypto path is dead code at runtime.** There are zero calls to
   `initialize_user_crypto` / `crypto()` / anything that populates the SDK
   `Client`'s KeyStore, yet `cipher_service.rs` calls
   `client.vault().ciphers().decrypt(...)` against it. `AuthService` keeps the
   user key only in its own `__PROTECTED__` storage. Compiles; cannot work.
   Tests pass because they never exercise real decryption.
2. `bitwarden-auth` is a declared dependency with **zero references**.
3. Two independent HTTP stacks with split token state (SDK `Client` +
   own reqwest `BitwardenApiClient`/`TokenManager`); self-hosted URL derivation
   disagrees between them (`{base}/api/api` vs `{base}/api`).
4. `send`, `receive`, `import`, `export` are all **stubs**.
5. bwcli-rs is substantially **ahead of upstream `crates/bw`**, whose command
   bodies are mostly `todo!()`. Upstream is finished on send/receive, `generate`
   parity, `config server`, and completions.

## Phases

Ordered so correctness work lands on a green build, before the large migration.

- [x] **1. Wire the SDK KeyStore after unlock** — fixes finding #1. Done
      against 2.0.0. `KeyService::initialize_client_crypto` loads the user key
      from `__PROTECTED__` storage into the SDK key store; called once from
      `main` via `ServiceContainer::unlock_sdk` when a session is present.
      Login now persists `user_{id}_crypto_privateKey` (TS-compatible key),
      required for `InitUserCryptoRequest::account_cryptographic_state`.
- [x] **2. Prove the vault round-trip** — `crates/bw-core/tests/keystore_init_tests.rs`,
      5 tests with real crypto (no mocks). Includes a regression guard asserting
      encryption *fails* on an uninitialized key store, and a folder
      encrypt→decrypt round-trip proving it succeeds after initialization.
      Follow-up: pre-existing logins have no stored private key and get an
      actionable `PrivateKeyNotFound`; backfilling from `/accounts/profile`
      would self-heal them without a re-login.
- [x] **3. Search/filter bugs** — done. `find_cipher_by_name` (ID-prefix match)
      replaced by `find_by_name` over decrypted `CipherListView`s, preferring
      exact name matches and erroring on genuine ambiguity rather than returning
      an arbitrary item. `--search` (name + subtitle) and `--url` (host
      comparison) now applied via `filter_decrypted` after decryption; the CLI
      was already passing them into `ItemFilters`, they were just dropped.
      `search_service.rs` had no test module at all — now 16 tests.
- [x] **4. Sync/write bugs** — done. Organizations are now parsed from the sync
      response profile (preferring `organizationsNew`, falling back to
      `organizations`) and persisted, so `bw list organizations` works. `sync`
      short-circuits on `GET /accounts/revision-date` unless `--force`, and
      reports a deleted account on a negative timestamp. `move_cipher` no longer
      swallows a bad `folder_id` into "no folder". `OrganizationPermissions`
      realigned to the current server field set (all `#[serde(default)]` so old
      state files still load). New `sync_service_tests.rs` (5 tests).
      Two items in `docs/bw-core-code-quality-issues.md` turned out to be stale
      or wrong and are annotated as such rather than "fixed".
- [x] **5. Migrate to SDK 3.0.0** — done against `../sdk-internal` @ `9794da58`.
      `rust-version` 1.88.0; **no toolchain bump was needed** — the SDK builds on
      the existing 1.91.1 pin despite upstream pinning 1.97.1. 15 breaks fixed:
      `ClientSettings` (+2 fields; now built with `..default()` so future field
      additions don't break it), `InitUserCryptoRequest::upgrade_token`,
      `SymmetricCryptoKey::make(SymmetricKeyAlgorithm::Aes256CbcHmac)`,
      `CryptoError::InvalidMac` -> `Decrypt`/`KeyDecrypt`, async
      `CiphersClient::{encrypt,decrypt,decrypt_list}` (+5 call sites),
      `CipherType` +3 variants (3 matches), `CipherView` +4 fields,
      `PasswordGeneratorRequest` +3 fields.
      `cargo test --workspace` now runs green for the first time: **187 passing,
      0 failures**. Also repaired the two long-broken test files
      (`import_export_tests`, `vault_write_service_tests`) and removed
      `test_create_cipher_rejects_invalid_uuid`, whose invariant typed `FolderId`
      now enforces at compile time.
- [x] **6. Adopt `bitwarden-exporters`; wire import/export.** Done. The three
      hand-rolled formatters (~382 LOC) are deleted; `ExportService` now
      delegates to `ExporterClient::export_vault`, which takes *encrypted*
      ciphers and decrypts them itself. `bw export` and `bw import` are no
      longer stubs. Password-protected export went from a stub that always
      errored to working. `--organizationid` is refused up front on both
      commands (the SDK's `export_organization_vault` is a `todo!()` that would
      abort the process). All five import parsers kept — `bitwarden-importers`
      only covers KDBX, so adopting it would be a five-format regression.
      The Bitwarden-JSON parser no longer borrows the export formatter's type;
      it owns lenient wire structs, so exports missing server-owned fields
      still import.
      Known behavior changes: CSV export is now the SDK's 11-column format
      (logins and secure notes only — cards and identities are dropped, which
      matches the TypeScript CLI) rather than the old 34-column dialect.
      Account-key-protected export (`encrypted_json` with no `--password`) has
      no SDK equivalent and is rejected rather than silently producing a
      password-protected file.
- [x] **7. Implement send/receive on `bitwarden-send`.** Text Sends work end to
      end: `list`, `get`, `create`, `edit`, `remove-password`, `delete`, and
      `receive` (anonymous and password-protected). Previously all stubs.
      Key piece: `JsonSendRepository`, a client-managed `Repository<Send>` over
      `data.json`. The SDK's send CRUD reads/writes through the state registry,
      and without registering an adapter it silently falls back to an in-memory
      DB that is empty every invocation — `bw send list` would always be empty.
      Sends are stored under the TypeScript CLI key `user_{id}_send_sends` and
      populated by `sync`.
      This is the first *client-managed repository* in the codebase, and is the
      pattern the deferred state decision would extend to ciphers and folders.
      **Not covered:** file Sends (refused explicitly on create/edit; `receive`
      shows metadata and warns) and email-OTP Sends. Both tracked separately.
- [ ] **8. Optional:** `bitwarden-auth` `login_via_password` (no 2FA or API-key
      support yet — keep the hand-rolled path for those); `generate` parity,
      `config server`, `clap_complete`, `bitwarden-cli` color.

## Bugs found and fixed along the way

- **`bw get template item | bw create item` was broken.** The CLI's own
  template output was rejected by its own parser
  (`invalid type: null, expected a sequence` on `"collectionIds": null`).
  Cause: `parse_item_input` deserialized straight into `CipherView`, a domain
  type whose server-owned fields (`creationDate`, `revisionDate`, `edit`,
  `viewPassword`, `organizationUseTotp`) are required. Fixed with a lenient
  `CipherInput` DTO that defaults everything and fills server-owned fields —
  the same shape upstream uses for sends (`SendJsonInput`). Guarded by
  `test_item_templates_are_parseable`, which round-trips every item template.
  Not a 3.0 regression; it was broken at 2.0.0 too.
- **Importing an empty file reported success.** A header-only or empty file
  parses to zero items and was returned as a successful import of nothing; now
  an explicit error.
- **Import validation errors lost their detail.** `ImportError::ValidationError`
  only carried a count ("Validation failed with 1 error(s)"); field/line detail
  went to stderr only. It now carries `first_error`, so single-line and
  `--response` JSON consumers get something actionable.

## Found by live testing against a real vault (2026-08-22)

The whole suite passed while three of these were broken. Nothing that existed
could have caught them, because no test crossed the network.

- **`bw login` failed for everyone.** The server renamed
  `ResetMasterPassword` to `ForcePasswordReset`; our required `bool` made serde
  reject the entire token response. The field is read nowhere. Now aliased and
  defaulted, along with `expires_in`/`token_type` — informational fields must
  not be able to fail a login. `Kdf`/`KdfIterations` are deliberately still
  required: defaulting them risks silently deriving a wrong key.
- **Writes succeeded but reported failure.** `write_service` deserialized
  POST/PUT responses straight into `Cipher`, which is `deny_unknown_fields` and
  rejects the server's `object` field. `bw create item` created the item, told
  the user it failed, and skipped the cache update. Five sites; all now go
  through the tolerant generated response models plus the SDK's `TryFrom`.
- **`bw edit item` and `bw move` could never have worked.** `update_cipher` set
  `revision_date = now()` before encrypting, but `CipherRequestModel` derives
  `lastKnownRevisionDate` from it and the server uses that for optimistic
  concurrency, so every edit was rejected as "out of date". The server owns
  that field; we no longer touch it.
- **The error handler hid the above.** `extract_error_message` deserialized into
  a struct whose fields are all `Option`, so parsing always succeeded and the
  `"Unknown error"` default was returned, making the raw-body fallback
  unreachable. It also didn't know about `validationErrors`. Split into a pure
  `describe_error_body` with 6 tests.

Common thread: hand-rolled API models and error handling drift from the server
and no test can catch it. Argues for phase 8 (`bitwarden-auth`) and for the
generated API clients, which are tolerant by construction.

**Verified live:** login, sync, `--force` bypass, revision-date skip, decrypted
`list items`, `--search`, exact-vs-ambiguous `get item`, `get password`,
folders, all three export formats, org-export refusal, and
create/edit/delete/restore round-trips. Organizations and sends now persist,
but both were empty in the test vault, so their *parsing* paths remain
unexercised.

## Do not adopt

- **`bitwarden-sensitive-value`** — does not zeroize, serializes transparently.
  A regression on `secrecy`.
- **`bitwarden-server-communication-config`** — SSO load-balancer cookies, not
  environment config. `services/api/environment.rs` stays hand-rolled; the SDK
  has no multi-URL (icons/notifications/events) model.
- **`bitwarden-state` SQLite as a second store** — would create a source of
  truth competing with `data.json`.

## Known SDK gaps (must stay hand-rolled)

- SSO login: absent from the SDK entirely.
- `newDeviceOtp`; master-password-hint (generated bindings only).
- No `CipherSyncHandler`, and no repository item for collections or
  organizations — `bitwarden-sync` requires writing our own handlers.
- `export_organization_vault` is `todo!()` and panics.
- Account-protected (non-password) export has no SDK equivalent.
- `bitwarden-importers` covers KDBX only — keep all five parsers.
- Client-side length validation.

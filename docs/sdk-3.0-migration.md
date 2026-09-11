# SDK 3.0.0 Migration & Correctness Plan

Status doc for moving bwcli-rs onto the current Bitwarden SDK and fixing the
defects that analysis turned up along the way.

## Context

- bwcli-rs was written against SDK **1.0.0**; `Cargo.toml` is bumped to **2.0.0**
  (builds clean).
- The SDK's `main` is at **3.0.0** — 582 commits / ~8 months ahead of
  the local checkout, with 22 new crates.
- Strategy: **adopt SDK crates, stay an independent CLI.** `crates/bw` is a
  reference implementation to crib from, not a destination.
- SDK consumption: **track the SDK's main.** Do each migration against a fixed
  commit (currently `26112cf3`) so the target doesn't move mid-change, then
  resume tracking.

## Decided (2026-08-22): go SDK-native, and target TS-CLI replacement

**Goal.** bwcli-rs is intended to *replace* the TypeScript CLI, not coexist with
it. Users are not expected to run both against one state file, so on-disk
interop is explicitly a non-goal. The long-term acceptance criterion is
different: **every TS-CLI command must have a working or stubbed replacement.**

**State layer: SDK-native.** Adopt `bitwarden-state` (SQLite), `bitwarden-pm`,
`bitwarden-unlock` and `PasswordManagerTokenHandler`; retire the `data.json`
machinery; ship a one-time importer for users coming from the TS CLI.

What tipped it: the interop we were protecting **did not exist**. Real
TS-CLI-written `data.json` (v2025.11.0) stores cipher `permissions` as
`{delete, response, restore}`, and SDK `CipherPermissions` is
`deny_unknown_fields` with only `{delete, restore}` — so every TS-written cipher
fails to deserialize. We could not read genuine TS state, and our own sync
writes a different shape. The two were already mutually incompatible; keeping
`data.json` was paying a cost for a benefit we never had.

Secondary benefit: letting the SDK own tokens removes the class of bug that
produced the deadlocked, `client_id`-less, unflushed refresh path.

## SDK export gap found in step 4 (worth reporting to the SDK team)

`CiphersClient::create`/`edit` and `FoldersClient::create`/`edit` take request
types — `CipherCreateRequest`, `CipherEditRequest`, `FolderAddEditRequest` —
that `bitwarden-vault` does not export; `cipher_client` is `pub(crate)`. The
methods are therefore uncallable from outside the crate, and nothing in the SDK
tree calls them: not the wasm bindings, not uniffi, not `crates/bw`.
`FoldersClient` has no `delete` at all.

Consequence: vault writes are split, each side forced rather than chosen.
- `CiphersClient`: cipher delete / soft-delete / restore / move (ids only; these
  update the state repository themselves).
- Generated `CiphersApi`/`FoldersApi` + an explicit repository write: cipher
  create/edit, all folder writes. The explicit write matters because reads now
  come from the repository, so without it a create would not appear until the
  next sync.

Step 3 closed the *transport* half of this: the generated clients are the same
ones `CiphersClient` calls internally, so these writes share the SDK's
authentication, renewal and 401 retry. What is still missing is only the
higher-level bookkeeping. Two smaller gaps had to be worked around:

- `Cipher` has a public `TryFrom<CipherDetailsResponseModel>`, but the write
  endpoints answer with `CipherResponseModel`. The SDK bridges these with
  `PartialCipher::merge_with_cipher`, which is `pub(crate)`. The two models are
  field-for-field identical apart from `collectionIds`, so `write_service`
  widens one into the other by destructuring — which makes the compiler flag it
  if that stops being true.
- `CipherResponseModel` carries no `collectionIds`, so an edit has to carry the
  ones it sent forward by hand. Missing this would silently unshare an
  organization item on every edit. (The SDK's `edit.rs` has a test for exactly
  this, which is how the trap was spotted.)

Revisit if those types get exported.

## TS-CLI parity matrix

**Source of truth:** read out of `~/Source/repos/Bitwarden/clients` at CLI
`v2026.8.0` (HEAD `cce8a34`, 2026-08-22) — `apps/cli/src/program.ts`,
`vault.program.ts`, `tools/send/send.program.ts`, `serve.program.ts`,
`dirt/report.program.ts`. The previous version of this matrix was written from
recollection and got two things wrong; see the corrections below. **Re-derive it
from the checkout rather than editing it by hand.**

Top-level commands the OSS TypeScript CLI has and we do not:

| Command | Notes |
|---|---|
| `archive` | `bw archive item <id>`. Paired with `restore`, whose description is now "Restores an object from the trash **or archive**". A whole feature we do not model. |
| `report` | `bw report password-health`, with `--no-check-exposed`. Overlaps our `get exposed`, which only checks one password. |
| `serve` | Local HTTP API. |
| `completion` | Shell completion; `clap_complete` would supply it. |
| `update` | Self-update check. |
| `sdk-version` | Prints the bundled SDK version. |


Object-level gaps:

| | TypeScript CLI | Ours |
|---|---|---|
| `get` | adds `notes`, `send` | missing both. `FieldType::Notes` already exists in `bw-core`, so `get notes` is small. |
| `move` | `<id> <organizationId> [encodedJson]` | matches now (was a folder move; see `BUGLIST.md` C23) |
| `share` | deprecated alias of `move` | accepted as an alias |
| `list` | `items folders collections org-collections org-members organizations` | all present; `org-collections`/`org-members` are stubs |
| `create` | `item attachment folder org-collection` | `org-collection` is a stub; `attachment` implemented |
| `edit` | `item item-collections folder org-collection` | `item-collections`, `org-collection` are stubs |
| `delete` | `item attachment folder org-collection` | `org-collection` is a stub; `attachment` implemented |
| `restore` | `item` | matches (fixed; was a bare id) |
| `archive` | `item` | command absent |

### What the SDK already provides for each stub

Surveyed 2026-08-23 against `sdk-internal` at `9794da58`, re-checked at
`26112cf3` on 2026-09-11 — none of these gaps closed. Every
"Not yet implemented" message is **ours** (`Response::error` in
`crates/bw-cli/src/commands/`) — none of these reach `bw-core`, let alone the SDK.
The question is what it would take to make each real.

**High-level SDK client, reachable today:**

| Stub | SDK entry point |
|---|---|
| ~~`create attachment`~~ **done** | `create_attachment()` + `encrypt_buffer()`; the byte upload is ours (`BUGLIST.md` S9) |
| ~~`get attachment`~~ **done** | `get_attachment_download_url()` + `decrypt_buffer()` |
| ~~`delete attachment`~~ **done** | `delete_attachment()` |
| `edit item-collections` | `CiphersClient::bulk_update_collections()` — adds or removes without duplicating |
| `get fingerprint` | `PlatformClient::fingerprint()` / `user_fingerprint()` |

Attachments are worth calling out: `attachment_client` is `pub(crate)`, the same
shape as the `cipher_client` export gap (see above), **but** it re-exports its
types (`AttachmentsClient`, `CreateAttachmentRequest`, `CreatedAttachment`,
`AttachmentFileUploadType`) and `VaultClient::attachments()` exists. So unlike
cipher create/edit, these are callable. The SDK also rolls back an orphaned
attachment slot if a later step fails. The same file-upload machinery is what file
Sends need, so doing attachments first de-risks that.

**Done, 2026-08-23**, in `bw-core/src/services/vault/attachment_service.rs`. One
thing the survey above got wrong: `create_attachment` opens the *slot* only — it
does not upload. The SDK's uploader exists but is private, and the generated
endpoint that looks like the answer sends no body, so the two transports (Azure
presigned `PUT`, `Direct` multipart `POST`) are reimplemented here. See
`BUGLIST.md` S9. The SDK's own rollback of an orphaned slot only covers failures
*inside* `create_attachment`; a failed upload afterwards is ours to undo, and
`AttachmentService::create` does.

**Generated endpoint exists; the crypto is ours to write:**

| Stub | What exists |
|---|---|
| `list org-collections` | `collections_api::get_many_with_details` / `get_all` |
| `list org-members` | `organization_users_api::get_all` / `get_mini_details` |
| `create`/`edit`/`delete org-collection` | `collections_api::post`/`put`/`delete`, with `CollectionView: CompositeEncryptable` for the name |
| `confirm` | `organization_users_api::confirm` — but wrapping the org key to the member's public key is not in the SDK |

Note `bitwarden-collections` has **no client at all** — only `collection.rs`,
`error.rs`, `tree.rs`. These are endpoints plus crypto traits, not drop-ins.

**Trivial, and stubbed for no remaining reason:**

- `get organization` — plain local data from the organization map; no SDK needed.
- `get collection` — `Collection: Decryptable<CollectionView>`, which `bw list
  collections` already uses. A lookup over a list we decrypt today.

**Nothing usable in the SDK:**

- `get exposed` — the generated `hibp_api::get` returns `Result<(), Error>`: no
  typed response body, so it cannot report anything. And the TypeScript CLI does
  not use the Bitwarden server for this; it calls the public pwnedpasswords
  k-anonymity range API directly. Needs a hand-rolled HTTP call either way.
  (`bw report password-health` is the bulk version, also absent from ours.)
- `config` — `config_api::get_configs` returns *server* config; `bw config server`
  is local settings, with no SDK equivalent.
- `decrypt` — not a TypeScript CLI command; still a removal candidate.

Rough value order given the above: ~~**attachments**~~ (done), then the two
trivial `get` stubs, then `edit item-collections` (one call), then the
org-collection/org-member set.

Ours that exist but are stubs: `config`, `confirm`, `login sso`,
`list org-collections|org-members`, `create org-collection`,
`edit item-collections|org-collection`, `delete org-collection`,
`get collection|organization|exposed|fingerprint`, file Sends,
org import/export.

Ours that are not TypeScript CLI commands at all: `decrypt` (stubbed — candidate
for removal).

Working: `login` (password + API key, 2FA, new-device OTP), `logout`, `lock`,
`unlock`, `status`, `sync`, `list items|folders|collections|organizations`,
`get item|username|password|uri|totp|folder|template`, `create item|folder`,
`edit item|folder`, `delete item|folder`, `restore item`, `generate`, `encode`,
`import`, `export`, text Sends, `receive`, `move-to-folder`.

`create|get|delete attachment` are implemented and **verified against a live
server on 2026-08-27**: upload, download, decrypt and delete all round-trip
byte-identically, including a 4 KiB random binary. The unit tests still do not
cross the network — that gap is unchanged, and it is what hid C4, C25 and C28 —
so the evidence here is the recorded live run, not the suite.

`move` / `share` (org-share) works, verified end to end against a real
organization. Organization keys now reach the key store via `sync`, which is also
what lets organization-owned items and collection names decrypt at all.

**Corrections to the earlier matrix** (both were memory, not source):

- **`device-approval` is not an OSS CLI command.** It appears nowhere in
  `apps/cli/src`. Presumably commercial-only; it should never have been on the
  missing list.
- **`move` is not a folder command.** `vault.program.ts:31` registers it as
  `this.shareCommand("move", false)` — "Move an item to an organization" — and
  `share` is the *deprecated* alias of it. Folder changes in the TypeScript CLI go
  through `bw edit item` with a changed `folderId`; there is no folder-move
  command. **Resolved:** `move` is now org-share, `share` is an alias, and our
  folder move is `move-to-folder`. See `BUGLIST.md` C23.
- **`archive` and `report` were missing from the matrix entirely.**

## Superseded: deferred decision on the on-disk state format

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
5. bwcli-rs is substantially **ahead of `crates/bw`** on coverage, whose command
   bodies are mostly `todo!()` (18 of them; no vault writes at all). `crates/bw`
   is finished on send/receive, `generate` parity, `config server`, and
   completions. Its *architecture*, however, is better than ours in three ways
   that each map to a bug we shipped — see
   `docs/cli-architecture-adoption.md`.

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
      the existing 1.91.1 pin despite the SDK pinning 1.97.1. 15 breaks fixed:
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
- [x] **8. Hand tokens to the SDK (`PasswordManagerTokenHandler`).** Done, and
      it deleted more than it added: `TokenManager` (269 LOC), `StoredAccessToken`,
      `TokenRefreshRequest`/`TokenResponse`, every `*_with_auth` method on
      `ApiClient`, `valid_access_token`, `execute_with_retry`'s refresh branch,
      `AccountManager::is_logged_in` and two dead `SessionManager` methods.
      Tokens now live in the SDK's `AUTHENTICATION_TOKENS` setting alongside a
      `USER_LOGIN_METHOD` (`client_id: "cli"`), and the SDK's middleware owns
      renewal: proactive at a 5-minute margin, once more on a 401, serialized
      behind a mutex. That is the same three things our hand-rolled path did,
      each of which had a bug.
      Prerequisite, done first: move cipher create/edit and all folder writes off
      the hand-rolled HTTP client and onto the generated `CiphersApi`/`FoldersApi`.
      With that, `BitwardenApiClient` has **no authenticated method left** — it
      serves only login, prelogin and the identity endpoints. Its doc comment now
      says so, because adding one back would recreate the split token state.
      `InternalClient::set_tokens`/`set_login_method` are `pub(crate)`, so
      `sdk_session::persist_tokens` writes the same two settings directly. Doing
      it by hand rather than via `bitwarden_core::auth().login_password` is
      deliberate: that method hardcodes `DeviceType::ChromeBrowser`, a fixed
      device identifier, and `client_id: "web"` — a CLI login would show up as a
      Chrome device.
      Note: **tokens no longer live in `data.json`.** Existing logins must
      re-run `bw login`; see step 9.
- [x] **9. One-time `data.json` carry-over.** Done, and it recovers a login
      without a re-login: verified against a real install whose tokens were
      stranded in `data.json` by step 8. `bw status` went from
      `unauthenticated` to `locked`, then `bw sync` renewed from the carried-over
      refresh token and succeeded.
      Runs automatically at startup, guarded on `is_authenticated`, so it is a
      no-op once migrated and needs no flag. Never overwrites an existing SDK
      setting; never removes or overwrites a `data.json` value, though it does
      *add* `activeAccountId` and a registry email when absent, because
      `data.json` still namespaces sends, collections, organizations and last-sync
      by user and eight call sites would otherwise be unable to find the account
      it just migrated.
      Account selection is, in order: the user id SDK state already names,
      `activeAccountId`, then the sole token holder. The first rule is a safety
      rule, not a convenience — with a conflicting `activeAccountId` it prevents
      pairing one account's tokens with another's cryptographic state. It also
      resolves the real-world case that motivated it: two accounts holding
      refresh tokens and no active account, which the sole-holder rule alone
      refuses to guess at.
      The carried-over token is recorded as already expired so the first request
      renews rather than sending a stale one. The vault is not carried over —
      `bw sync` rebuilds it, avoiding the shape mismatches that made the formats
      incompatible. The session cannot carry over; `bw unlock` mints a new one.
- [ ] **10. Remaining:** `generate` parity, `config server`, `clap_complete`,
      `bitwarden-cli` color.

## Bug list

Every defect found during this migration — SDK and ours, open and fixed — is
tracked in [`BUGLIST.md`](../BUGLIST.md). The narrative sections below stay for
context; the bug list is the running record.

## Two more SDK landmines (2026-08-22)

- **`initialize_user_crypto` clobbers `USER_LOGIN_METHOD`.** It unconditionally
  writes `UserLoginMethod::Username { client_id: "" }`
  (`bitwarden-core/src/key_management/crypto.rs:403`). Once the SDK owns tokens
  this is destructive twice over: it blanks the `client_id` renewal must send,
  and for an API-key login it discards the client secret those tokens are
  re-minted from. `bw unlock` runs through `initialize_crypto`, so **an unlock
  would have silently broken token renewal** — the same `invalid_request` failure
  we had just fixed, reintroduced from a different direction. Found by reading a
  real `user.sqlite`, which had `client_id: ""` sitting in it.
  `sdk_session::initialize_crypto` now snapshots the login method and restores it
  afterwards, treating a blank `client_id` as absent. Two tests cover it; both
  fail without the restore.

- **`bw list folders` and `bw export` read a store `sync` no longer writes.**
  Step 4 moved `sync` to write ciphers and folders into the SQLite repositories,
  but `VaultService::get_ciphers`/`get_folders` still read the `data.json` keys.
  `bw list items` hid it by going through `CiphersClient::list`. So `list folders`
  failed with "Vault not synced" and `export` (which uses `encrypted_ciphers`)
  was broken too. My step-4 verification checked `list items` and the write path
  and reported a folder count read straight from the repository — never the
  command. Both now read the repository. Verified: `list folders` returns the
  folder, `export` returns 11 items and 1 folder.

## Bugs found and fixed along the way

- **`bw get template item | bw create item` was broken.** The CLI's own
  template output was rejected by its own parser
  (`invalid type: null, expected a sequence` on `"collectionIds": null`).
  Cause: `parse_item_input` deserialized straight into `CipherView`, a domain
  type whose server-owned fields (`creationDate`, `revisionDate`, `edit`,
  `viewPassword`, `organizationUseTotp`) are required. Fixed with a lenient
  `CipherInput` DTO that defaults everything and fills server-owned fields —
  the same shape `crates/bw` uses for sends (`SendJsonInput`). Guarded by
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

## Phase 8: removing hand-rolled code (in progress)

Done:
- **SDK client is now authenticated.** `StoredAccessToken` implements
  `ClientManagedTokens` over our stored token, wired via
  `Client::new_with_token_handler`. Before this the SDK client had no
  credentials, which is *why* the CLI carried a second HTTP stack — any
  `*_api()` call would have gone out unauthenticated.
- **Sync moved to the generated clients.** `sync_api().get` and
  `accounts_api().get_account_revision_date` replace the hand-rolled calls;
  `endpoints::api::{SYNC, ACCOUNT_REVISION_DATE}` deleted.
- **`GET /accounts/profile` eliminated.** Identity now comes from the access
  token's own claims via `JwtToken`, removing a round trip *and* the
  hand-rolled `ProfileResponse` model — one less thing that can break the way
  `ForcePasswordReset` did.

Two pre-existing bugs this uncovered, both latent because refresh had never
actually run:
- **`TokenManager::refresh_access_token` deadlocked.** The `refresh_state` guard
  was only dropped on the "already refreshing" path; the other path re-locked
  the same non-reentrant mutex while still holding it. Any refresh hung forever.
- **Refresh omitted `client_id`**, so the identity server answered
  `invalid_request`. Combined with the deadlock, token refresh could never have
  worked — the CLI simply stopped functioning an hour after login. Also added the
  missing `storage.flush()`, without which renewed tokens were never persisted.

Not worth doing, with reasons:
- **Cipher/folder write endpoints.** The generated clients return
  `CipherResponseModel`, and `TryFrom<CipherResponseModel> for Cipher` was
  removed in 3.0.0 (only `CipherDetailsResponseModel` has one). Migrating would
  need an uglier conversion hop than the current code. These are properly
  retired by adopting `CiphersClient`, which is gated on the state decision.
- **`services/api/environment.rs`.** No SDK equivalent; `ClientSettings` models
  only `api_url`/`identity_url`, not icons/notifications/events/web-vault.

Investigated and deliberately **not** done — both would be worse than what we
have:

- **`bitwarden-auth` `login_via_password`.** Verified against `origin/main`:
  `LoginResponse` has only an `Authenticated` variant (`TwoFactorRequired` is
  commented out, new-device verification is a TODO), `LoginApiRequest::new`
  hardcodes the three `two_factor_*` fields to `None` with no way to set them,
  and `client_credentials` appears nowhere in the crate. So it cannot do 2FA,
  new-device verification, or API-key login — all of which this CLI supports.
  Worse than merely lacking them: a 2FA-enabled account gets a two-factor
  challenge that `LoginResponse` *cannot represent*, so routing login through it
  would actively break those accounts. Revisit when the variant is uncommented.
- **`services/crypto.rs` -> `MasterPasswordAuthenticationData::derive`.** The
  file is already pure SDK delegation — three one-line calls into
  `bitwarden-crypto`, no custom crypto (its own header says so). The SDK helper
  does not return the `MasterKey`, which we still need to decrypt the user key,
  so adopting it would run PBKDF2 twice per login (600k iterations each).
  The one real prize would have been correct salt handling — the SDK notes salt
  can differ from email — but the live server returns no salt: both
  `POST /accounts/prelogin` and `POST /accounts/prelogin/password` answer
  `{kdf, kdfIterations, kdfMemory, kdfParallelism}` and nothing else. Email-as-
  salt is what is actually available.

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

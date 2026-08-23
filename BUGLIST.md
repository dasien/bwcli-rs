# Bug list

Everything found while migrating bwcli-rs to Bitwarden SDK 3.0.0 and going
SDK-native, from 2026-08-20 onward. Kept as a running list: **add to it, don't
prune it.** A bug that has been fixed is still worth keeping — several here
recurred from a different direction, and the record is what caught them.
Entries are corrected in place when a diagnosis turns out to be wrong, with the
correction noted rather than the original quietly overwritten (see C3/C23).

Ids are stable: a fixed bug keeps its number and moves section rather than being
renumbered, so references from commits and code comments stay valid.

Sections are grouped by where the defect lives, because that decides who can fix
it. Within each, **Open** comes before **Fixed**.

Legend for **Status**:

| | |
|---|---|
| **Fixed** | fixed in this repo, with a test unless noted |
| **Worked around** | the defect is upstream; we compensate locally |
| **Open** | known, not addressed |
| **Won't fix** | deliberately left, with a reason |

A recurring shape worth stating once, because it explains why so much of this was
invisible: **a passing test suite proved almost nothing here.** No test crossed
the network, so every hand-rolled API model and error path could drift from the
server unchecked. And repeatedly, a working *read* path masked a broken *write*
path — verifying reads is much weaker evidence than it feels like.

---

## Index

**SDK (`sdk-internal`)** — none fixed upstream; all either worked around here or
left alone.

| | Bug | Status |
|---|---|---|
| S1 | `initialize_user_crypto` destroys the persisted login method | Worked around |
| S2 | `CiphersClient`/`FoldersClient` write methods uncallable externally | Worked around |
| S3 | `PartialCipher` is `pub(crate)`; no `TryFrom<CipherResponseModel>` | Worked around |
| S4 | `CipherResponseModel` omits `collectionIds` | Worked around |
| S5 | `get_sdk_managed_migrations` omits `LocalUserDataKeyState` | Worked around |
| S6 | `export_organization_vault` is `todo!()` | Worked around |
| S7 | `CipherPermissions` is `deny_unknown_fields`, rejects TS-CLI data | Won't fix |
| S8 | `bitwarden-sensitive-value` doesn't zeroize | Won't fix |

**bwcli-rs** — 1 open, 29 fixed.

| | Bug | Status |
|---|---|---|
| C27 | `bw get template` needs an unlocked vault | **Open** |
| C23 | `bw move` collided with the TS CLI's org-share command | Fixed |
| C24 | `bw get org` should be `bw get organization` | Fixed |
| C25 | `bw encode` required an argument instead of reading stdin | Fixed |
| C28 | `bw unlock --raw` printed the whole blurb, not the session key | Fixed |
| C29 | A session key reached a pushed branch as a **filename** | Fixed |
| C26 | String payloads were JSON-quoted, breaking `$(bw get password …)` | Fixed |
| C1 | `bw export --format json` emits invalid JSON | Fixed |
| C2 | Self-hosted URLs persisted but never read back | Fixed |
| C3 | `bw restore` took a bare id; `bw move` could not clear a folder | Fixed |
| C4 | `bw list folders` / `bw export` read a store `sync` stopped writing | Fixed |
| C5 | `refresh_access_token` deadlocked | Fixed |
| C6 | Token refresh omitted `client_id` | Fixed |
| C7 | Renewed tokens never persisted | Fixed |
| C8 | Vault crypto was dead code at runtime | Fixed |
| C9 | `bw login` failed for every user | Fixed |
| C10 | Writes succeeded but reported failure | Fixed |
| C11 | `bw edit item` / `bw move` could never have worked | Fixed |
| C12 | The error handler hid C9, C10 and C11 | Fixed |
| C13 | `bw get template item \| bw create item` rejected by our own parser | Fixed |
| C14 | A stale session silently returned garbage | Fixed |
| C15 | `UnlockClient::unlock` leaves the client with no user id | Fixed |
| C16 | Tracing wrote to stdout | Fixed |
| C17 | Non-UUID user id failed deep inside the SDK | Fixed |
| C18 | Error chains truncated to their outermost message | Fixed |
| C19 | Importing an empty file reported success | Fixed |
| C20 | Import validation errors lost their detail | Fixed |
| C21 | `bw sync` reported "not authenticated" for an existing login | Fixed |
| C22 | Identity mix-up risk in the carry-over | Fixed |

Note how the numbering clusters: C5–C7 are three independent defects in one
refresh path that had **never once executed successfully**, and C9–C12 were all
found in a single afternoon of live testing after C12 made errors legible.

---

## SDK bugs (`sdk-internal`) — worth reporting upstream

### Open / worked around

#### S1. `initialize_user_crypto` destroys the persisted login method
- **Command:** `bw unlock` (and any second `bw login`)
- **Location:** `bitwarden-core/src/key_management/crypto.rs:403`
- **What happens:** it unconditionally writes
  `USER_LOGIN_METHOD = UserLoginMethod::Username { client_id: "" }`. Once the SDK
  owns tokens (`PasswordManagerTokenHandler`), that setting is what renewal reads
  the OAuth `client_id` from — so a blank one makes the identity server answer
  `invalid_request`. For an **API-key** login it is worse: the whole variant is
  replaced, discarding the `client_secret` those tokens are re-minted from, with
  no way to recover but a fresh `bw login`.
- **Why it hid:** `bw unlock` succeeds; the vault decrypts fine. The damage only
  surfaces an hour later when a token needs renewing.
- **Found by:** reading a real `user.sqlite` and noticing `client_id: ""` sitting
  in it.
- **Our fix:** `sdk_session::initialize_crypto` snapshots the login method,
  calls through, then restores it — treating a blank `client_id` as absent so a
  first login gets a correct one written instead. **Worked around** (`sdk_session.rs`).
- **Proposed upstream fix:** don't touch `USER_LOGIN_METHOD` here at all; it is
  not this function's state. Failing that, preserve an existing method's variant
  and `client_id`, and take the `client_id` as a parameter.
- **Tests:** `initializing_crypto_leaves_a_usable_login_method`,
  `initializing_crypto_preserves_an_api_key_login_method` — both fail without the
  restore.

#### S2. `CiphersClient`/`FoldersClient` write methods are uncallable from outside the crate
- **Command:** `bw create item`, `bw edit item`, `bw create folder`, `bw edit folder`, `bw delete folder`
- **Location:** `bitwarden-vault` — `cipher_client` is `pub(crate)`, so
  `CipherCreateRequest`, `CipherEditRequest` and `FolderAddEditRequest` are
  unexported. `FoldersClient` has no `delete` at all.
- **What happens:** `CiphersClient::create`/`edit` and `FoldersClient::create`/`edit`
  are `pub` but take types no external caller can name. Nothing in the SDK tree
  calls them either — not the wasm bindings, not uniffi, not upstream `bw`.
- **Our fix:** use the generated `CiphersApi`/`FoldersApi` clients underneath
  instead, plus an explicit state-repository write (which the high-level client
  would have done for us). Same transport, auth and retry; only the bookkeeping
  is ours. **Worked around** (`vault/write_service.rs`).
- **Proposed upstream fix:** export the three request types, and add
  `FoldersClient::delete`.
- **Status:** Open upstream.

#### S3. `PartialCipher::merge_with_cipher` is `pub(crate)`, and `Cipher` has no `TryFrom<CipherResponseModel>`
- **Command:** `bw create item`, `bw edit item`
- **Location:** `bitwarden-vault/src/cipher/cipher.rs:1968` (trait),
  `:1861` (the one public `TryFrom`, for `CipherDetailsResponseModel`)
- **What happens:** the write endpoints answer with `CipherResponseModel`, but
  the only public conversion into a domain `Cipher` takes
  `CipherDetailsResponseModel`. The SDK bridges the two internally with a
  `pub(crate)` trait. (In 2.0.0 there *was* a `TryFrom<CipherResponseModel>`; it
  was removed in 3.0.0.)
- **Our fix:** destructure `CipherResponseModel` into `CipherDetailsResponseModel`
  — they are field-for-field identical apart from `collectionIds` — then use the
  public `TryFrom`. Written as an exhaustive destructure specifically so the
  compiler flags it if that stops being true. **Worked around**
  (`vault/write_service.rs::cipher_from_response`).
- **Proposed upstream fix:** make `PartialCipher` public, or restore
  `TryFrom<CipherResponseModel> for Cipher`.
- **Status:** Open upstream.

#### S4. `CipherResponseModel` omits `collectionIds`, so a naive edit unshares the item
- **Command:** `bw edit item` on an organization item
- **Location:** `bitwarden-api-api/src/models/cipher_response_model.rs`
- **What happens:** `PUT /ciphers/{id}` returns no collection ids. Storing that
  response as-is drops the item's collection membership from local state, so it
  looks unshared until the next full sync — and any subsequent write persists the
  loss.
- **Found by:** upstream's own `cipher_client/edit.rs:573` has a test whose
  comment says `collection_ids must be preserved even though CipherResponseModel
  omits them` — which is what flagged the trap before we hit it.
- **Our fix:** carry the collection ids we sent forward into the stored cipher.
  **Worked around** (`vault/write_service.rs::update_cipher`).
- **Status:** arguably by design in the API; the hazard is that the type system
  doesn't signal it. Worth a doc comment upstream at minimum.

#### S5. `get_sdk_managed_migrations` omits `LocalUserDataKeyState`
- **Command:** every `bw unlock` (and any command that initializes crypto)
- **Location:** `bitwarden-pm/src/migrations.rs:12`
- **What happens:** the list creates tables for `Cipher`, `Folder`,
  `SettingItem`, `OrganizationSharedKey` and `Send` — but not
  `LocalUserDataKeyState`, which `initialize_user_crypto` then tries to write.
  Result: `Unable to initialize local user data key` logged at **ERROR** on every
  unlock. Upstream's own `bw` hits this too, since it uses this exact list.
- **Our fix:** our own migration list adds `Add(LocalUserDataKeyState::data())`,
  keeping the shared entries in the same order so the two stay compatible.
  **Worked around** (`services/sdk.rs::state_migrations`).
- **Proposed upstream fix:** add it to the list.
- **Status:** Open upstream.

#### S6. `export_organization_vault` is `todo!()` and aborts the process
- **Command:** `bw export --organizationid <id>`
- **Location:** `bitwarden-exporters/src/export.rs:52`
- **What happens:** `todo!()` panics. In a CLI that is a process abort with a
  panic message, not an error the user can act on.
- **Our fix:** refuse `--organizationid` up front on both `export` and `import`
  with a clear message, so the panic is unreachable. **Worked around**
  (`commands/`).
- **Status:** Open upstream.

#### S7. `CipherPermissions` is `deny_unknown_fields` and rejects real server data
- **Command:** anything reading a TypeScript-CLI-written `data.json`
- **Location:** `bitwarden-vault/src/cipher/cipher_permissions.rs:10`
- **What happens:** the type is `{delete, restore}` with `deny_unknown_fields`.
  The TypeScript CLI (v2025.11.0) writes `{delete, response, restore}`. Every
  TS-written cipher therefore fails to deserialize — and because these are stored
  as a map, one bad entry fails the *whole* vault read.
- **Significance:** this is the finding that settled the state-format decision.
  The on-disk interop we thought we were protecting **did not exist in either
  direction**, so keeping `data.json` was paying a cost for a benefit we never
  had.
- **Our fix:** none needed after going SDK-native — we no longer read TS vault
  data. **Won't fix** locally.
- **Proposed upstream fix:** drop `deny_unknown_fields`, or add the field.
  `deny_unknown_fields` on types that parse server or foreign-client data is a
  recurring source of these (see also C4).

#### S8. `bitwarden-sensitive-value` does not zeroize and serializes transparently
- **Location:** `crates/bitwarden-sensitive-value`
- **What happens:** presents as a secret-wrapper type but neither zeroizes on
  drop nor redacts on serialize, so adopting it would be a regression on the
  `secrecy` crate we already use.
- **Our fix:** don't adopt. **Won't fix** locally; listed here because the name
  invites exactly the wrong assumption.

---

## bwcli-rs bugs — our code

### Open

#### C27. `bw get template` needs an unlocked vault
- **Command:** `bw get template item`
- **Location:** `bw-cli/src/main.rs` — `needs_unlocked_vault`
- **What happens:** the whole `get` family is classified as needing an unlocked
  vault, but templates are static JSON compiled into the binary. So
  `bw get template item` fails with *"Vault is locked"* when it needs nothing from
  the vault at all. The TypeScript CLI's `get template` requires no session.
- **Found by:** writing a test that wanted a JSON document from a command needing
  no credentials, and finding `get template` could not supply one.
- **Proposed fix:** classify `Get(GetCommands::Template(_))` as not needing an
  unlocked vault, alongside `receive`, which is already excluded for the same reason.
- **Status:** **Open.** Cosmetic in effect — the workaround is to unlock — but it
  makes `bw get template item | bw create item` need a session for the template
  half, which is the pipeline C13 exists to protect.

### Fixed

#### C23. `bw move` collided with the TypeScript CLI's org-share command
- **Command:** `bw move <id> <organizationId> [encodedJson]`
- **Location:** `bw-cli/src/commands/vault.rs`, `bw-core/src/services/vault/write_service.rs`,
  `bw-core/src/services/vault/sync_service.rs`, `bw-core/src/models/vault/sync_response.rs`
- **What happened:** in the TypeScript CLI `move` shares an item into an
  organization (`vault.program.ts:31` registers it as `shareCommand("move",
  false)`; `share` is the *deprecated* alias). Ours read a **folder** id in the
  organization slot, so a TS-compatible script would hand us an organization id
  where we expected a folder.
- **Fix:** `move` is now org-share, implemented on the SDK's public
  `CiphersClient::share_cipher` — which reassigns the item, carries password
  history across, re-encrypts under the organization key, `PUT`s, and updates the
  repository. `share` is accepted as an alias, as upstream does. Our folder move
  became `bw move-to-folder <id> [folderId]`, which is not a TypeScript CLI
  command and says so in its help. **Fixed.**
- **Prerequisite that was missing:** sharing re-encrypts under the organization's
  key, and **nothing ever put organization keys in the key store.** `sync` parsed
  organizations for `bw list organizations` but dropped `profile.organizations[].key`
  on the floor, so `share_cipher` would have failed at encryption time. `sync` now
  extracts those keys and calls `crypto().initialize_org_crypto`, which loads them
  and persists them to `OrganizationSharedKey` for later unlocks. This also means
  organization-owned *items* can decrypt for the first time.
- **Deliberately non-fatal:** unwrapping an organization key needs the user's
  private key, so a locked or partially unlocked vault cannot do it. That failure
  is logged and sync continues — `sync` is how a bad local state gets repaired, so
  it must survive one. The pre-existing sync tests caught this: the first version
  made it fatal and three of them failed with `Missing private key`.
  `share_cipher` instead pre-flights the specific key it needs and says what to do
  if it is absent.
- **Verified end to end** against a real organization ("Rust Test Org"), after an
  earlier round could only cover argument parsing and error paths because the test
  vault had none:
  - `sync` persisted the organization key, and `bw list collections` decrypted
    `Default collection` — the name only decrypts if the key genuinely reached the
    key store, which is stronger evidence than the key merely being stored.
  - `echo '["<collection-id>"]' | bw encode | bw move <id> <org-id>` returned the
    item with `organizationId` and `collectionIds` set, and its username and
    password still correct — so the re-encryption preserved the payload.
  - A **fresh invocation** read it back decrypted, so the repository write and the
    org-key restore-on-unlock both work, not just the in-process state.
  - It survived `bw sync --force`, i.e. the server has it, and it shows up in
    `bw list items` as an organization item.
  - Re-sharing it was refused with "this item already belongs to an organization".
  - Cleanup left the vault at its original 11 items, 1 folder, empty trash.
- **Tests:** `move_shares_into_an_organization`, `share_is_an_alias_for_move`,
  `move_to_folder_accepts_a_missing_folder`, `move_rejects_a_bare_item_and_folder_pair`,
  `the_sync_response_yields_organization_keys`,
  `an_unreadable_organization_key_is_skipped_not_fatal`,
  `a_key_that_cannot_be_unwrapped_does_not_fail_the_sync`, and five on
  collection-id parsing.

#### C25. `bw encode` required an argument instead of reading stdin
- **Command:** `echo '[...]' | bw encode`
- **Location:** `bw-cli/src/commands/tools.rs` — `EncodeCommand`
- **What happened:** ours took a required positional `<DATA>`. The TypeScript
  CLI's `encode` takes **no argument at all** — its description is literally
  "Base 64 encode stdin" — and its own help pipes it into `create`, `edit` and
  `move`. So the documented pipeline failed with a clap usage error.
- **Found by:** running `bw encode | bw move` while verifying C23. The feature was
  unusable in exactly the way its documentation describes.
- **Fix:** `<DATA>` is optional and stdin is read when omitted, with the trailing
  newline trimmed — encoding a shell's newline changes the base64 the next
  command receives. The positional stays, since this CLI shipped with it as the
  only form. **Fixed**, tests `encode_reads_stdin`,
  `encode_still_accepts_an_argument`.

#### C26. String payloads were JSON-quoted, breaking every `$(bw ...)` capture
- **Commands:** `bw get password|username|uri|totp`, `bw generate`, `bw encode`
- **Location:** `bw-cli/src/output/formatter.rs` — `print_human`
- **What happened:** human mode pretty-printed the payload as JSON regardless of
  type, so a string came out **wrapped in double quotes**:
  `PASS=$(bw get password <id>)` yielded `"hunter2"`, quotes included. `bw generate`
  the same. The TypeScript CLI prints string payloads bare — `base-program.ts`:
  for a `string` response, `out = data`, with no JSON encoding.
- **Severity:** the worst of the parity bugs found. It silently corrupts the most
  common scripted use of the CLI, and `--raw` masked it (`print_raw_value` already
  printed strings bare), so anyone who hit it would likely have worked around it
  rather than reported it.
- **Found by:** the `bw encode | bw move` pipeline still failing after C25 —
  `Invalid symbol 34` is a `"`. Chasing one broken pipe found the general bug.
- **Fix:** `print_human` prints `Value::String` bare and keeps pretty-printing
  everything else. **Fixed.**
- **Verified live:** `bw generate` and `bw get password` now emit bare values,
  `bw encode | bw move` runs end to end, and `status`, `list items`, `get item`
  and `get template` still emit parseable JSON.
- **Tests:** `string_output_is_not_json_quoted`,
  `generate_output_is_not_json_quoted`, `document_output_is_still_json`.

#### C28. `bw unlock --raw` printed the instructional blurb, not the session key
- **Commands:** `bw unlock --raw`, `bw login --raw`
- **Location:** `bw-cli/src/commands/auth/vault_ops.rs`, `commands/auth/login.rs`,
  `bw-cli/src/output/`
- **What happened:** `unlock` put its whole human message — *"Your vault is
  unlocked! ... $ export BW_SESSION=..."* — into the response `data` as one
  string, so `--raw` printed all of it. `export BW_SESSION=$(bw unlock --raw)`
  therefore captured a paragraph. The TypeScript CLI's help states the contract
  outright ("Pass `--raw` option to only return the session key") and implements it
  with `MessageResponse.raw` (`unlock.command.ts:100`: `res.raw =
  process.env.BW_SESSION`). `login` had the same shape.
- **Found by:** writing the smoke test in `HANDOFF.md` and using
  `bw unlock --raw` in it — the documented idiom for the single most common setup
  step, which turned out not to work.
- **Fix:** `Response::with_raw()` carries a machine-readable form alongside the
  human one, and `print_raw` prefers it. Applied to `unlock` and both `login`
  paths. Same principle as [C26](#c26-string-payloads-were-json-quoted-breaking-every-bw--capture):
  `--raw` and stdout are for machines, prose is for people. **Fixed**, test
  `unlock_raw_does_not_emit_the_instructional_blurb`.

#### C29. A session key reached a pushed branch as a filename
- **Not a product bug** — a process failure, logged here because it is the most
  serious thing that happened in this work and the near-misses are instructive.
- **What happened:** a file named
  `:BW_SESSION="pQEEAlCZune…"` — the key in the *filename*, the file itself empty —
  was created by a stray `:` turning an `export BW_SESSION="…"` into a shell
  redirect, then swept in by `git add -A`. It rode along in 11 commits from
  `d97821e` and was **pushed to `origin`**.
- **Caught by:** the repository owner reading the branch, not by any check.
- **Why the pre-push scan missed it — two independent near-misses:**
  1. It searched file *contents*: `git grep`, and `git log -p | grep '^\+.*BW_SESSION'`.
     A filename appears only in the `diff --git` header, which does not start with
     `+`, so the pattern could not match.
  2. The tracked-file check was `git ls-files | grep -iE "…|bwsession"` — and
     `bwsession` does not match `BW_SESSION`. It looked like path coverage and was
     not.
  `.gitignore` would not have helped either: it listed `*.bwsession`, not this
  shape. Three layers that each appeared to cover it.
- **Remediation, in the order it matters:**
  1. **Rotate first.** The owner had already logged out and back in, which
     invalidates `session_protected_user_key`, so the leaked envelope was dead
     before the history was touched. Deleting a secret does not un-leak it.
  2. **Rewrite, do not just delete.** Removing the file in a new commit would have
     left it reachable in `d97821e` for anyone cloning. Used
     `git filter-branch --index-filter` over `master..HEAD`, then force-pushed with
     `--force-with-lease`.
  3. Verified the rewrite was surgical: 17 commits before and after, identical
     subjects, and `git diff` between old and new tips showed *only* the removed
     path with no other file changed.
  4. Dropped `refs/original`, expired the reflog, and `gc --prune=now`, so the blob
     is unreachable locally too. **Note:** GitHub may retain the old objects,
     addressable by SHA, until its own GC — which is precisely why step 1 comes
     first.
- **A trap worth knowing:** the first rewrite attempt silently did *nothing* and
  reported `Ref … is unchanged`. The filename begins with `:`, which git parses as
  **pathspec magic**, so `git rm --cached -- ':BW_SESSION="…"'` matched no file.
  `--` stops option parsing but not magic parsing; `:(literal)` is what disables it.
  A no-op rewrite that exits successfully is exactly the failure you would not
  notice.
- **Fix:** `scripts/check-secrets.sh` checks **paths before contents**, in the
  working tree, the index, or every commit in a range. It matches the session-key
  CBOR prefix (`pQEEAl…`), JWT shapes, `BW_SESSION` followed by a long token, and
  the local-state filenames. Verified against the real case: staging a file with
  that exact name fails the check. Also added `user.sqlite`, `*.sqlite` and the
  journal/WAL patterns to `.gitignore` — `user.sqlite` now holds the live tokens
  and the sealed user key, which the old ignore list predated.

#### C24. `bw get org` should be `bw get organization`
- **Command:** `bw get organization <id>`
- **Location:** `bw-cli/src/commands/vault.rs` — `GetCommands::Organization`
- **What happened:** the object was registered as `#[command(name = "org")]`, but
  the TypeScript CLI's `getObjects` list (`vault.program.ts`) spells it
  `organization`. Same class as [C3](#c3-bw-restore-took-a-bare-id-bw-move-could-not-clear-a-folder):
  a TS-compatible script fails at argument parsing, before anything happens.
- **Found by:** diffing our `get`/`list`/`create`/`edit`/`delete` object lists
  against the ones extracted from `vault.program.ts`, once a current checkout was
  available. Not from a report — it would have sat there indefinitely.
- **Fix:** primary name is now `organization`, with `org` kept as a clap alias
  because this CLI shipped with `org` as the only name and breaking it buys
  nothing. **Fixed**, test `get_accepts_the_typescript_organization_object`.
- **Related gaps found in the same pass, features rather than bugs** (tracked in
  the parity matrix, not here): `get notes` and `get send` are missing objects,
  and `archive` and `report` are missing commands.

#### C1. `bw export --format json` emitted invalid JSON
- **Command:** `bw export --format json > vault.json`
- **Location:** `bw-core/src/services/import_export/export/mod.rs`,
  `bw-cli/src/commands/tools.rs`
- **What happened:** `ExportService` wrote the document to stdout itself, then the
  command printed `"Exported 11 item(s)"` after it. Redirecting produced a file no
  JSON parser accepts (`Extra data: line 274 column 2`). With `--response` it was
  worse: the document *and* the response JSON both went to stdout — two documents.
- **Root cause, not just the symptom:** the service owned stdout. Only the command
  knows whether stdout is the data channel for a given invocation, so that
  decision was in the wrong place.
- **Fix:** `export` now *returns* the document in `ExportResult::contents` when
  there is no `--output`, and the command places it: to stdout with the count on
  **stderr**, or inside the JSON under `--response`. A new `Response::silent()`
  suppresses the trailing human output for commands whose payload is already on
  stdout. **Fixed.**
- **Verified live:** `bw export --format json > vault.json` parses cleanly (11
  items, 1 folder) with `Exported 11 item(s)` on stderr; `--response` yields a
  single JSON document with the export under `data.data`.
- **Tests:** `exporting_without_a_path_returns_the_document`,
  `exporting_to_a_path_returns_no_document`, `a_json_export_parses_on_its_own`,
  `export_keeps_status_off_stdout`.

#### C2. Self-hosted URLs were persisted but never read back
- **Command:** any authenticated command against a self-hosted server, run
  without `--server`
- **Location:** `bw-core/src/services/container.rs`, `services/sdk.rs`
- **What happened:** `BASE_URLS` was written at login and by the carry-over, but
  the client was always built from the CLI arguments alone. A later invocation
  without `--server` targeted Bitwarden cloud — **including token renewal, which
  would have sent a self-hosted refresh token to `identity.bitwarden.com`.**
- **Why it was awkward:** the URLs the client needs live in the state database the
  client owns. Chicken and egg.
- **Fix:** split `open_state` from `create_sdk_client_with_state`, so `BASE_URLS`
  can be read before the client is built. Explicit arguments still win; stored
  URLs are the fallback; cloud is the default. `resolve_environment` recovers the
  base URL for the services `Environment` models but `BASE_URLS` does not carry
  (icons, notifications, events), and recognises cloud's own pair rather than
  treating `api.bitwarden.com` as a base to derive from. **Fixed.**
- **Not verified against a real self-hosted server** — I have no instance to test
  against. Covered by three unit tests on the resolution rules
  (`no_urls_resolves_to_cloud`, `the_cloud_url_pair_resolves_back_to_cloud`,
  `a_self_hosted_pair_keeps_both_urls_and_derives_the_base`), which is weaker
  evidence than the live checks elsewhere in this file.

#### C3. `bw restore` took a bare id; `bw move` could not clear a folder
- **Commands:** `bw restore item <id>`, `bw move <id>`
- **Location:** `bw-cli/src/commands/vault.rs`, `bw-cli/src/main.rs`
- **What happened:** ours was `bw restore <id>`; the TypeScript CLI is
  `bw restore <object> <id>` with `item` the only valid object
  (`apps/cli/src/vault.program.ts:380`). Any script written against it failed with
  a clap usage error. Separately, `bw move`'s folder argument was required, so
  there was no way to remove an item from all folders — even though
  `CiphersClient::move_many(ids, None)` supports exactly that, and the code already
  translated a literal `"null"`.
- **Fix:** `restore` gained the object level as a clap subcommand; `move`'s folder
  argument is now optional, with a missing, empty, or `"null"` value all meaning
  no folder. **Fixed.**
- **Verified live:** `bw move <id> <folder>` sets `folderId`, `bw move <id>` clears
  it to `None`, and `bw restore item <id>` un-trashes the item.
- **Tests:** `restore_takes_an_object_argument`, `restore_rejects_a_bare_id`,
  `move_accepts_a_missing_folder`.
- **Follow-on:** verifying the TS CLI's actual syntax revealed that `move` means
  something else entirely there — see [C23](#c23-bw-move-collides-with-the-typescript-clis-org-share-command).

#### C4. `bw list folders` and `bw export` read a store `sync` no longer writes
- **Commands:** `bw list folders`, `bw export`
- **Location:** `bw-core/src/services/vault/mod.rs` — `get_ciphers`, `get_folders`
- **What happened:** step 4 moved `sync` to write ciphers and folders into the
  SQLite state repositories, but these two readers still read the `data.json`
  keys. `list folders` failed with *"Vault not synced"*; `export` (via
  `encrypted_ciphers`) was broken the same way.
- **Why it hid:** `bw list items` goes through `CiphersClient::list`, which reads
  the repository, so the most obvious command worked. **And my own step-4
  verification reported a folder count read straight from the repository rather
  than from `bw list folders`** — the check confirmed the data was there, not
  that the command could reach it.
- **Fix:** both now read the state repository, with an actionable error naming
  `bw sync --force`. **Fixed.** Verified live: `list folders` returns the folder,
  `export` returns 11 items and 1 folder.

#### C5. `TokenManager::refresh_access_token` deadlocked
- **Command:** any command run ~1h+ after login
- **Location:** `bw-core/src/services/api/token_manager.rs` (since deleted)
- **What happened:** the `refresh_state` guard was dropped only on the
  "already refreshing" path; the other path re-locked the same non-reentrant
  mutex while still holding it. Any refresh hung forever.
- **Fix:** scoped the guard, then **deleted the whole module** in step 8 — the
  SDK's middleware serializes renewals behind a mutex correctly. **Fixed.**

#### C6. Token refresh omitted `client_id`
- **Command:** as C5
- **Location:** `bw-core/src/models/api/token.rs` (since deleted)
- **What happened:** `TokenRefreshRequest` had no `client_id`, so the identity
  server answered `invalid_request`. Combined with C5 and C7, **token refresh
  could never have worked** — the CLI simply stopped functioning about an hour
  after login.
- **Fix:** added the field; later obviated by the SDK, which takes `client_id`
  from the persisted login method. **Fixed.** (This is the same failure mode S1
  would have reintroduced from a different direction — hence the test there.)

#### C7. Renewed tokens were never persisted
- **Command:** as C5
- **Location:** `bw-core/src/services/api/token_manager.rs` (since deleted)
- **What happened:** missing `storage.flush()` after writing refreshed tokens, so
  every invocation refreshed again from the stale pair.
- **Fix:** added the flush; later obviated by the SDK. **Fixed.**

#### C8. Vault crypto was dead code at runtime
- **Commands:** every command needing decryption
- **Location:** `bw-core/src/services/` — nothing populated the SDK `KeyStore`
- **What happened:** zero calls to `initialize_user_crypto` or anything else that
  populates the SDK client's key store, yet `cipher_service` called
  `client.vault().ciphers().decrypt(...)` against it. The user key lived only in
  our own `__PROTECTED__` storage. It compiled; it could not work.
- **Fix:** wire crypto initialization, persist the account private key, and prove
  it with a test that asserts encryption *fails* against an uninitialized key
  store. **Fixed.** (Later superseded by `bitwarden-unlock`.)

#### C9. `bw login` failed for every user
- **Command:** `bw login`
- **Location:** `bw-core/src/models/api/auth.rs` — `LoginResponse`
- **What happened:** the server renamed `ResetMasterPassword` to
  `ForcePasswordReset`. Our required `bool` made serde reject the **entire** token
  response: `missing field 'ResetMasterPassword'`. The field is read nowhere.
- **Note:** my first hypothesis — that `Kdf` had moved — was **wrong**. The
  diagnostic added to the error path (C12) is what revealed the real cause, by
  listing the field names the server actually sent.
- **Fix:** aliased and defaulted, along with `expires_in`/`token_type`;
  informational fields must not be able to fail a login. `Kdf`/`KdfIterations`
  are deliberately still required — defaulting them risks silently deriving a
  wrong key. **Fixed**, 3 regression tests.

#### C10. Writes succeeded but reported failure
- **Commands:** `bw create item`, `bw edit item`, folder writes
- **Location:** `bw-core/src/services/vault/write_service.rs`
- **What happened:** POST/PUT responses were deserialized straight into `Cipher`,
  which is `deny_unknown_fields` and rejects the server's `object` field. `bw
  create item` **created the item**, told the user it failed, and skipped the
  cache update.
- **Fix:** five sites moved to the tolerant generated response models plus the
  SDK's `TryFrom`. **Fixed.**

#### C11. `bw edit item` and `bw move` could never have worked
- **Commands:** `bw edit item`, `bw move`
- **Location:** `bw-core/src/services/vault/write_service.rs::update_cipher`
- **What happened:** `revision_date` was set to `now()` before encrypting, but
  `CipherRequestModel` derives `lastKnownRevisionDate` from it and the server uses
  that for optimistic concurrency. Every edit was rejected as out of date.
- **Fix:** the server owns that field; we no longer touch it. **Fixed.**

#### C12. The error handler hid C9, C10 and C11
- **Location:** `bw-core/src/services/api/client.rs::extract_error_message`
- **What happened:** it deserialized into a struct whose fields are all `Option`,
  so parsing *always* succeeded and returned the `"Unknown error"` default —
  making the raw-body fallback unreachable. It also didn't know about
  `validationErrors`.
- **Significance:** this is the one that mattered most. Three real bugs were
  reporting themselves as "Unknown error"; fixing the reporter is what made them
  findable.
- **Fix:** split into a pure, testable `describe_error_body` with
  `validationErrors` support and a raw-body fallback. Response-deserialization
  failures now report the server's top-level **field names** — never values,
  since token responses carry credentials. **Fixed**, 6 tests.

#### C13. `bw get template item | bw create item` was rejected by our own parser
- **Command:** exactly that pipe
- **Location:** `bw-cli/src/commands/input.rs::parse_item_input`
- **What happened:** `invalid type: null, expected a sequence` on
  `"collectionIds": null`. Input was deserialized straight into `CipherView`, a
  domain type whose server-owned fields (`creationDate`, `revisionDate`, `edit`,
  `viewPassword`, `organizationUseTotp`) are required.
- **Fix:** a lenient `CipherInput` DTO that defaults everything and fills
  server-owned fields — the shape upstream already uses for sends
  (`SendJsonInput`). **Fixed**, guarded by `test_item_templates_are_parseable`,
  which round-trips every template. Not a 3.0 regression; broken at 2.0.0 too.

#### C14. A stale session silently returned garbage
- **Commands:** all vault commands
- **Location:** `bw-cli/src/main.rs`
- **What happened:** unlock failure was swallowed at `debug` level, so a stale
  `BW_SESSION` produced 11 items with **empty names** rather than an error.
- **Fix:** a `needs_unlocked_vault(&Commands)` classification table makes unlock
  failure fatal for vault commands (`receive` excluded — it is anonymous).
  **Fixed.**

#### C15. `UnlockClient::unlock` leaves the client with no user id
- **Commands:** `bw create item`, `bw edit item` after `bw unlock`
- **Location:** `bw-core/src/services/sdk_session.rs::unlock_with_session`
- **What happened:** `unlock` restores the keys but never calls `init_user_id` —
  only `initialize_user_crypto` and `load_from_state` do. Decryption doesn't need
  it, so **reads looked perfectly fine while every create and edit failed** with
  *"Client User Id has not been set"* (an `EncryptionContext` records who
  encrypted the item).
- **Fix:** set it from persisted state, mirroring `load_from_state`. **Fixed**,
  with a regression test asserting encryption works after unlocking — the exact
  operation that failed. (Arguably an SDK gap too; the workaround is small and
  local enough to keep here.)

#### C16. Tracing wrote to stdout
- **Command:** `bw status | jq`, any piped command
- **Location:** `bw-cli/src/main.rs`
- **What happened:** log lines interleaved with JSON on stdout, corrupting it.
- **Fix:** tracing to **stderr**. **Fixed.** (C1 is the same class, still open.)

#### C17. A non-UUID user id failed deep inside the SDK
- **Command:** `bw login`, `bw unlock`
- **Location:** `bw-core/src/services/sdk_session.rs`
- **What happened:** a non-UUID user id silently became `None`, then failed much
  later with the opaque *"Unable to initialize local user data key"*.
- **Fix:** validated up front with a message naming the bad value. **Fixed**,
  test `a_non_uuid_user_id_is_rejected_up_front`.

#### C18. Error chains were truncated to their outermost message
- **Location:** `bw-core/src/services/auth/auth_service.rs` and others
- **What happened:** `e.to_string()` on an `anyhow::Error` reports only the outer
  context, hiding the actual cause.
- **Fix:** `{:#}` throughout the error-mapping paths. **Fixed.**

#### C19. Importing an empty file reported success
- **Command:** `bw import <format> <empty-or-header-only-file>`
- **Location:** `bw-core/src/services/import_export/`
- **What happened:** zero parsed items was returned as a successful import of
  nothing.
- **Fix:** an explicit error. **Fixed.**

#### C20. Import validation errors lost their detail
- **Command:** `bw import`
- **Location:** `bw-core/src/services/import_export/` — `ImportError::ValidationError`
- **What happened:** it carried only a count (*"Validation failed with 1
  error(s)"*); field and line detail went to stderr only, so `--response`
  consumers got nothing actionable.
- **Fix:** carries `first_error`. **Fixed.**

#### C21. `bw sync` reported "not authenticated" for an existing login
- **Command:** every authenticated command, immediately after step 8
- **Location:** `bw-core/src/services/state_import.rs` (the fix)
- **What happened:** moving tokens into SQLite left every existing install's
  tokens stranded in `data.json`, where nothing read them any more. A deliberate
  break, but one that needed a migration path rather than "run `bw login` again".
- **Fix:** the startup carry-over (step 5/9). **Fixed**, and verified against a
  real install: `unauthenticated` → `locked`, then `bw sync` renewed from the
  carried-over refresh token and succeeded.

#### C22. Identity mix-up risk in the carry-over
- **Command:** first run after upgrading, with multiple accounts in `data.json`
- **Location:** `bw-core/src/services/state_import.rs::resolve_user`
- **What happened:** the first version chose the account by `activeAccountId`.
  With SQLite already holding account A's cryptographic state and `data.json`
  pointing at account B, that would have **paired B's tokens with A's crypto
  state**.
- **Found by:** the real install had refresh tokens for two accounts and
  `activeAccountId: null`, which made the selection rule worth thinking about
  properly rather than just relaxing.
- **Fix:** selection order is now the user id SDK state already names →
  `activeAccountId` → sole token holder; ambiguity declines rather than guesses.
  **Fixed**, 3 tests including the mix-up case directly.

---

## Test-fixture and process mistakes (mine)

Not product bugs, but each one produced a wrong result I initially believed, so
they are worth the same scrutiny.

- **Invalid baseline comparison.** To compare test failures before/after, I ran
  `git stash` — which also reverted the uncommitted `Cargo.toml` version bump, so
  the build died at dependency resolution and reported 1 "error" instead of 44.
  Redone stashing only `crates/`, confirming 44 pre-existing failures on both
  sides.
- **Verified the data, not the command.** See C4. Reporting a repository row count
  as evidence that `bw list folders` worked is the specific mistake; the lesson is
  to assert through the user-facing path.
- **Fixture crypto errors:** generating `Key` and `PrivateKey` from two
  independent calls (different random user keys, so crypto init failed); reusing
  one client for both login and unlock (double crypto initialization);
  `UserKey::new(user_key)` where `make_user_key()` already returns a `UserKey`.
- **A structurally invalid `EncString` fixture** made the importer's private-key
  carry-over look like a code failure. It also exposed a real gap: an unparseable
  key was being **silently dropped**. Now warned about, since the user would
  otherwise get a migration that syncs but cannot unlock, with nothing pointing at
  why.
- **Two stale entries in the code-quality doc:** "Unsafe Mutex Unwrap" was already
  fixed, and "Unsafe Unwrap on Array Slicing" was not a defect (provably
  infallible). Annotated rather than reported as fixed.
- **A misdiagnosis I corrected:** I first said the unreadable `permissions.response`
  data was stale bwcli-rs output. It was **TypeScript-CLI-written** — which is
  worse, and reframed the whole interop question (S7).
- **Asserted a TS CLI command's semantics from memory.** C3 originally claimed
  `bw move`'s only problem was a required-vs-optional argument. In the TypeScript
  CLI, `move` is the org-share command and takes an *organization* id — a name
  collision with different meaning, which is a much bigger problem than the one I
  wrote down (now C23). The local `Bitwarden/clients` checkout settles questions
  like this in seconds; parity claims should be read out of it, not recalled.
- **Three of these were found by using the CLI as its own documentation
  describes.** C25 and C26 came from running `bw encode | bw move`, straight out of
  the TypeScript CLI's help; C28 came from writing `export
  BW_SESSION=$(bw unlock --raw)` into the handoff doc. Every one of those is a
  first-five-minutes command, and none had a test. Walking the documented happy
  path end to end finds more than adding cases to what already passes.
- **Then trusted a checkout without checking its age.** Having been told the
  checkout "could be very old", re-reading it at `v2026.8.0` found three further
  errors in the parity matrix — `device-approval` listed as a missing OSS command
  when it does not exist there at all, and `archive` and `report` missing from the
  matrix entirely — plus C24. Both the *fact* and the *freshness of the source*
  need checking; the matrix now records the commit it was derived from so the
  question is answerable next time.

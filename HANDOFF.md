# Picking this up on another machine

Everything needed to get from a bare checkout to a working, verified build of the
SDK-3.0 migration branch. Written 2026-08-23; the pinned revisions below are what
the current work was verified against.

## 1. Push what is here first

**This branch is not on the remote yet.** From the machine you are leaving:

```bash
cd ~/Source/repos/bwcli-rs
git status                              # expect a clean tree
git push -u origin sdk-3.0-migration
```

The branch is a series of commits ahead of `master`, ending with the one that adds
this file. `git log --oneline master..sdk-3.0-migration` is the index; the commit
messages carry the reasoning and the live-verification results.

Nothing sensitive is tracked, and `.gitignore` covers `data.json`, `user.sqlite`
and `bw-data/`. Worth re-checking before you push, because these hold live
credentials:

```bash
git ls-files | grep -iE "sqlite|data\.json|bw-data|bwsession"   # must print nothing
```

## 2. Three repositories, and the two siblings are load-bearing

| Repo | Path | Pin | Why |
|---|---|---|---|
| `bwcli-rs` | `~/Source/repos/bwcli-rs` | branch `sdk-3.0-migration` | this project |
| `sdk-internal` | `~/Source/repos/sdk-internal` | tag **`rust-v3.0.0`** (`9794da5`) | **path dependency** — the build fails without it |
| `Bitwarden/clients` | `~/Source/repos/Bitwarden/clients` | `cce8a34`, CLI `v2026.8.0` | source of truth for TypeScript-CLI parity |

`Cargo.toml` uses **relative path dependencies** — `path = "../sdk-internal/crates/..."`
— so the layout matters. `sdk-internal` must be a sibling of `bwcli-rs`:

```
~/Source/repos/
├── bwcli-rs/
├── sdk-internal/
└── Bitwarden/clients/
```

Put it elsewhere and every `bitwarden-*` dependency fails to resolve. If your
other machine uses a different root, either mirror this layout or add a
`[patch]`/path override — do **not** edit the paths in `Cargo.toml`, since that
would land in the diff.

```bash
mkdir -p ~/Source/repos/Bitwarden && cd ~/Source/repos

git clone https://github.com/dasien/bwcli-rs.git
git clone https://github.com/bitwarden/sdk-internal.git
git clone https://github.com/bitwarden/clients.git Bitwarden/clients

cd bwcli-rs      && git checkout sdk-3.0-migration
cd ../sdk-internal && git checkout rust-v3.0.0        # detached HEAD, deliberately
```

### The SDK pin is not optional

`sdk-internal` is checked out at the **tag `rust-v3.0.0`, detached HEAD** — not
`main`. `Cargo.toml` also pins `version = "=3.0.0"` on every crate, so a
`sdk-internal` on `main` whose crates have moved past 3.0.0 fails at dependency
resolution rather than compiling against something unexpected. That is the
intended behaviour: the 2.0→3.0 upgrade broke on an exhaustive struct literal, and
the pin makes such a break loud.

If you want to move the SDK forward, do it as its own commit that also updates the
`=3.0.0` pins, so a bisect can tell an SDK bump from a CLI change.

### The clients checkout is for reading, not building

Never needs `npm install`. It exists so parity claims can be *read out of the
source* rather than recalled. The files that matter:

```
apps/cli/src/program.ts              login, logout, lock, unlock, sync, status,
                                     generate, encode, config, completion,
                                     update, sdk-version
apps/cli/src/vault.program.ts        list, get, create, edit, delete, restore,
                                     move, confirm, import, export, share,
                                     archive — and the object lists
apps/cli/src/base-program.ts         how responses are printed (see C26)
apps/cli/src/tools/send/send.program.ts
apps/cli/src/serve.program.ts, dirt/report.program.ts
```

Two bugs came from asserting this CLI's behaviour from memory instead (`BUGLIST.md`
C23, and the parity-matrix corrections). **Re-derive the matrix from the checkout;
do not hand-edit it.** Record the commit you read when you do — the current matrix
does.

## 3. Toolchain

- Rust **1.92.0** on the verified machine; `rust-version = "1.88.0"` is the floor.
- Edition 2024. There is no `rust-toolchain.toml`, so a stable rustup is enough.
- No system dependencies beyond what `rustls`/`reqwest` need. No OpenSSL.
- `sqlite3` on `PATH` is handy for poking at state, not needed to build.

```bash
cd ~/Source/repos/bwcli-rs
cargo build --workspace     # first build pulls the whole SDK graph; expect minutes
cargo test --workspace      # expect 262 passing, 2 ignored, 0 failing
```

If tests fail before you have touched anything, suspect the SDK pin first.

### Disk, and why the dev profile looks the way it does

`target/` was **28 GiB** before tuning and is ~2 GB now. Two causes, both handled
in `Cargo.toml`, and both easy to undo by accident:

- `[profile.dev] debug = "line-tables-only"` and `[profile.dev.package."*"] debug = false`.
  Full DWARF is roughly a third of every artifact, and this workspace links the
  SDK graph into ~12 binaries. Set `debug = 2` temporarily if you need variable
  inspection, then put it back.
- **`bw-core/Cargo.toml` deliberately has no `tokio` dev-dependency.** Adding one
  with `features = ["test-util"]` — which is *not* in tokio's `full` — changes
  tokio's feature set for the test build, changing its fingerprint, forcing a
  second compilation of every crate above it. That alone roughly doubled `target/`.
  There is a comment saying so; leave it.

`cargo clean` is the cure if it creeps back up.

## 4. Local state, and the credentials in it

On macOS, `~/Library/Application Support/Bitwarden CLI/`
(Linux `~/.config/Bitwarden CLI/`; override with `BITWARDENCLI_APPDATA_DIR`):

| File | Contents |
|---|---|
| `user.sqlite` | **Live credentials.** `authentication_tokens` (access + refresh), `user_login_method` (the API-key client secret when one is used), `session_protected_user_key`, `account_crypto_state`, `base_urls`, plus the `Cipher`/`Folder`/`Send` tables |
| `data.json` | Legacy store. Still holds sends, collections, organizations, last-sync, KDF config, the account registry — and pre-migration tokens |

**None of this may be committed.** Both are gitignored, including a bare
`*.sqlite`, because `BITWARDENCLI_APPDATA_DIR` can put them anywhere. Note
`storage/path.rs` resolves appdata to `./bw-data` when that directory exists, so
running the binary from the repo root drops state into the working tree.

State does **not** transfer between machines. On the new one, `bw login` and
`bw unlock`; the session key is machine-local by construction — it seals the user
key into that machine's `user.sqlite`.

If you copy `data.json` over from an old install, the startup carry-over
(`services/state_import.rs`) will pull the login into `user.sqlite` on the next
command. It is a no-op once migrated. `BUGLIST.md` C21/C22 explain the rules,
including why it refuses to guess between two accounts.

### Test vault

The verification in the commit messages was run against a live test account —
**no real credentials, per the owner** — in this state, which is worth restoring
to after any destructive test:

- 11 items, 1 folder (`Test Folder - Rust CLI (Renamed)`), empty trash
- organization **"Rust Test Org"** (`2c5dcda7-…`) with one collection
  (`Default collection`, `6412aa89-…`) — needed for the `bw move` org-share path,
  which cannot be tested without an organization
- 0 sends

Never commit a session key or token. Export `BW_SESSION` in the shell; do not put
it in a file in the repo.

## 5. Read these, in this order

| File | What it gives you |
|---|---|
| `docs/sdk-3.0-migration.md` | why the migration went the way it did: the SDK-native decision, phases 1–10, the parity matrix, the per-stub SDK-capability survey, and the "do not adopt" list with reasons |
| `BUGLIST.md` | every defect found, SDK and ours, open and fixed — 1 open, 28 fixed, 8 SDK. Read the header: ids are stable, corrections are noted in place |
| `git log master..sdk-3.0-migration` | the commit messages carry the reasoning and the live-verification results |

Two themes to absorb before trusting the suite:

- **A passing test suite proved almost nothing here.** No test crosses the
  network, so hand-rolled API models and error paths drifted from the server
  unchecked. Several bugs were found only by running the binary against the real
  server.
- **A working read path repeatedly masked a broken write path.** Verifying reads
  is much weaker evidence than it feels like. A repository row count was once
  reported as evidence that `bw list folders` worked, when the command was broken
  (`BUGLIST.md` C4) — assert through the user-facing path.
- **Walking the documented happy path finds more than adding test cases.** C25,
  C26 and C28 were all found by running commands exactly as the TypeScript CLI's
  own help describes them (`bw encode | bw move`,
  `export BW_SESSION=$(bw unlock --raw)`). All three are first-five-minutes
  commands; none had a test.

## 6. Where to start working

State: the SDK-native sequence is complete — SQLite state, session lifecycle,
tokens, vault reads and writes, the `data.json` carry-over, and org-share.

Next, in the value order the SDK survey implies:

1. **Attachments** — `create`/`get`/`delete attachment`. Three stubs, high-level
   SDK support that is genuinely reachable, and it de-risks file Sends by sharing
   the upload machinery. See the survey in `docs/sdk-3.0-migration.md`.
2. **`get organization` and `get collection`** — stubbed for no remaining reason;
   both are lookups over data already decrypted locally.
3. **`edit item-collections`** — one call to `CiphersClient::bulk_update_collections`.
4. **C27** (the only open bug) — `bw get template` needlessly demands an unlocked
   vault, which makes `get template | create item` need a session for the template
   half.
5. Then the larger absences: `archive`/restore-from-archive, `get notes`,
   `list org-collections`/`org-members`, `report`, `serve`, `completion`.

Also still pending from the original plan: the bulk import endpoint instead of
per-item creates, file Sends, and email-OTP Sends on `receive`.

## 7. A quick smoke test to confirm the setup

```bash
cargo test --workspace                 # 262 pass, 2 ignored
cargo build --release
./target/release/bw --version
./target/release/bw status             # "unauthenticated" on a fresh machine

# then, against the test account:
./target/release/bw login
export BW_SESSION="$(./target/release/bw unlock --raw)"
./target/release/bw sync
./target/release/bw list items | jq length          # 11
./target/release/bw list organizations | jq -r '.[].name'   # Rust Test Org
./target/release/bw get password <id>              # must print bare, no quotes (C26)
```

That last one is the cheapest check that the output layer is behaving: a quoted
value means C26 has regressed and every `$(bw ...)` capture is wrong.

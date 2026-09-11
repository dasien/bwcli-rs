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

> Building needs a Rust toolchain, which a fresh machine will not have — see
> section 3 if `cargo` is not found.

| Repo | Path | Pin | Why |
|---|---|---|---|
| `bwcli-rs` | `~/Source/repos/bwcli-rs` | branch `sdk-3.0-migration` | this project |
| `sdk-internal` | sibling of `bwcli-rs` | commit **`26112cf3`** | **path dependency** — the build fails without it |
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
cd ../sdk-internal && git checkout 26112cf3           # detached HEAD, deliberately
```

### The SDK pin is not optional

`sdk-internal` is checked out at **commit `26112cf3`, detached HEAD** — not
`main`, and deliberately *not* a tag.

**Do not use the `rust-v3.0.0` tag.** It is not where it looks: locally it
resolves to `7fd530e4` (May 2026), which predates `bitwarden-unlock` entirely, so
dependency resolution fails before anything compiles. The commit SHA is the
trustworthy pin; the tag is not.

`Cargo.toml` pins `version = "=3.0.0"` on every crate, so an `sdk-internal` whose
crates have moved past 3.0.0 fails at dependency resolution rather than compiling
against something unexpected. That is intended: the 2.0->3.0 upgrade broke on an
exhaustive struct literal, and the pin makes such a break loud.

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

### Install it first — `cargo: command not found` is the expected fresh-machine state

```bash
# Is it installed but not on PATH, or not installed?
ls ~/.cargo/bin/cargo
```

**If that file exists, it is a PATH problem.** On macOS the shell is zsh, and zsh
does **not** read `~/.profile` — which is where the rustup installer often writes
its PATH line. Fix it for new shells and for the current one:

```bash
echo '. "$HOME/.cargo/env"' >> ~/.zshrc
. "$HOME/.cargo/env"
```

(That is exactly how the verified machine is set up, via `~/.profile` there.)

**If it does not exist, install rustup:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
rustup default stable
```

On a fresh Mac you also need a linker, or the first build fails at `cc` rather
than at `cargo`:

```bash
xcode-select --install     # no-op if already present
```

### Versions

- Rust **1.92.0** on the verified machine; `rust-version = "1.88.0"` is the floor.
- Edition 2024. There is no `rust-toolchain.toml`, so `rustup default stable` is
  enough — nothing here needs nightly.
- No system dependencies beyond what `rustls`/`reqwest` need. **No OpenSSL**, which
  is deliberate: `reqwest` is configured `default-features = false` with
  `rustls-tls`, so there is no system TLS library to install or mismatch.
- `sqlite3` on `PATH` is handy for poking at state; not needed to build. macOS
  ships it.

```bash
rustc --version && cargo --version    # confirm before going further
```

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

- 12 items, 1 folder (`Test Folder - Rust CLI (Renamed)`), empty trash
- one of those 12 is **`ATTACHMENT TEST - delete me`**
  (`49d227ce-281e-41f8-913b-b4b00001cec4`), deliberately kept: it is the only
  **organization-owned** item, and org ownership is what gets past the personal
  premium gate on attachments. Recreating it means `create item` then `move` into
  the org, and an item cannot be un-shared from an org afterwards — so it is
  cheaper to keep than to remake. Leave it unless you are deliberately resetting.
- organization **"Rust Test Org"** (`2c5dcda7-…`) with one collection
  (`Default collection`, `6412aa89-…`) — needed for the `bw move` org-share path,
  which cannot be tested without an organization
- 0 sends

Never commit a session key or token. Export `BW_SESSION` in the shell; do not put
it in a file in the repo.

### Do this first on a new clone — hooks are not cloned

```bash
git config core.hooksPath scripts/hooks
```

**This is not optional.** Git does not clone `.git/hooks`, so a fresh checkout has
no protection until you run that line. The hooks live in `scripts/hooks/` so they
are versioned; `core.hooksPath` is what activates them.

- `pre-commit` refuses to commit a credential-shaped path or staged blob. It is
  the gate that would have stopped C29, where a stray file was swept in by
  `git add -A`.
- `pre-push` re-checks the commits being pushed, because history can arrive by
  rebase, amend, merge or cherry-pick without a commit of its own — and publishing
  is the step that cannot be undone.

Both are verified against the real leak: creating that file, `git add -A`, and
committing is blocked, and a commit forced past `pre-commit` with `--no-verify` is
blocked at push.

Run it by hand any time:

```bash
scripts/check-secrets.sh              # working tree and index (warns on strays)
scripts/check-secrets.sh --staged     # what pre-commit runs
scripts/check-secrets.sh master..HEAD # every commit on the branch
```

**Prefer `git add <path>` to `git add -A`.** The hooks make a sweep safe rather
than fatal, but `-A` is what turned a shell typo into a published credential.

It checks **paths before contents**, which is the lesson from `BUGLIST.md` C29: a
session key once reached a pushed branch as a *filename* (`:BW_SESSION="pQEEAl…"`,
from a stray `:` turning an `export` into a redirect, then `git add -A`), and a
contents-only scan cannot see that.

If a secret does reach a pushed branch: **rotate it first** (`bw lock` invalidates
every outstanding session), then rewrite history — deleting it in a new commit
leaves it reachable forever. `git filter-branch --index-filter` over `master..HEAD`
then `git push --force-with-lease`. If the path begins with `:`, use a
`:(literal)` pathspec, or git treats it as pathspec magic and the rewrite silently
does nothing while reporting success.

## 5. Read these, in this order

| File | What it gives you |
|---|---|
| `docs/sdk-3.0-migration.md` | why the migration went the way it did: the SDK-native decision, phases 1–10, the parity matrix, the per-stub SDK-capability survey, and the "do not adopt" list with reasons |
| `BUGLIST.md` | every defect found, SDK and ours, open and fixed — 2 open, 33 fixed, 9 SDK. Read the header: ids are stable, corrections are noted in place |
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

1. **Attachments** — **done and verified live on 2026-08-27.** `create`/`get`/
   `delete attachment` in `bw-core/src/services/vault/attachment_service.rs`.
   The hand-rolled upload transport (`BUGLIST.md` S9) is proven: `create` then
   `get --raw` returns byte-identical content for a text file, a 4 KiB random
   binary and a multibyte UTF-8 file. Filename and substring lookup, ambiguity
   refusal, all three `--output` forms with 0600/0700 modes, and delete-by-id
   were all exercised against the real server. The run found C34.
   Still not covered: the orphaned-slot rollback, which needs an upload to fail
   after the slot is created — no way to force that from outside.
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
./target/release/bw list items | jq length          # 12 (see the fixture note above)
./target/release/bw list organizations | jq -r '.[].name'   # Rust Test Org
./target/release/bw get password <id>              # must print bare, no quotes (C26)

# attachments — implemented but never yet run against a real server:
ITEM=<item-id>
echo "hello attachment" > /tmp/att.txt
./target/release/bw create attachment --file /tmp/att.txt --itemid "$ITEM"
./target/release/bw get item "$ITEM" | jq '.attachments'          # expect one entry
./target/release/bw get attachment att.txt --itemid "$ITEM" --raw # expect the text back
./target/release/bw get attachment att.txt --itemid "$ITEM" --output /tmp/out/
./target/release/bw delete attachment <attachment-id> --itemid "$ITEM"
```

The attachment round trip is the one that matters: `create` then `get --raw` must
return the same bytes, which is the only check that proves the upload transport
and the encryption agree. Restore the test vault afterwards — the account is
documented above as having no attachments.

That last one is the cheapest check that the output layer is behaving: a quoted
value means C26 has regressed and every `$(bw ...)` capture is wrong.

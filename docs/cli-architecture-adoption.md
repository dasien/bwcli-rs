# Adopting three design choices from `crates/bw`

Bitwarden's own Rust CLI (`../sdk-internal/crates/bw`) is a separate effort from
this one and far less complete — 18 `todo!()`s, no vault writes at all. But three
of its structural choices are better than ours, and each of them makes a bug we
actually shipped *impossible to write*.

That is the criterion for this document. Not "theirs is tidier" — every item here
is traceable to a numbered entry in `BUGLIST.md`.

| Their choice | Our equivalent | Bugs it would have prevented |
|---|---|---|
| `Result<CommandOutput>` — failure is `Err` | `Ok(Response::error(..))` | **C31** |
| `CommandOutput` + one renderer | `Response` + per-command `println!` | **C26**, **C28** |
| Typestate auth on each command | `needs_unlocked_vault` match | **C27** (still open) |

Three of those four were found by *running the binary*, not by testing. The suite
was green through all of them. Fixing the shapes is how that stops recurring.

## Scale

- 25 command entry points (`execute_*`)
- 139 `Response::` construction sites — 64 of them `Response::error`
- 14 files reference `Response`
- 13 hand-threaded `global_args.raw` / `.silent` / `with_raw` sites

Large but mechanical, and guarded by 278 tests including integration tests that
assert on real stdout.

---

## Phase 1 — `Result<CommandOutput>`: make C31 unrepresentable — **DONE**

**The defect this closes.** A handled failure is currently `Ok`:

```rust
Err(e) => Ok(Response::error(e.to_string()))
```

`main` matched only on `Result::Err`, so every failing command exited 0 (C31).
That is fixed, but the *shape that caused it* is still there: 64 sites still
report failure as success-with-an-error-inside, and nothing stops the 65th from
reintroducing it.

**Target.** Failure is `Err`. There is no other way to express it.

```rust
pub type CommandResult = anyhow::Result<CommandOutput>;

// before
Err(e) => Ok(Response::error(e.to_string()))
// after
Err(e) => Err(e.into())          // or bail!("...")
```

`anyhow`, not `color_eyre` — we already use `anyhow` in every handler signature,
and `crates/bw` pulls `color_eyre` in largely for its panic hooks. Swapping error
libraries is a separate argument; don't bundle it.

**Constraints that must not regress.**

- `--response` must still emit `{"success":false,"message":"..."}` on failure.
  The renderer builds that from the `Err`; no command constructs it.
- `--cleanexit` must still force exit 0.
- Error *text* must be preserved verbatim. Integration tests assert on it, and
  users match against it.

**Done when:** `Response::error` no longer exists, and the suite still passes.

**Outcome.** `Response` is now a struct with no error variant; 63 call sites
became `Err(anyhow::Error::msg(..))`; handlers return `CommandResult`. The dead
`error.rs` — whose `into_response` was the literal C31 factory, converting
business errors into `Ok(Response::error(..))` — was deleted; nothing used it.

Centralising the renderer immediately exposed two further defects that the
scattered version hid, both now fixed and logged: **C32** (`--response` printed
*nothing* on a locked vault, because pre-flight checks bypassed the renderer)
and **C33** (errors prefixed `Error:`, which the TypeScript CLI never emits).
281 tests pass, up from 278.

---

## Phase 2 — `CommandOutput`: make the C26/C28 class unrepresentable — **DONE**

**The defect this closes.** Commands do their own I/O today:

```rust
if global_args.raw {
    println!("{}", password);
    Ok(Response::success_message(""))
} else {
    Ok(Response::success(password))
}
```

That branch is copy-pasted across `get username|password|uri|totp`. Each copy is
a chance to get it wrong, and two of them were:

- **C26** — string payloads JSON-quoted, so `$(bw get password x)` captured
  `"hunter2"` *with the quotes*.
- **C28** — `unlock --raw` printed the whole instructional blurb instead of just
  the key, breaking `export BW_SESSION=$(bw unlock --raw)`.

Both are the same bug: the decision about *how* to render lives in 25 places
instead of one.

**Target.** Commands return a value and never touch stdout.

```rust
pub enum CommandOutput {
    /// A string that is the payload. Printed bare under `--raw` — no quotes.
    Plain(String),
    /// Structured data. Serialized by the renderer.
    Object(Box<dyn erased_serde::Serialize>),
    /// Prose for humans, with a different `--raw` form. This is C28's shape.
    Message { human: String, raw: Option<String> },
    /// Arbitrary bytes straight to stdout — attachment downloads.
    Bytes(Vec<u8>),
    /// Nothing to print; the command already owns stdout (`bw export`).
    Silent,
}
```

`Bytes` is worth calling out: `save_attachment` currently does its own
`stdout().write_all`, which is the last place a command performs I/O. Attachments
are arbitrary binary, so this variant must stay bytes and never become `String`.

**Deliberately not adopted:** their `Output::{YAML,Table,TSV}`. The TypeScript CLI
has no such flag and parity is the goal, so adding formats would be divergence,
not adoption. `Object` leaves the door open if that changes.

**Done when:** no `println!` outside `output/`, and `raw`/`silent` appear in the
renderer only.

**Outcome.** Both hold. `Response` (a struct of four independent `Option`s, most
combinations meaningless) is now the `CommandOutput` enum; commands no longer
call `println!` or touch stdout at all. `--raw`/`--silent` live only in the
renderer, with one deliberate exception: `save_attachment` still reads `raw` to
decide *file vs stdout*, which is destination rather than formatting and is
exactly what `BW_RAW` controls in the TypeScript CLI (`utils.ts:154`).

The survey found **C35** before a line was changed: all four hand-rolled `--raw`
branches printed the value *and* returned an empty message, so `--raw` emitted a
trailing blank line. `$(...)` strips trailing newlines, which is why nobody had
noticed. The branches were redundant — `Plain` already prints bare in both modes
— so deleting them fixed the bug and removed the duplication that caused it.

Two divergences from the TypeScript CLI were found and **deliberately not fixed
here**, because they are behaviour changes rather than refactoring: **C36** (we
pretty-print by default; the TypeScript CLI is compact unless `--pretty`, and its
`--raw` does not affect JSON at all) and **C37** (`bw export` item order is
non-deterministic). Both are logged for an explicit decision.

Verified by differential run against the post-Phase-1 binary: 16 of 17 command
forms byte-identical, the one difference being `get template --raw` becoming
compact — intended, and closer to the TypeScript CLI. Attachment and export
`Bytes` paths re-verified live, including a 4 KiB binary round trip. 282 tests
pass.

**A limitation of that harness worth recording:** it captures output with
`$(...)`, which strips trailing newlines — so it *cannot* detect C35-class bugs.
C35 was confirmed with `od -c` and `wc -l` instead. A differential harness is
only as good as its comparison; this one normalises exactly the whitespace that
one real bug lived in.

---

## Phase 3 — typestate auth: close C27 structurally

**The defect this closes.** Auth requirements live in one hand-maintained match:

```rust
fn needs_unlocked_vault(command: &Commands) -> bool {
    match command {
        List(_) | Get(_) | Create(_) | ... => true,
        Login(_) | Logout(_) | ... => false,
    }
}
```

The comment above it says it is explicit "so that adding a command forces a
deliberate choice" — a reasonable instinct, but it only forces a choice, not a
*correct* one. **C27 is that list being wrong**: the whole `Get(_)` family is
marked as needing an unlocked vault, but `get template` returns static JSON
compiled into the binary. So `bw get template item | bw create item` demands a
session for the half that needs nothing.

Note C27 has sat open precisely because the cheap fix — special-casing
`Get(GetCommands::Template(_))` — makes the match *more* intricate and no more
trustworthy. The structural fix is the one worth doing.

**Target.** Each command declares what it needs; the dispatcher extracts it once,
and a command that needs an unlocked vault is *handed* one.

```rust
trait BwCommand {
    type Client: ClientState;   // AnyState | LoggedIn | Unlocked
    async fn run(self, client: Self::Client) -> CommandResult;
}
```

`get template` declares `AnyState` and cannot demand a session. A command needing
crypto declares `Unlocked` and receives an already-unlocked client — so it cannot
forget to check, and cannot check redundantly.

This is the largest and least mechanical of the three. It is also the only one
that is optional: Phases 1 and 2 stand alone and deliver most of the value. If
effort runs short, stop after Phase 2 and fix C27 with the special case.

**Done when:** `needs_unlocked_vault` is gone and C27 closes without a special case.

---

## Sequencing

1 → 2 → 3. Phases 1 and 2 are one refactor split for reviewability: `CommandOutput`
is the `Ok` variant of Phase 1's `Result`, so Phase 1 lands with a placeholder
`CommandOutput::Object` and Phase 2 fills in the variants. Phase 3 comes last,
because rewriting dispatch is easier once all 25 handlers share a signature.

Each phase is independently shippable and must leave the suite green.

## What this does not fix

Every bug above was found by running the binary, and **no test in this repo
crosses the network**. Restructuring makes a class of bug unwritable; it does not
make the suite trustworthy. The unverified attachment upload (`BUGLIST.md` S9)
stays unverified through all three phases.

Worth stating plainly because the risk is real: a large refactor that leaves the
tests green *feels* like proof, and here it would not be. Re-run the live checks
in `HANDOFF.md` §7 after Phase 2, when output handling has moved.

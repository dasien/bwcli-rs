//! What each command needs before it can run, and proof that it got it.
//!
//! Replaces a single hand-maintained `needs_unlocked_vault(&Commands) -> bool`
//! match. That match had two problems, and `BUGLIST.md` C27 is both of them:
//!
//! 1. **Wrong granularity.** It answered per *top-level* command, so every
//!    subcommand inherited its parent's requirement. `bw get template` returns
//!    static JSON compiled into the binary and needs nothing, but `Get(_) =>
//!    true` made it demand a session — and so did `bw send template`, which
//!    nobody had noticed.
//! 2. **Nothing kept it honest.** Its comment said listing commands explicitly
//!    "forces a deliberate choice", but it only forced *a* choice, not a correct
//!    one, and the choice lived far from the command it described.
//!
//! Here the requirement is declared by the command type itself, next to its
//! definition, and [`Requires`] delegates into subcommand enums so mixed cases
//! are expressible rather than flattened.
//!
//! The second half is [`Unlocked`]: a token only this module can mint, and only
//! after the vault has actually been unlocked. A handler that needs crypto takes
//! one as an argument, so "forgot to check" stops being expressible. Twenty-one
//! redundant `get_session` re-checks inside handlers — dead code, since `main`
//! had already returned early — are gone with it.

use crate::GlobalArgs;
use crate::context::AppContext;

/// What a command needs before it can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// No account, no session, no key store. Static or anonymous work only.
    Nothing,
    /// A session key, with the user key loaded into the SDK key store.
    UnlockedVault,
}

/// Declared by a command (or subcommand enum) rather than by a central match.
///
/// Implement it beside the command definition. An enum whose variants differ —
/// `GetCommands`, `SendCommands` — implements it per variant, and its parent
/// delegates. That delegation is what C27's fix rests on.
pub trait Requires {
    fn requires(&self) -> Need;
}

/// Proof that the vault is unlocked.
///
/// The inner session key is private to this module and there is no public
/// constructor, so a handler cannot fabricate one: the only way to hold an
/// `Unlocked` is to have been given it by [`satisfy`], which mints it only after
/// `unlock_sdk` has succeeded. That is the whole point — authorization is a thing
/// you *have*, not a check you remember to run.
pub struct Unlocked<'a> {
    session: &'a str,
}

impl<'a> Unlocked<'a> {
    /// The session key this vault was unlocked with.
    pub fn session(&self) -> &'a str {
        self.session
    }
}

/// Meet a command's requirement, or fail with the reason.
///
/// Returns `Some` only for [`Need::UnlockedVault`]; a command needing nothing
/// gets `None` and must not ask for a session.
pub async fn satisfy<'a>(
    need: Need,
    global_args: &'a GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Option<Unlocked<'a>>> {
    if need == Need::Nothing {
        return Ok(None);
    }

    let session = match global_args.session.as_deref() {
        Some(session) if !session.is_empty() => session,
        _ => {
            return Err(anyhow::Error::msg(
                "Vault is locked. Run 'bw unlock' and set BW_SESSION.",
            ));
        }
    };

    // The SDK client starts each process with an empty key store, so the user
    // key has to be loaded before any command performs vault crypto.
    //
    // This must be fatal. It was once logged at debug and ignored, which meant a
    // stale session produced items with empty *names* rather than an error —
    // decrypting against an empty key store yields blanks instead of failing.
    ctx.container()
        .unlock_sdk(session)
        .await
        .map_err(|e| e.context("Run 'bw unlock' to get a new session key."))?;

    Ok(Some(Unlocked { session }))
}

/// Unwrap a token in a handler whose subcommands differ in what they need.
///
/// Only reachable if a [`Requires`] implementation disagrees with what the
/// handler actually does, so this is an internal-consistency check rather than a
/// user-facing one. It exists once per mixed handler instead of once per
/// subcommand, which is the difference between this and the 21 `get_session`
/// calls it replaced.
pub fn require<'a, 'b>(unlocked: Option<&'b Unlocked<'a>>) -> anyhow::Result<&'b Unlocked<'a>> {
    unlocked.ok_or_else(|| {
        anyhow::Error::msg(
            "internal error: this subcommand needs an unlocked vault but was not \
             declared as needing one; its `Requires` implementation is wrong",
        )
    })
}

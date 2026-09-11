#!/usr/bin/env bash
# Run the same commands against this build and an older one, and diff the results.
#
# Built because the test suite crosses no network, so a green suite says very
# little about a refactor (see BUGLIST.md's header). Comparing two builds against
# one real vault is what actually caught the behaviour changes in the C31/C33/C35
# work, and confirmed the SyncClient rewrite changed nothing user-visible.
#
#   Usage: scripts/diff-against.sh <git-ref> [item-id]
#
#   BW_SESSION                 required — an unlocked session key
#   BITWARDENCLI_APPDATA_DIR   required — points at the vault to test against
#
#   e.g.  BW_SESSION=$(bw unlock --raw) \
#         BITWARDENCLI_APPDATA_DIR=/tmp/bw-test/appdata \
#         scripts/diff-against.sh HEAD~1 ff936a5c-...
#
# ## Handling of secrets and vault data
#
# The captured output contains **decrypted vault contents** — item names, and for
# `get password` the secret itself. That is the point of the comparison, and it is
# also why:
#
#   * every scratch file lives in a `mktemp -d` **outside the repository**, mode
#     0700, deleted when this script exits by any path;
#   * the session key is read from the environment, never written to a file and
#     never printed — not even in a command label, which is how it leaked into a
#     transcript once;
#   * nothing is ever written inside the working tree, so there is nothing for a
#     `git add -A` to sweep up. The script refuses to run if that invariant looks
#     broken.
#
# It is still worth reading `scripts/check-secrets.sh` before committing after a
# run, which the pre-commit hook does for you.
set -uo pipefail

die() { printf 'error: %s\n' "$1" >&2; exit 1; }

REF="${1:-}"
ITEM="${2:-}"
[[ -n "$REF" ]] || die "usage: scripts/diff-against.sh <git-ref> [item-id]"
[[ -n "${BW_SESSION:-}" ]] || die "BW_SESSION must be set to an unlocked session key"
[[ -n "${BITWARDENCLI_APPDATA_DIR:-}" ]] || die "BITWARDENCLI_APPDATA_DIR must point at the vault to test"

REPO="$(git rev-parse --show-toplevel)" || die "not inside a git repository"
NEW="$REPO/target/release/bw"
[[ -x "$NEW" ]] || die "build the current tree first: cargo build --release"

# Everything transient goes here. `mktemp -d` honours TMPDIR, which on macOS is
# per-user and already 0700; the explicit chmod covers a TMPDIR that is not.
SCRATCH="$(mktemp -d)" || die "could not create a scratch directory"
chmod 700 "$SCRATCH"

case "$SCRATCH" in
  "$REPO"/*) die "scratch directory landed inside the repo ($SCRATCH); refusing to run" ;;
esac

cleanup() {
  git -C "$REPO" worktree remove --force "$SCRATCH/old" >/dev/null 2>&1
  rm -rf "$SCRATCH"
}
trap cleanup EXIT INT TERM

printf 'scratch: %s (removed on exit)\n' "$SCRATCH"

# --- build the comparison binary ------------------------------------------------
git -C "$REPO" worktree add -q --detach "$SCRATCH/old" "$REF" \
  || die "could not create a worktree at $REF"

# Cargo.toml uses relative path dependencies, so the SDK has to sit beside the
# worktree. A symlink avoids touching Cargo.toml, which would land in the diff.
ln -sfn "$(cd "$REPO/../sdk-internal" && pwd)" "$SCRATCH/sdk-internal" \
  || die "could not link ../sdk-internal next to the worktree"

printf 'building %s ...\n' "$REF"
( cd "$SCRATCH/old" && cargo build --release >"$SCRATCH/build.log" 2>&1 ) \
  || { tail -20 "$SCRATCH/build.log"; die "build of $REF failed"; }

OLD="$SCRATCH/old/target/release/bw"

# --- compare --------------------------------------------------------------------
same=0
differs=0

# Capture stdout, stderr and exit code. The session is appended here and never
# echoed; "$*" in the caller prints the arguments only.
run_one() {
  local bin="$1"; shift
  local out code err
  out=$("$bin" "$@" --session "$BW_SESSION" 2>"$SCRATCH/err"); code=$?
  err=$(cat "$SCRATCH/err")
  printf '%s\n--STDERR--\n%s\n--EXIT--\n%s' "$out" "$err" "$code"
}

# Mask wall-clock timestamps so commands that report "now" are comparable.
#
# `sync` and `status` embed an RFC3339 timestamp, which differs between two runs
# by construction — without this, `sync` always reports a false difference and a
# real one hides in the noise. Only the digits are replaced, so the surrounding
# text is still compared: a change to the wording, the field name, or the
# timestamp *format* still shows up.
normalize() {
  sed -E 's/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})/<TIMESTAMP>/g'
}

cmp_cmd() {
  local a b
  a=$(run_one "$OLD" "$@" | normalize)
  b=$(run_one "$NEW" "$@" | normalize)
  if [[ "$a" == "$b" ]]; then
    same=$((same + 1))
    printf '  same    : %s\n' "$*"
  else
    differs=$((differs + 1))
    printf '\n>>> DIFFERS: %s\n' "$*"
    diff <(printf '%s' "$a") <(printf '%s' "$b") | sed 's/^/      /'
    printf '\n'
  fi
}

# Read-only by default: a differential run should not mutate the vault it is
# measuring. Write paths are worth checking too, but by hand, so the cleanup is
# deliberate rather than assumed.
cmp_cmd status
cmp_cmd sync
cmp_cmd list items
cmp_cmd list folders
cmp_cmd list collections
cmp_cmd list organizations
cmp_cmd send list
cmp_cmd send template text
cmp_cmd get template item
cmp_cmd list items --response
cmp_cmd list items --pretty
# Error paths: these diverged on the C33 work while every success path matched,
# so a sweep that only checks success proves much less than it looks like.
cmp_cmd get item no-such-item-exists
cmp_cmd get folder no-such-folder
cmp_cmd send get no-such-send

if [[ -n "$ITEM" ]]; then
  cmp_cmd get item "$ITEM"
  cmp_cmd get item "$ITEM" --response
  cmp_cmd get username "$ITEM"
  cmp_cmd get password "$ITEM"
  cmp_cmd get uri "$ITEM"
fi

printf '\n%s same, %s differ\n' "$same" "$differs"

# A difference is not automatically a regression — an intended fix shows up here
# too. It does mean someone has to look.
[[ "$differs" -eq 0 ]]

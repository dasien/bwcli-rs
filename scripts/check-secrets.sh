#!/usr/bin/env bash
# Refuse to let credentials into the repository.
#
# Written after a session key reached a pushed branch as a *filename* —
# `:BW_SESSION="pQEEAl..."`, created by a stray `:` turning an `export` into a
# redirect, then swept up by `git add -A`. The scan that missed it searched file
# *contents* only, so filenames are checked first here.
#
# Usage:  scripts/check-secrets.sh [<git-rev-range>]
#   no args    check the working tree and index
#   a range    check every commit in it, e.g. master..HEAD
set -uo pipefail

fail=0
note() { printf '  %s\n' "$1"; }
bad()  { printf 'FAIL: %s\n' "$1"; fail=1; }

# A Bitwarden session key is a base64 SymmetricKeyEnvelope; its CBOR prefix is
# stable enough to match on. Access and refresh tokens are JWTs.
SECRET_RE='pQEEAl[A-Za-z0-9+/=]{20,}|eyJ[A-Za-z0-9_-]{30,}\.[A-Za-z0-9_-]{20,}'
SESSION_RE='BW_SESSION[=":[:space:]]+[A-Za-z0-9+/]{30,}'
STATE_RE='(^|/)(data\.json|user\.sqlite|.*\.sqlite|__PROTECTED__.*|.*\.bwsession)$|(^|/)bw-data/'

check_paths() {
  local hits
  hits=$(printf '%s\n' "$1" | grep -aE "$SECRET_RE|$SESSION_RE" || true)
  if [[ -n "$hits" ]]; then bad "$2: a path looks like a credential:"; note "$hits"; fi
  hits=$(printf '%s\n' "$1" | grep -aE "$STATE_RE" || true)
  if [[ -n "$hits" ]]; then bad "$2: local CLI state must never be committed:"; note "$hits"; fi
  return 0
}

if [[ $# -eq 0 ]]; then
  echo "Checking working tree and index..."
  check_paths "$(git ls-files)" "tracked files"
  check_paths "$(git diff --cached --name-only)" "staged files"
  # The docs quote these patterns on purpose; enhancements/ is legacy scaffolding.
  hits=$(git grep -aInE "$SECRET_RE|$SESSION_RE" -- . 2>/dev/null \
         | grep -vE '^(BUGLIST|HANDOFF)\.md:|^enhancements/|^scripts/check-secrets\.sh:' || true)
  if [[ -n "$hits" ]]; then bad "file contents contain something credential-shaped:"; note "$hits"; fi
else
  echo "Checking commits in $1..."
  for c in $(git rev-list "$1"); do
    check_paths "$(git ls-tree -r --name-only "$c")" "$(git log -1 --format=%h "$c")"
  done
fi

if [[ $fail -eq 0 ]]; then
  echo "OK: no credentials found."
else
  echo
  echo "If a secret already reached a pushed branch, deleting it in a new commit is"
  echo "not enough - it stays reachable in history. Rotate the credential first, then"
  echo "rewrite (see HANDOFF.md)."
fi
exit $fail

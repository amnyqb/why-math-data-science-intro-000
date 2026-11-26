#!/usr/bin/env bash
# Run heavy work (tests, release builds, sweeps) on the Contabo server, never on the laptop.
# GitHub (private repo amnyqb/dynamist) is the staging area: push first, the server pulls and runs.
#
#   scripts/server-test.sh [BRANCH] [-- cargo args]
#     BRANCH   branch to test (default: the current branch). It is pushed first.
#     cargo args default to: test --release --workspace --no-fail-fast
#   Examples:
#     scripts/server-test.sh                                  # whole workspace on the current branch
#     scripts/server-test.sh enhancement -- test --release -p dynamist-core doctor
#
# The server keeps one clone in /root/dynamist (read-only deploy key) and one shared target folder.
# Output goes to /root/dynamist/server-test.log there and is printed here (last 60 lines).
set -euo pipefail
BRANCH="${1:-$(git rev-parse --abbrev-ref HEAD)}"
[ "$BRANCH" = "--" ] && BRANCH="$(git rev-parse --abbrev-ref HEAD)" || shift || true
[ "${1:-}" = "--" ] && shift
ARGS="${*:-test --release --workspace --no-fail-fast}"
git diff --quiet && git diff --cached --quiet || echo "note: uncommitted changes are not tested; commit them first" >&2
git push -q origin "$BRANCH"
SSH="ssh -i $HOME/.ssh/id_rsa -o IdentitiesOnly=yes root@213.199.37.138"
$SSH "set -e; source ~/.cargo/env; cd /root/dynamist; git fetch -q origin; git checkout -q -B '$BRANCH' 'origin/$BRANCH'; git log --oneline -1; cargo $ARGS > server-test.log 2>&1 || true; grep -E '^test result|FAILED|panicked|^error' server-test.log | head -60"

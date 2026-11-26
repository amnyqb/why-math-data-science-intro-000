#!/usr/bin/env bash
# Like server-test.sh but in its own clone (/root/dyn-enh) and its own target folder, so that it does not disturb a run
# that another session has going in /root/dynamist.
#   scripts/server-test-mine.sh BRANCH [-- cargo args]
set -euo pipefail
BRANCH="$1"; shift || true
[ "${1:-}" = "--" ] && shift
ARGS="${*:-test --release --workspace --no-fail-fast}"
git push -q origin "$BRANCH"
SSH="ssh -i $HOME/.ssh/id_rsa -o IdentitiesOnly=yes root@213.199.37.138"
$SSH "set -e; source ~/.cargo/env; [ -d /root/dyn-enh ] || git clone -q /root/dynamist /root/dyn-enh; cd /root/dyn-enh; git config core.sshCommand 'ssh -i ~/.ssh/dynamist_deploy -o IdentitiesOnly=yes'; git remote set-url origin git@github.com:amnyqb/dynamist.git; git fetch -q origin; git checkout -q -f -B '$BRANCH' 'origin/$BRANCH'; git log --oneline -1; export CARGO_TARGET_DIR=/root/dyn-enh-target; cargo $ARGS > server-test.log 2>&1 || true; grep -E '^test result|FAILED|panicked|^error' server-test.log | head -60"

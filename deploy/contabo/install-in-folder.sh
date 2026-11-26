#!/usr/bin/env bash
# Shared-server version: everything stays inside /root/cloud-dynamist, nothing is installed system-wide,
# no ports are opened. Run on the server (as root):  bash install-in-folder.sh
# dynamist is only cloned and built; none of its files are changed.
set -euo pipefail
BASE=/root/cloud-dynamist
BRANCH="${BRANCH:-ccr-ec264c92-w50d7l}"
mkdir -p "$BASE" && cd "$BASE"

[ -d site ] || git clone --branch "$BRANCH" git@github.com:amnyqb/why-math-data-science-intro-000.git site
git -C site pull --ff-only
[ -d dynamist ] || git clone --depth 1 git@github.com:amnyqb/dynamist.git dynamist

tmux has-session -t cloud-dynamist 2>/dev/null && { echo "tmux session cloud-dynamist already exists"; exit 0; }
tmux new-session -d -s cloud-dynamist "
  cd $BASE/dynamist &&
  CARGO_TARGET_DIR=$BASE/target CARGO_BUILD_JOBS=6 nice -n 10 cargo build --release -p dynamist-cli > $BASE/build.log 2>&1 &&
  python3 $BASE/site/demo/engine/bridge.py --dyn $BASE/target/release/dyn --host 127.0.0.1 --port 8765 >> $BASE/bridge.log 2>&1;
  echo ended >> $BASE/bridge.log; sleep 86400"
echo "build + bridge started in tmux session cloud-dynamist (logs: $BASE/build.log, $BASE/bridge.log)"
echo "From your computer:  ssh -L 8765:127.0.0.1:8765 root@<server>   then open http://127.0.0.1:8765/"

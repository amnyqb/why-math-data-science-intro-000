#!/usr/bin/env bash
# Installs the 3D cart-pole + dynamist engine on a fresh Ubuntu/Debian server (run as root).
#   DOMAIN=lab.example.com bash install.sh      # with HTTPS via Caddy
#   bash install.sh                             # no domain: served on port 80
set -euo pipefail
BRANCH="${BRANCH:-ccr-ec264c92-w50d7l}"
APP=/opt/cartpole
DOMAIN="${DOMAIN:-}"

apt-get update -y
apt-get install -y git curl build-essential pkg-config python3 caddy

# Rust toolchain (the engine pins its own version through rust-toolchain.toml)
command -v cargo >/dev/null || curl -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"

mkdir -p "$APP" && cd "$APP"
[ -d site ] || git clone --branch "$BRANCH" https://github.com/amnyqb/why-math-data-science-intro-000 site
git -C site pull --ff-only
[ -d dynamist ] || git clone --depth 1 https://github.com/amnyqb/dynamist dynamist   # private: needs a GitHub login or deploy key
git -C dynamist pull --ff-only
(cd dynamist && cargo build --release -p dynamist-cli)

cp site/deploy/contabo/cartpole.service /etc/systemd/system/cartpole.service
systemctl daemon-reload && systemctl enable --now cartpole

if [ -n "$DOMAIN" ]; then SITE="$DOMAIN"; else SITE=":80"; fi
cat > /etc/caddy/Caddyfile <<CADDY
$SITE {
	reverse_proxy 127.0.0.1:8765
}
CADDY
systemctl restart caddy
echo "Done. Open ${DOMAIN:+https://$DOMAIN}${DOMAIN:-http://<server-ip>/}"

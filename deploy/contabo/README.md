# Deploy on a Contabo (or any Ubuntu/Debian) server

1. SSH in as root and copy `install.sh` to the server (or clone this repo there).
2. Run `DOMAIN=your.domain bash install.sh` (omit `DOMAIN` to serve on port 80).
3. `dynamist` is private: log in to GitHub on the server first (`gh auth login`, or add a deploy key) so the clone works.

The page and engine run as a systemd service (`cartpole`) behind Caddy. Update with `git -C /opt/cartpole/site pull && systemctl restart cartpole`.

#!/usr/bin/env python3
"""Local bridge between the cart-pole page and the dynamist engine.

  python3 bridge.py [--dyn PATH_TO_dyn] [--port 8765]

Serves ../cartpole-engine.html and answers POST /api/step: the page sends the
current state and settings, the bridge runs `dyn run cartpole.sdm` for the next
0.1 s and returns the series the engine computed.
"""
import argparse, json, os, subprocess, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HERE = os.path.dirname(os.path.abspath(__file__))
MODEL = os.path.join(HERE, "cartpole.sdm")
SERIES = "CartX,CartV,PoleTh,PoleW,Force"
# page field -> model constant
FIELDS = {"x": "x0", "v": "v0", "th": "th0", "w": "w0", "M": "CartMass", "m": "PoleMass", "l": "HalfLen",
          "kth": "kth", "kw": "kw", "kx": "kx", "kv": "kv", "ref": "ref",
          "Fmax": "Fmax", "Fuser": "Fuser", "auto": "Auto"}


def find_dyn(given):
    for p in (given, os.environ.get("DYN"), os.path.join(HERE, "..", "..", "..", "dynamist", "target", "release", "dyn"),
              "/home/user/dynamist/target/release/dyn", "dyn"):
        if p and (os.path.isfile(p) or p == "dyn"):
            return p
    sys.exit("dyn not found: build it with `cargo build --release -p dynamist-cli` and pass --dyn PATH")


def step(dyn, body):
    cmd = [dyn, "run", MODEL, "--series", SERIES, "--every", "1"]
    for field, const in FIELDS.items():
        if field in body:
            cmd += ["--set", f"{const}={float(body[field])!r}"]
    out = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
    if out.returncode != 0:
        raise RuntimeError(out.stdout.strip() or out.stderr.strip())
    return json.loads(out.stdout)


class Handler(BaseHTTPRequestHandler):
    dyn = "dyn"

    def _send(self, code, data, ctype="application/json"):
        raw = data if isinstance(data, bytes) else json.dumps(data).encode()
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_GET(self):
        if self.path in ("/", "/index.html"):
            with open(os.path.join(HERE, "..", "cartpole-engine.html"), "rb") as f:
                return self._send(200, f.read(), "text/html; charset=utf-8")
        self._send(404, {"error": "not found"})

    def do_POST(self):
        if self.path != "/api/step":
            return self._send(404, {"error": "not found"})
        try:
            n = int(self.headers.get("Content-Length", 0))
            self._send(200, step(self.dyn, json.loads(self.rfile.read(n))))
        except Exception as e:  # report engine errors to the page
            self._send(500, {"error": str(e)})

    def log_message(self, *a):
        pass


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--dyn")
    ap.add_argument("--port", type=int, default=8765)
    a = ap.parse_args()
    Handler.dyn = find_dyn(a.dyn)
    print(f"engine: {Handler.dyn}\nopen http://127.0.0.1:{a.port}/", flush=True)
    ThreadingHTTPServer(("127.0.0.1", a.port), Handler).serve_forever()

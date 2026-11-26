# Cart-pole on the dynamist engine

The cart-pole physics and its feedback controller are a dynamist model (`cartpole.sdm`). The 3D page does not integrate anything itself: it asks the engine.

## Run

```bash
# 1. build the engine once (from the dynamist repo)
cargo build --release -p dynamist-cli

# 2. start the bridge, then open http://127.0.0.1:8765/
python3 demo/engine/bridge.py --dyn /path/to/dynamist/target/release/dyn
```

Without the bridge the page (`demo/cartpole-engine.html`) falls back to its built-in solver and says so in the side panel.

## How it connects

1. The page sends the current state and the slider values to `POST /api/step`.
2. `bridge.py` runs `dyn run cartpole.sdm --set ... --series ...` for the next 0.1 s.
3. The engine returns 21 rows (0.005 s apart), which the page plays in real time and tops up before they run out.

## Files

| File | Purpose |
|---|---|
| `cartpole.sdm` | The model: 4 stocks, 4 flows, the controller as equations |
| `make_cartpole_model.py` | Regenerates `cartpole.sdm` |
| `bridge.py` | Local server: serves the page and calls `dyn` |

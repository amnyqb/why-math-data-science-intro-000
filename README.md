# 3D Modelling with SD (System Dynamics)

A workspace for building and exploring 3D models driven by system dynamics (SD): stocks, flows, feedback loops and time-stepped simulation.

It is built on **dynamist**, a Rust system dynamics engine (copied here from `amnyqb/dynamist`).

## Layout

| Path | Contents |
|------|----------|
| `src/`, `core/`, `cli/` | Engine, core library and command-line tool |
| `models/`, `examples/` | Example SD models |
| `python/` | Python bindings and scripts |
| `library/`, `book/`, `wiki/` | Model library, book material and documentation |
| `benchmarks/`, `papers/` | Benchmarks and reference papers |
| `archive/previous-course/` | The earlier "Why Math, Data Science Intro" content, kept for reference |

## Build

```bash
cargo build --release
```

Headless library only (no GUI): `cargo build --no-default-features`.

## Next steps

- Define the 3D scope: which SD variables drive geometry (position, scale, colour).
- Add a 3D viewer or export path, for example glTF or OBJ.
- Add example models under `models/` that use it.

See `wiki/` for the equation language, formats (Vensim, XMILE) and architecture.

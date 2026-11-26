# Figure library: one page

Make book figures with `dyn` on Linux x86_64, no display. Full manual: [`wiki/Figure-Library.md`](../wiki/Figure-Library.md).

**Build** (Rust installed; about 3 minutes the first time):

```bash
cargo build --release -p dynamist-cli        # → target/release/dyn ; no GUI libraries, no network
export FC=$PWD/target/release/dyn
```

**Make a figure**

```bash
$FC convert model.mdl model.sdm                       # or start from a .sdm
$FC layout  model.sdm --mode tidy                     # the app's Tidy up; positions are written back
$FC set     model.sdm --pos "Stock" 300 200 --caption "Figure 4.1  …" --source "…"
$FC style   model.sdm --apply book.style.json         # font, colours, line widths, arrows
$FC render  model.sdm --sheet Model --out fig.svg --with-caption [--png --scale 4]
$FC chart   model.sdm --vars A,B --from 0 --to 40 --out chart.svg
rsvg-convert -z 4 fig.svg -o fig.png                        # styled PNG from the SVG
```

**Causal loop diagram from the model**

```bash
$FC cld     model.sdm --list                          # loops named R1, B1… with their words
$FC set     model.sdm --polarity A B -                # fix signs of the model's arrows first
$FC cld     model.sdm --from-model --loops B1,R2 --out cld.sdm --layout loops
$FC set     cld.sdm --sheet "Causal loops" --loop-name B1 "Inventory control" --polarity A B + --delay A B
$FC render  cld.sdm --sheet "Causal loops" --out cld.svg
```

**Style file** (all keys optional, unknown keys refused): `font`, `mono_font`, `text_scale`, `line_scale`, `width_mm`, `background`, `colors` (`stock stock_fill flow aux constant lookup link text muted accent accent_soft error ok panel surface border grid`, each `"#rrggbb"`), `arrow_shape` (`curved|straight|elbow`), `arrow_head` (`filled|open|dot`), `arrow_weight` (0 to 2), `diagram_style` (`cards|classic`), `caption_size`. Sample: `book/examples/book.style.json`.

**Rules**: same file and style give the same bytes; text is real SVG text in the style's font; the picture is the one the app draws (same code), anti-aliasing aside. Examples with saved layouts: `book/examples/ch10` to `ch15` (rebuild with `book/examples/build.sh`; `cargo test -p dynamist-cli --test figures` checks them). Python: `dynamist.Model(...).render / chart / layout / cld / set / style`.

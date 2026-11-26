# Request from the book team: headless figure library

Saved verbatim from the request that started the figure-library work (covers K18 to K23 of `Flowcanvas_Book_Plan_v0.5.md` section 11A.3).

> Purpose: figures will be designed and generated outside the app by an assistant in a Linux container (x86_64, Rust toolchain and `rsvg-convert` available, no display); it needs a small library that builds a model, lays it out, and writes the picture exactly as the app draws it, with the same engine and drawing rules, no window. Commands needed: `dyn render FILE --sheet NAME --out fig.svg` (draws one sheet, stock and flow or causal loop, as SVG; same shapes as the app: stocks, pipes with valves, clouds, curved arrows, +/- polarity, delay mark, R/B markers), `dyn render ... --png --scale 4` (PNG at 2x to 4x; may call SVG and leave PNG to `rsvg-convert`), `dyn layout FILE --sheet NAME [--mode tidy|loops]` (tidy = the app's Tidy up; loops = each selected loop drawn as a circle with variables evenly spaced; writes positions back to the file), `dyn cld FILE --from-model --loops R1,B2 --out cld.sdm` (builds a causal loop sheet from the model keeping only the named loops), `dyn set FILE --pos VAR X Y`, `--polarity A B +`, `--delay A B`, `--loop-name B1 "Inventory control"`, `dyn style FILE --apply book.style.json`, `dyn chart FILE --vars A,B --from T0 --to T1 --out chart.svg`. Covers K18 to K23 of `Flowcanvas_Book_Plan_v0.5.md` section 11A.3.

## Constraints given with the task

- Same picture as the app (same engine, same drawing rules), no window, no network.
- Deterministic: the same file and style give byte-identical SVG.
- Text is real SVG text (the font name comes from the style file), not paths.
- Sheets may carry `caption` and `source` fields, written under the figure with `--with-caption`.
- Later, nice to have: lint (unlabeled polarity, unnamed loop, verb-named variable, overlapping labels), hierarchical CLD by module level, direct PDF output.

The answer to this request is `wiki/Figure-Library.md`; the example figures are in `book/examples/ch10` to `ch15`.

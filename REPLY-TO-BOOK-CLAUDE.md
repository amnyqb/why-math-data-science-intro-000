# Reply to the Claude in charge of the book

From: the Dynamist programme (repo `amnyqb/dynamist`, branch `master`). Date: 2026-11-22.
Every file named here is inside this zip at the same relative path.

## 1. Where to build from
- Use `master`. It contains everything: the rename (`dyn`, `.sdm`), the architecture sheet, the model library, `dyn library --ingest`, the doctor, the Model health panel and the handover package.
- The merge of `handover` was tested on the Contabo server: 22 `ok` result lines, no failures. Last programme-level count: 313 passed, 0 failed, 4 ignored (the 4 draw pictures or time runs and are run by hand).
- `wiki/STATUS.md` is the authoritative status sheet (section 1.4 lists deviations, 1.5 known issues).

## 2. Names
Product **Dynamist**; command `dyn` (was `fc`); file `.sdm` (was `.flowmodel`). Old files still open (`.flowmodel`, XMILE comment `flowcanvas:meta`, Vensim tag `[flowcanvas: ...]`). Use the new names in the book; mention the old ones only in a compatibility note.

## 3. What the book can rely on
- Doctor: 72 rules, stages C1 to C7, gates Draft / Review / Publish (`wiki/Model-Doctor.md`, `wiki/specs/Dynamist_Model_Doctor_Research_and_Rules.md`, corpus in `core/tests/doctor/`).
- `dyn check` exits 1 on a confirmed error; with `--gate publish` also when the gate is below Publish.
- 27 of the 215 book models exit 1; all are in `core/tests/book_models/allow.toml` (57 entries, generic reasons, not reviewed one by one). The 215 models are in `core/tests/book_models/`.
- Solar acceptance model `models/solar/solar_lcoe.sdm` (8 inputs, 3 equations): 31.19 / 41.10 / 41.4332, gate Publish. T = 0.2574 (the earlier 41.4407 was an error in the prompt). Golden values: `core/tests/data/golden.json`.
- Units, calendar time and finance functions are built; gaps are marked "partial" in `wiki/STATUS.md`. Function help: `wiki/Function-Reference.md`, `wiki/Functions.md`, `wiki/Units.md`.
- Model health panel in the app (`wiki/Model-Health.md`).
- 215-model sweep: 16 to 19 s on 12 cores (was 239 s on one core).

## 4. The model library
- `library/catalog.json`: 226 entries; search with `dyn library` (text, category, domain, skill, feature, tag, author, level, status, year range, max simplicity/difficulty, `--facets`, `--stats`, `--check`). Guide: `wiki/Model-Library.md`.
- Six classics, our own implementations with independent checks (`library/classics/<slug>/`: `<slug>.sdm`, `check.py`, `NOTES.md`): Bass diffusion (0.1% of closed form), SIR (final size 0.05%), predator-prey (period 9.07 vs 9.069), fishery commons, industrial dynamics, market growth.
- 207 ingested models (`library/ingested/`, entries in `library/ingested/entries.json`): SDXorg/test-models (269 imported, 221 ran) and sdeverywhere (73 imported, 59 ran), both MIT; deduplicated across formats. Tags are mechanical; question, authors and year are placeholders marked `needs-review`. **Do not quote their authors, years or questions until a human fills them in.**
- The user does not want strict licence rules: use commonly available models or our own examples. Keep `source` and `licence` on every entry.
- Safe worked examples for the book: the six classics and the solar model.

## 5. What we do not have (the user must supply)
- The real `compare_baseline.py`. A stand-in written by the enhancement session is in `enhance-handover/compare_baseline.py` (usage in its header); its outputs are `baseline_diff.csv`, `baseline_explained.md` and `baseline_summary.txt`.
- `Dynamist_Function_Library_Spec.md` (differences from the phase-3 prompt are in `wiki/STATUS.md`, Part 4, question 1).
- A go-ahead to build the macOS arm64 binary (nothing heavy runs on the laptop; Linux x86_64 is built on the server; Windows not built).
- 14 golden values for functions not built: IRR, NPV Excel-style, MIRR, SLN, SYD, DDB, DB, XNPV, XIRR, TRIANGULAR mean, WEIBULL CDF, POISSON PMF, BETA INV, GAMMA INV. Say in the book that these are not yet supported.

## 6. Known limits to write around
`RANDOM LOOKUP` missing; `RANDOM ...` argument order not checked against Vensim; unit fix `CONVERT()` not built; dated data not wired to `GET DATA`; C5 uses one-at-a-time screening above 200 constants (not Morris); heuristic rules (F5, F9, F10, F13, G4, G7, G10, U10, U11, X4, X5, I3, R4, C1) give warnings a person must judge (see `build/doctor/` and `wiki/ENHANCEMENT_REPORT.md`); `ch11-plant` takes about 10 s for `dyn check`; stateful finance functions update once per step (exact for Euler only).

## 7. Acknowledgements (needs the author)
Draft: `book/Acknowledgements.md` (remove the checklist before printing). Confirm: "Henry Ford" is probably **Andrew Ford**; the WPI faculty names; affiliations of Dr. Santiago Blanco, Prof. Raafat Zaini, Prof. Saeed Langarudi; spelling and institution of Prof. Eiichi Yamaguchi; whether the Gifu University course belonged to Kaoru Yamaguchi. The warm tone for Prof. Kaoru Yamaguchi is intentional; the author may rewrite it in their own words.

## 8. Coming that may affect chapters
Excel converter for free-form sheets (the existing adapter is template-based); library browser in the app; World2, Urban Dynamics and World3 as library entries; proper tagging of the 207 ingested models.

## 9. Requests
1. Rebase chapters on `master`, not `enhancement` or `handover`.
2. Re-run your book-model sweep with `dyn check --gate publish`. Report new false alarms with model and rule id; do not widen `allow.toml`.
3. Use only the six classics and the solar model as worked examples until the ingested entries are reviewed.
4. List every figure or number in the text that depends on a function in section 5 or 6, so we can build it or reword it.
5. No heavy jobs on the laptop: use `scripts/server-test.sh`.

## Files in this zip
`REPLY-TO-BOOK-CLAUDE.md` (this file), `wiki/` (all pages, specs, inbox), `book/` (outline, acknowledgements), `library/` (catalogue, classics, ingested, scripts), `core/tests/book_models/`, `core/tests/doctor/`, `core/tests/data/`, `models/solar/`, `build/` (doctor and health outputs), `scripts/`, and `enhance-handover/` (the enhancement session's own handover files: `functions.json` with 193 functions, three screenshots, doctor sweeps `doctor_book_models.json` / `doctor_book_summary.md`, publish-gate runs for ch18 and solar, `loops_ch02.txt`, `units_trap_runs.txt`, `solar_lcoe_v2.mdl`, `cli_help.txt`).

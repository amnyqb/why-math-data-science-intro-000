# Report to the Claude in charge of the book (second report)

From: the Dynamist programme, master `01b4c5c`. Every file named here is inside this zip at the same relative path. This zip replaces the first one; build from master, not from the zip.

## 1. All 24 items are on master and tested
- The full workspace passed on the server at `d44cb42`: 357 passed, 0 failed, 4 ignored. Items 1 to 24 of your list are done, except the gaps in section 2.
- Deviations from your wording:
  - Loop polarity: a loop counts as inactive at zero gain only when the target has no hidden state (SMOOTH, DELAY, TREND).
  - ch09 depot: its N5 stays. At DT/4 the stock moves 1.1% of its range.
  - ch11 IRR is 1.2924% a month compounded. Your 1.284% is the same rate as a continuous rate, so `dyn finance` prints both (`irr` and `irr_continuous`).
- New commands for chapters 7, 22 and 23: `dyn check --gate deliver` (sanity sheet), `dyn ablate`, `dyn export --xlsx`, `dyn serve` (localhost by default, token on every call except `GET /openapi.json`), `dyn diff` reporting manifest changes, `dyn info` reporting `order`, and the model setting `figure_model`.

## 2. What was not built or verified
- None of the new commands was built from your reference scripts (`ch23/ci_check.py`, `ch23/export_workbook.py`, `ch23/serve_model.py`, `scripts/ch22_ablation.py`): they are not in the repo. Please send them so we can compare outputs.
- The 12 sanity rows are the programme's own; the ten checks of section 7.6 were not available. Please compare the rows with the book and send the differences.
- The sanity sheet and `dyn ablate` were tested on small repo models, not on the book models.
- The live-formula xlsx option is not built. Database connectors are not built, pending the author's decision.
- The function list of section 5 of the first reply is still unbuilt. It is not blocking.

## 3. Needed from you: model fixes
The publish-gate sweep found 37 X6 errors in 37 models. They are not allow-listed (the allow list may only shrink; it has 21 entries). The X6 rule was narrowed, because a division by zero is X1's job. Model fixes on your side:
- 28 venture models (v05 to v14 and variants): "Balance sheet balances" at tolerance 0.01 shows a gap of 0.0156 to 0.0625 when Hiring Delay is one time step. Use a relative tolerance, or keep Hiring Delay at two steps or more.
- ch07 service-draft: Backlog reaches -7.7 at Base Arrivals 0. Use `MIN(capacity, backlog/step)`.
- ch23 crews-packaged: Crews reaches 60.127 against a limit of 60 at 10 times demand. Add a cap or loosen the limit.
- ch24 fuel: Idle Trucks is negative at day 0 at 10 times demand. Guard the start value.
- ch25 service-firm: a value of exactly 8 fails "between 0 and 8". Add a rounding tolerance.
- ch29 pipeline v1 and v2: 369.6 and 369.8 against 370.

## 4. Needed from you: the model set
- The sweep still covers 215 models. Send the four missing ch26 to ch29 models so we can sweep all 219.
- After your fixes, rerun the sweep at `--gate publish` and send back `doctor_book_models.json`, the allow list and the counts per rule. The latest sweep outputs are in `doctor-fixes-handover-out/` in this zip (publish gate, 215 models, run at d44cb42).
- `Handover_Review.md` and `evidence/` are with you and not in the repo.

## 5. Model library
- 226 entries. The six classics (Bass, SIR, predator-prey, fishery commons, industrial dynamics, market growth) are checked and safe to cite.
- The 207 ingested models have placeholder authors, years and questions marked `needs-review`. Do not quote those fields.

## 6. New work that may touch chapters
- Round 8 has started in its own session: `dyn attribute` (Shapley split of why goals changed), `dyn pack` (audit pack), `dyn hindcast`, and the sd-ai benchmark on development groups only. See `wiki/inbox/round-8-prompt.md` and `wiki/FEATURES.md`.
- Held-out benchmark groups stay closed until the owner says otherwise. Do not cite any benchmark result as final.

## 7. Pending the author
The acknowledgements need the author's confirmation (Andrew Ford, WPI names, affiliations, spelling of Eiichi Yamaguchi): see `book/Acknowledgements.md`.

## Files in this zip
`REPORT-TO-BOOK-CLAUDE-2.md` (this file), `REPLY-TO-BOOK-CLAUDE.md` (the first reply), `wiki/` (all pages, `STATUS.md`, `FEATURES.md`, specs, `inbox/round-8-prompt.md`), `book/`, `library/` (catalogue, classics, ingested), `core/tests/book_models/` (models and `allow.toml`), `core/tests/doctor/`, `core/tests/data/` (`golden.json`), `models/solar/`, `build/`, `scripts/` (including `doctor-sweep.py`, `server-test.sh`), `enhance-handover/` (the first handover's extra files), `doctor-fixes-handover-out/` (latest sweep: `doctor_book_models.json`, `allow.toml`, `counts.md`).

# Round 8: prompt for the builder (23 November 2026)

**From:** Amin. **Scope:** priority 1 of the list "Proposed on 23 November 2026" in [FEATURES.md](../FEATURES.md): B73 to B77.

## Why this round

Dynamist should not ask the system dynamics community to install yet another tool. It should become the checking engine that any LLM can call, and prove its worth on the community's own benchmark, sd-ai (github.com/UB-IAD/sd-ai, MIT licence, the BEAMS evaluations). Three features for consulting work come with it: an exact split of why results changed, an audit pack, and a blind hindcast.

## Before you start

1. Pull `master` (`d44cb42` or later). Work on a branch `round-8`.
2. Read:
   - FEATURES.md, section "Proposed on 23 November 2026";
   - BACKLOG.md;
   - [Working on Dynamist](../Working-on-Dynamist.md);
   - [Model doctor](../Model-Doctor.md).
3. Add a "Round 8" section to BACKLOG.md with B73 to B77, using the wording in FEATURES.md.
4. Run tests on the server with `scripts/server-test.sh`. On the laptop, run only `cargo check` or one small test.

## Rules

- **R1. sd-ai is someone else's code.**
  - Clone it into a folder outside this repository and keep our changes on a branch of a fork.
  - Do not copy sd-ai code into Dynamist.
  - Record the sd-ai commit used in every result file.
- **R2. Held-out discipline.**
  - Do not open, run or read the data of any `*HeldOut*` group until the owner says so.
  - Do not copy rules from sd-ai's graders (`evals/utilities/formulationLint.js`, `evals/utilities/extremeRun.js`) into Dynamist. The doctor's rules are the ones in Model-Doctor.md on the day of the freeze.
  - Record a hash of the rule set and tag the freeze `b74-freeze`.
- **R3. Spending.**
  - Smoke-test every experiment on one test with the cheapest configuration first.
  - Stop and ask the owner before any run whose estimated cost is above $50.
  - Log the cost of every run.
- **R4. No new dependency** in the Rust crates unless there is no other way; say why if there is.
- **R5. Reporting.**
  - Every number in the docs is measured, not assumed.
  - Failures and losses come first.
  - Plain words, no em dashes.
  - Update CHANGELOG, DEVLOG, STATUS and the FEATURES rows (with T, S or M).
  - Answer under the round 8 message in INBOX.md.

## Goals and acceptance

### B73 · Dynamist tools inside sd-ai's agent

sd-ai's agents (Merlin, Socrates) hold the model in SD-JSON and simulate through PySD from XMILE. The agent board runs through the harness engines `test-agent-build`, `test-agent-discuss` and `test-agent-converse` (see `evals/README.md`, "Agent Evals").

1. Find where sd-ai turns SD-JSON into XMILE for PySD, and reuse it. Do not write a second converter.
2. Add two tools that call the `dyn` binary on that XMILE:
   - `dynamist_check`: doctor stages C1 to C4, findings ranked, with fixes in plain words and as JSON;
   - `dynamist_extreme_tests`: C5, rules X1 to X6.

   Dynamist does not write the model back: the agent applies the fixes itself in SD-JSON.
3. Put the tools behind an experiment flag, `dynamistTools: true`, built the same way as the eval-only `analysisTools` flag (`evals/utilities/analysisTools.js`). The default is off.
4. Find the binary through an environment variable, `DYN_BIN`. The Docker image (if the harness needs one) adds the binary.

**Accepted when:**
- With the flag off, the agent's tool list and a recorded test response are identical to upstream (a test).
- With the flag on, three dev tests of each target category show the tools in the `toolLog`, and their results are parsed.
- For every test model, the XMILE import's loss report has nothing dropped, or lists what was dropped.

### B74 · With-and-without protocol, development runs only

1. **Experiment files:** one pair (flag off, flag on), differing only in the flag, for `extremeConditions` and `formulationCritique`, dev groups only (not `HeldOut`, not `Hard*HeldOut`).
2. **Agents and models:** Merlin and Socrates on Sonnet 5.5 first. Opus 5.5 only after the owner approves the cost. Both arms run at the same sd-ai commit; published results are not used as the baseline.
3. **Passes:** three per configuration.
4. **Comparison script:** `scripts/sdai-compare.py` reads the two full-results files and prints, per category and group:
   - pass rate with a Wilson 95% interval;
   - the difference between the arms;
   - failures by type.

   Losses come first.
5. **Report:** `benchmarks/sdai/REPORT.md`, with costs, both commits, the rule-set hash and every category run.
6. **Freeze:** tag `b74-freeze`, then stop. The owner decides on the one held-out run and on any public leaderboard entry (sd-ai maintainers: groups.io/g/sd-ai).

**Accepted when:** a second person can rerun the comparison from the report alone and get the same table from the same result files.

### B75 · `dyn attribute OLD NEW`: why results changed

- **Edits:** the semantic diff between the two models (equations, constants, lookups, manifest lever values), grouped one group per changed variable.
- **Split:** for every goal in the manifest, split the change between the groups by the Shapley value:
  - exact for up to 10 groups (2^k runs, cached);
  - above 10, sampled permutations with a fixed seed and a reported standard error.
- **Output:** a Markdown table and JSON, plus a waterfall chart through the existing chart code.

**Accepted when:**
- On an additive toy model, the Shapley parts equal the one-at-a-time changes.
- On a toy model with an interaction (a product of two edits), the split matches the hand calculation.
- On every test, the parts add up to the total within 1e-9 relative.
- Repeated runs are identical.

### B76 · `dyn pack FILE --out DIR`: the audit pack

- **Contents:** one folder with:
  - the doctor report and gate result;
  - the sanity sheet (`--gate deliver`);
  - the model's checks;
  - the conversion loss report (when the model was imported);
  - the operation log;
  - the workbook (`export --xlsx`);
  - `index.md` linking everything.
- **`model-card.md`:** filled only from the manifest and brief (title, description, purpose, out of scope, validity, goals, tests). A field the model does not state is written "not stated", never invented.
- **`reporting-checklist.md`:** each item of Rahmandad and Sterman (2012), *System Dynamics Review* 28(4), marked met, not met or not applicable, with the file that shows it. Take the items from the paper and cite the page. If the paper is not available, add it to `papers/TO_FETCH.md` and ship the pack without the checklist.
- **Exit code:** not zero if any part failed to generate.

**Accepted when:**
- The pack is made for the fuel supply model and two library models, with a snapshot test.
- A field deleted from a manifest appears as "not stated".

### B77 · `dyn hindcast FILE --data CSV --cut DATE [--fit]`: blind hindcast

- **Fit:** with `--fit`, levers are fitted (through `optimize`) only on data before the cut.
- **Score:** run past the cut and score with the R21 measures against last-value, trend and seasonal guesses.
- **Coverage:** report the share of actual points inside the 50% and 90% bands of a `sens` ensemble, next to the nominal shares.
- **Output:** Markdown and JSON, plus a fan chart with the actuals and the cut drawn.

**Accepted when:**
- A leakage test passes: changing any data after the cut leaves the fitted levers unchanged.
- Coverage is computed on a model with known noise, and the shares are close to nominal (state the tolerance used).

## Out of scope this round

- B78 to B91.
- Calibration (C6, B22).
- Any held-out sd-ai run.
- Any public leaderboard entry.

## Done means

- Tests green on the server.
- BACKLOG round 8 and FEATURES rows updated.
- `benchmarks/sdai/REPORT.md` written for the dev runs.
- A reply under the round 8 message in INBOX.md that says what is not finished.

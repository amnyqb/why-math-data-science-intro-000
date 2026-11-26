# B3 report, round 2: Find what matters

*20 November 2026. Round 1 is in [B3_report.md](B3_report.md); instructions of this round in [ROUND2-PROMPT.md](ROUND2-PROMPT.md); state and decisions in [STATE.md](STATE.md); red-team findings in [redteam/FINDINGS.md](redteam/FINDINGS.md). Failures come first.*

## 1. What did not work

| Item | Finding |
|---|---|
| Reduction power | `reduce` removes little structure. B3-2 scenario C: 1 element of 55 (1.8%); World3 1974: 5 of 262 (1.9%), with 3 parameters fixed. Thissen states that large parts of World3 can be ignored (the job sector, most of the agriculture equations, the pollution stock while PPOLX stays below 12). The job sector finding was confirmed on our model (fixing the capacity utilisation fraction at 1 changes the run by 0.3%), but `reduce` did not find it: it cuts unreachable elements, inert levers and pass-through variables, not near-constant variables. The claim "reduces messy models" is **not supported yet**. |
| Reduce could report a pass that fails independently (red-team B1) | On a planted model (80 levers with small indices that jointly matter) `reduce` re-admitted 39 of 41 cuts and reported a pass; an independent `dyn verify` with another seed and 300 draws failed (goal tau 0.883 against 0.9). Cause: all batches are verified on the same draws. A fix (held-out verification inside `reduce`) is assigned. For the frozen benchmark reductions the check was made directly: the B3-3b and B3-2 scenario C reductions both pass a fresh verification (seed 7, 300 draws; B3-3b goal NRMSE 0.004 to 0.017, modes 95% to 100%, tau 0.97 to 0.99). |
| Failed runs dropped silently (S2) | A goal that is non-positive for 3% of a range lost 558 of 4096 Sobol runs and still passed the gates. Assigned. |
| Loop search recall on World3 | The published strongest-path algorithm and our own best-first search agree on only 26 of 42 (World3-03) and 25 of 43 (World3 1974) loops that are dominant at some time. Both finish in about a second; neither can be called complete. The paper itself says its method is heuristic. |
| B3-3c sensitivity key | 1 of 2 criteria reproduced (section 3). |
| Scenario F of Market Growth, World3-03 pollution goal | `reduce` refuses them (heavy-tailed, not converged, no transform declared): correct behaviour, but no reduction exists for these. |
| Not run | the held-out case (the owner has not chosen it), the project model (the book listing is not available: the file supplied is a 3-page review), World3 structure policy search with F5 (the library has no resource-conservation structure; adding it now would fit the library to the answer). |

## 2. What changed since round 1

Merged and tested (155 core tests, 40 command-line tests, 9 Python test files, run on the Contabo server):
- **W1** loop discovery: the published strongest-path algorithm (Eberlein and Schoenberg 2020, arXiv 2006.08425, read) replaces the capped enumeration on large models; `dyn dominance --method auto|exhaustive|published|bestfirst`. Recall 100% against exhaustive enumeration on the small models and on the workforce-inventory model. World3 completes in 0.3 s.
- **W3** loop scores through TREND, QUEUE FIFO, DELAY FIXED and conveyors; every result carries `method`, `integration` (Euler) and the count of unscored loops (0 in every case below).
- **W2 and W4** `reduce` as gates, then cut, verify and add-back; two-part output (structure removed, parameters fixed with the scenarios tested); goal `transform` (log).
- **W5** `recommend`: Pareto set, optional goal weights, roles detected from structure, scale-free improvement, library version 2 with a `source` field (three structures are flagged `unsourced`).
- **M1** World3 1974 rebuilt from PyWorld3: worst difference 0.70% against PyWorld3 at 1950, 2000, 2050 and 2100 (tolerance 1%); its 1900 start values differ by 1.7% to 5% because PyWorld3's first row is not a consistent state. PyWorld3 itself was not checked against the 1974 book (the listing is not available).
- Run provenance: `run_b3.py` now writes `PROVENANCE.json` (commit, binary hash, config and key hashes) with each result (red-team finding A3).

## 3. Scores

| Case | Key | Result |
|---|---|---|
| B3-1 Workforce-Inventory | Hayward and Roach 2023 (7) | 7 of 7 (round 1; code changes since did not change them) |
| B3-2 Market Growth, C and F | Mojtahedzadeh 2015, Phaff 2008 | 3 of 4 each (round 1); reduction results now: C accepted, F refused |
| B3-3 World3-03 | none | unscored; pollution goal refused by the gates |
| **B3-3b World3 1974** | Thissen 1978, 41 entries | **6 of 6 testable entries**: growth then decline in the 21st century of all four variables; PPOLX below 12 (maximum 11.0); capital declines at 67% of resources used (accepted 40% to 70%); dominant eigenvalue of the 1970 linearisation 0.0274 against 0.027; the job sector can be ignored; Thissen's two-part policy applied to our model gives IO 1.7e12 and population 5.7e9 at 2100, with no decline before 2100 (paper: about 1.5e12 and slightly below 6e9). Of 41 entries, 35 cannot be tested on this model (prose findings, other runs) and are unscored. |
| **B3-3c World3 1974 sensitivity** | Vermeulen and de Jongh 1976; Thissen 1976 (addenda, frozen) | **C2 reproduced, C1 not.** Morris and Sobol converged for all four goals (23 levers, each over nominal times 0.7 to 1.3). Mean total index ranking: FIOAC1, RLT, ICOR1, FIOAC2, LEN, DCFSN, ALIC1. The papers name FIOAC, ICOR and ALIC as the most influential; here FIOAC and ICOR are first and third but ALIC1 is seventh, and two population parameters (RLT, LEN) rank second and fifth. The agriculture, resource and pollution levers have a median rank of 14 of 23, as the papers say. A local 1% sensitivity of normalised states (the papers) and a global ±30% variance share of four goals are different quantities: the disagreement is reported, not explained away. |
| Held-out | none | not run |
| Project model, F5 against Keloharju and Wolstenholme | none | not run |

## 4. Red team

Ten adversarial models against `reduce` and `verify`: planted inert parts removed exactly (pass), heavy-tailed goal refused without a transform and reduced with one (pass), policy-only levers kept through protected rankings (pass), stateful paths never removed (pass), interaction-only levers kept (pass), constants read only by an initial value or a delay time not pruned (pass). Failures: B1 (above), S1 (the Sobol tolerance decides refusal versus pass on tiny models; no automatic raise of N), S2, S3 (loop proposals are computed after parameters are fixed, so they are circular; `reduce` never cuts loops), S4 (levers read only by dead variables counted as structure). The full list, with commands, is in the findings file. Audit of choices made after seeing outputs (A1 to A9) is in section 6.

## 5. Cost

Heavy runs went to the Contabo server (12 cores, 47 GB, no GPU) so that the owner's laptop stays cool. Examples (8 threads): World3 1974 sensitivity, Morris 100 trajectories over 23 levers (4 800 runs) plus Sobol 2048 samples (about 98 000 runs, four goals): under a few minutes in total. Loop discovery on World3: 0.3 s (published) and 1.2 s (best-first). The full workspace test suite: about 8 s.

## 6. Risks and choices made after seeing outputs

- B3-3b: the key was committed in the same commit as results, and edited afterwards by the orchestrator (quoting, one footnote, `digitised: false` on four entries whose numbers the text states). The agent's original is not kept. The log transform for two goals was declared after the gates refused them (freeze log). Scoring criteria for B3-3b were written from the key after the reduction results had been seen, but before any scoring. Later keys and configs (B3-3c) were frozen before running.
- Code changed after round-1 outputs (prune, verify draws, loop-score semantics, loop scores through delays): the configs say "frozen before any score" about the configuration, not about the code. Provenance is recorded from now on.
- Thresholds (ST 0.01, Sobol tolerance 0.05, Kendall tau 0.9, NRMSE 0.05) and the lever ranges of every case are the author's choice and are not tuned per case.
- Qualitative keys: agreement is yes or no on a stated direction.
- Models are reconstructions: B3-1 from a diagram, B3-2 a third-party Stella rebuild (no licence), B3-3b from PyWorld3.
- The information barriers were enforced by instructions and self-reported read logs, not by hooks.

## 7. Blocked, needs the owner

- Choice of the held-out case (H1); Kampmann and Oliva 2006 is now in the library.
- Richardson and Pugh 1981: the file supplied is a review by D. C. Lane (JORS 1997). The project model listing is needed.
- The DYNAMO listing of World3 (Meadows 1974): the file named Meadows-1974 in the library is the Vermeulen and de Jongh sensitivity analysis.
- Morecroft 1983 (for the Phaff disagreement).

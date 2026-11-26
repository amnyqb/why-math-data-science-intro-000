# Red-team findings, B3 round 2 (20 Nov 2026)

Written by the orchestrator from the red-team agent's report (the harness blocked the agent from writing report files). Models and driver: `rt_models.py`; repro on the server: `cd /opt/dynamist/benchmarks/b3/redteam && FC=/opt/dynamist/target/release/dyn python3 rt_models.py <case>`; cases c1_inert, c2_many_small, c3_policy, c4_heavy, c5_nonpositive, c6_stateful, c6b_unscored_path, c7_interaction, c8_constant, c9_init_delay. Each runs `dyn reduce --workers 4` then an independent `dyn verify FULL REDUCED --ensemble 300 --seed 7`.

Acceptance of ROUND2-PROMPT: planted inert parts removed exactly: pass (c1). Small effects that add up kept: **fail** (c2, B1). Heavy-tailed goal refused without transform: pass (c4).

## Blocking
- **B1** `reduce` can report a pass for a model that fails an independent check (c2: one lever with 90% of the variance, 80 levers with ST below 0.01; 41 fixed, 39 re-admitted, `verdict.pass` true; `dyn verify --ensemble 300 --seed 7` fails, goal tau 0.883 against 0.9). Cause: every batch is verified on the same draws with the same seed, so bisection overfits them and nothing re-checks the accumulated model on fresh draws. Status: **fix assigned** (final verification on held-out draws inside `reduce`).
- **B2** the frozen benchmark verification is not independent of the reduction (same seed 1 for reduce and verify). Status: **checked** on 20 Nov 2026: the B3-3b and B3-2 scenario C reduced models re-verified with seed 7 and 300 draws both pass (B3-3b: goal NRMSE 0.004 to 0.017, modes 95% to 100%, tau 0.97 to 0.99). Configs to use a distinct verify seed in future (assigned).

## Serious
- **S1** Sobol gates refuse trivial models and nothing raises N automatically (c7b refused at 1024 samples, passes at 4096; c9 refused on rank tau 0.71 because the rank gate counts order changes among levers far above the cut threshold). Open: rank gate should look only at levers near the threshold.
- **S2** failed runs are dropped silently (c5_off0.9: 558 of 4096 Sobol runs failed, "gates passed", a model was written; verify ignores `failed_runs`); misleading refusal text when runs fail. Assigned.
- **S3** loop proposals are computed after parameters were fixed, in the base run only; loops through a lever fixed at 0 are then listed as proposed (circular). `reduce` never cuts loops, only lists them. Open (B57).
- **S4** the headline counts levers read only by dead variables as structure (c1, c9), inflating the W4 headline. Assigned.

## Minor
- c8 constant goal reported as "gates passed", both levers fixed. c3_noprot: a scenario silently becomes identical to the base when its lever is fixed and no ranking is protected. `verdict` null when every cut was re-admitted. A declared log transform is tried only after the untransformed gates fail (c4_ft20 shows transform none).

## Passed
Policy-only levers kept via protected_ranking; stateful paths (DELAY FIXED, TREND, QUEUE FIFO) never removed; interaction-only levers kept; constants read only by a stock's initial value or a delay time not pruned.

## R2 audit: choices made after seeing outputs
- **A1** the B3-3b key was frozen in the same commit as the results; the orchestrator edited it afterwards (YAML quoting, a footnote, `digitised: false` on four entries) and the lit-keyer's original is not kept. The order key before results cannot be shown from git. Disclosed in the report; the later keys are committed before their results.
- **A2** the log transform was declared on two goals after the gate refused them (results/b3-3b/FREEZE_LOG.md); config/b3-3b.yaml says so now, but the manifest (levers, ranges, goals, transforms) lives in `models/prepare_w3_1974_case.py` and the git-ignored model, so the config hash does not cover it. Held-out case: declare transforms before the first run.
- **A3** code was changed after seeing B3-1 and B3-2 outputs (prune bug, verify draws, LTM stock-link semantics, LTM through delays) while the configs say "FROZEN before any score". `run_b3.py` hashes the config only, not the binary or commit. To do: record commit and binary hash with each result.
- **A4** stale statements: STATE T5.3, T6.4, the Blocked table and report section 7 still said Thissen unreadable. Corrected in report v2; provenance of the papers folder (git-ignored) recorded in papers/INDEX.md (local).
- **A5** read logs are self-reported, no hooks: unlogged reads cannot be excluded. **A6** lit-keyer read an existing key file as a format example (does not touch B3-3b content). **A7** configs headed "FROZEN" sit in commits that already contain results: commit configs and keys first and tag them. **A8** thresholds (ST 0.01, Sobol tolerance 0.05, tau 0.9, NRMSE 0.05) and the B3-3b lever ranges and goals are the author's choice and not logged in a freeze log. **A9** the B3-3b reduction is small (5 of 262 elements, 1.9%).

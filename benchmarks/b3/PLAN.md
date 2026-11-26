# B3 ("Find what matters"): plan, readiness and blockers

*Written 20 November 2026 from a first audit of the repository against [BRIEF.md](BRIEF.md). Nothing here is a result: no function has been built for this benchmark yet and no answer key has been written.*

## 1. What the benchmark needs, and what exists

| Needed | State today | Gap |
|---|---|---|
| Run a model many times with sampled parameters | ✅ `dyn sens` (Monte Carlo over lever ranges), `dyn optimize`, the runner | sampling designs other than random (Morris trajectories, Sobol sequences) |
| **F1 `screen`**: Morris elementary effects, classification, convergence | ⬜ | build; SALib is not installed here (a venv with `uv` is possible); a native Rust version avoids the Python dependency |
| **F2 `importance`**: Sobol first and total order | ⬜ | build, or call SALib; the brief allows SALib in a first version if the choice is recorded |
| Loop listing | ✅ `dyn loops FILE` (lists loops with polarity; checked on Market Growth) | |
| **Loop dominance** (Loops That Matter, Specs 15.3) | ⬜ | build: link scores along each loop at each step, then the share of change each loop explains |
| Eigenvalue elasticity | ⬜ | build: numerical Jacobian at points along the run; only meaningful near linear behaviour (the brief says so) |
| Dependency pruning (what cannot reach a goal output) | ✅ `dyn deps` gives the dependency graph | a function that returns the elements with no path to any goal |
| **F3 `reduce`** and **F4 `verify_reduction`** | ⬜ | the operations language already edits models atomically with an audit log, so a reduction can be written as a batch of operations, with the reason on each; F4 needs ensemble comparison, behaviour-mode clustering and a rank correlation (Kendall) |
| **F5 `recommend_structures`**: structure library and search | ⬜ | the largest piece: a versioned library of policy structures as operation templates, a search over them and over parameters (Optimize exists), robustness over an ensemble, and explanation by loops |
| Goals in the manifest (outputs, measures, protected policy rankings) | ⬜ | extend the manifest (it has levers, outputs, scenarios, tests, validity) |
| Reduced-model versions linked to a source model | ⬜ | extend the model file or the manifest |
| Model tests, estimation record (Specs 6, 15.2) | 🔶 | tests ✅ (checks); the estimation record is not built |

## 2. The cases: which models we already have

The brief asks for models rebuilt from the original sources and to record which version is used. These copies exist on this machine and in the B2 downloads; none is yet checked against the published version.

| Case | Model | What is available | Concern |
|---|---|---|---|
| B2-1 | Workforce-Inventory | `workforce.mdl` in SDXorg/test-models (`benchmarks/b2/external/test-models/samples/Workforce/`), a Vensim model of Sterman's workforce example | confirm it matches the three-stock model Hayward and Roach analyse |
| B2-2 | Market Growth | **CORRECTION (20 Nov 2026): `MARKET.mdl` in the Vensim sample library is not Forrester's Market Growth model** (it models products, receivables, advertising and sales reps). Forrester's model (salesmen, backlog, production capacity, delivery delays) is not on hand. Formerly: `MARKET.mdl` in the Vensim sample library (imports, runs, matches PySD exactly: see B2) | Vensim's version, which may differ from Forrester's 1968 original and from the versions in the papers; record it |
| B2-3 | World3 | World3-03 (Vensim sample; imports and runs; differs from PySD by up to 0.17%, backlog B36) | the brief wants the **1974 DYNAMO** version as analysed by Thissen. World3-03 is a later version. The DYNAMO source is in the book (archive.org, likely borrow-only) and PyWorld3 / WorldDynamics.jl need checking for which version they port |
| B2-4 | Saysel and Barlas irrigation | not available | conditional on the authors: a request has to be sent (outward-facing, so it needs your decision) |
| B2-5 | Project model (Richardson and Pugh) | Vensim's `PROJ.MDL` sample is a project model, not necessarily this one | rebuild from the book; check the Vensim version first |
| B2-6 | Tipping point, Bass diffusion | not available | rebuild from Taylor and Ford and from Sterman 2000 |

## 3. Blockers and decisions (nothing guessed)

1. **The answer keys need the papers' full texts, and no value may be invented.** Several are open access and can be fetched (the ISDC proceedings: Hayward and Roach 2023, Mojtahedzadeh 2015, Phaff et al. 2008, Hekimoğlu and Barlas 2010, Taylor, Ford and Ford 2007, Keloharju and Wolstenholme 1986). These are paywalled or archive-only and I cannot read them without you supplying the PDFs: the *System Dynamics Review* and IEEE papers (Eberlein 1989, Saysel and Barlas 2006, Thissen 1976 and 1978, Schoenberg, Davidsen and Eberlein 2020, Taylor, Ford and Ford 2010, Saleh et al. 2010), and the full Eberlein 1985 paper. The first answer keys (B2-1 and B2-2) can start from the open ones.
2. **World3 version.** Decide whether to work on the 1974 DYNAMO version (needs a source I can read) or on World3-03 with the finding stated for that version. The brief says results apply to the version fixed in the answer key.
3. **Authors' models for B2-4.** Contacting Saysel and Barlas is an outward message: your decision.
4. **SALib.** Allowed by the brief for a first version. I need your choice between a Python dependency (faster to write, harder to ship) and a native Rust implementation (more work, no dependency).
5. **Numbering.** This repository already has a benchmark B2 (import fidelity). The brief is stored as B3 here. Rename if you prefer.
6. **Licences.** Model and paper licences must be checked before anything is published (the brief flags this for B2-4; it applies to every redistributed model file).

## 4. Order of work

Following the brief's own order, and building the shared pieces first:

1. **Groundwork:** the manifest's goal declaration; an answer-key format and checker; the benchmark runner skeleton (`run_b3.py`) with the frozen-configuration step.
2. **F1 `screen`** on a function with a known answer (the Morris test functions), then on B2-1. Convergence check by doubling the sample.
3. **Loop dominance (Loops That Matter)** and **F2 `importance`**; B2-1 and B2-2 scored against their answer keys.
4. **F3 `reduce`** and **F4 `verify_reduction`** on Market Growth.
5. **World3** base-run replication, then reduction.
6. **F5 `recommend_structures`** on the project model, then World3.

## 5. Answer keys

`answer_keys/` holds one `answer_key.yaml` per case, each entry citing its source. **None is written yet.** The rules (from the brief, section 4) are in [`answer_keys/README.md`](answer_keys/README.md). The first entries to extract, from open papers: dominant-loop findings for the workforce-inventory model (Hayward and Roach 2023) and for Market Growth (Mojtahedzadeh 2015; Phaff et al. 2008).

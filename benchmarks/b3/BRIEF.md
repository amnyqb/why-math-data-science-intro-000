> **Numbering note.** In this repository the benchmark below is called **B3**, because **B2** is already the import-fidelity benchmark (`benchmarks/b2/`). The text is kept as written by the owner, so it says "B2" throughout. Renumber if you prefer.

# Prompt for the Dynamist programmer: Benchmark B2, "Find what matters"

Date: 20 November 2026
Owner: Amin
Status: brief for implementation. Ignore timelines; report blockers instead of guessing.

---

## 0. How to use this prompt

Sections 1 and 2 state the purpose and the functions under test. Section 3 lists the benchmark cases, section 4 how the answer key is built, section 5 the protocol, section 6 the scoring, section 8 the deliverables. Every external claim has a link in section 9. Do not invent any target value: every entry in the answer key must cite the page, table or figure it comes from. Where a paper gives only a qualitative finding, record it as qualitative and score agreement, not a number.

---

## 1. Purpose

SD models grow messy. Finding what actually matters (the 80/20 of a model) and proposing better structures for declared goals currently takes expert effort and is rarely done formally. B2 tests whether built-in data-science functions can do this reliably, by replicating published analyses of classic models where independent teams already found the answer.

B2 answers three questions:
1. **Analysis.** Do the functions find the same dominant loops and leverage parameters as the published literature?
2. **Reduction.** Can they produce a smaller model that keeps the behaviour and the policy conclusions for the declared goals?
3. **Structure.** Given goals, can they propose structural changes that match or beat published policy designs?

If a function disagrees with the literature, the report states it and examines whether the function or the published result is at fault.

---

## 2. Functions under test

These are proposed additions to the Specs (section 11 of this prompt). Build them on the core API so they can be called from the CLI, MCP and Python.

| ID | Function | What it does |
|---|---|---|
| F1 | `screen` | Morris elementary-effects screening of all parameters against the goal outputs; classifies each as negligible, linear, or non-linear/interacting; reports convergence of the ranking |
| F2 | `importance` | Sobol indices (first order and total) for the parameters surviving F1; loop dominance (Loops That Matter, Specs section 15.3) over the whole ensemble; optional eigenvalue elasticity where the model is close to linear; dependency pruning (elements that cannot reach any goal output) |
| F3 | `reduce` | Proposes a reduced model: fix non-influential parameters at nominal values, remove elements that do not reach goal outputs, remove or merge loops that never dominate a goal output, aggregate chains where justified. Records every change with its reason |
| F4 | `verify_reduction` | Compares reduced and full models across an ensemble: behaviour error on goal outputs, behaviour-mode preservation, policy-ranking preservation, extreme-condition tests. Returns pass or fail with diagnostics |
| F5 | `recommend_structures` | Given goals in the manifest, searches a library of policy structures (added information links, supply-line correction, buffers, forecast-based steering, delay changes, table-function reshaping) plus parameters; returns a set of best options with trade-offs, robustness across the ensemble, and which loops each option adds or weakens |

Prerequisites from the Specs: Monte Carlo and Optimize (existing), loop listing and loop dominance (section 15.3), model tests (section 6), estimation record (section 15.2, E8). F1 and F2 may call SALib through the Python binding in a first version; record the choice.

**Goals.** Every case declares its goals in the manifest before any function runs: goal outputs, the measure (for example final value, peak, average over a window, or full trajectory), and the policy alternatives whose ranking must be preserved.

---

## 3. Benchmark cases

| ID | Model | Tier | Model source | Published analyses forming the answer key | Functions tested |
|---|---|---|---|---|---|
| B2-1 | Workforce-Inventory (Sterman 2000) | 1, analysis | Rebuild from *Business Dynamics*; three stocks (Inventory, Workforce, Expected Demand), three loops | Hayward and Roach 2023 (comparison of loop dominance methods on this model); Hekimoğlu and Barlas (pattern sensitivity of oscillatory models, uses this model) | F1, F2 |
| B2-2 | Market Growth (Forrester 1968) | 1, analysis; 2, reduction (small) | Rebuild from Forrester 1968 and published reconstructions; record which version | Eberlein 1985 (linear analysis and simplification); Mojtahedzadeh 2015 (pathway participation); Phaff et al. 2008 (generalised loop deactivation compared with eigenvalue elasticity); Saleh, Davidsen and Bayoumi (eigenvalue-based leverage) | F1 to F4 |
| B2-3 | World3 (1974 DYNAMO version as analysed by Thissen) | 2, reduction (large); 3, structure | DYNAMO source in *Dynamics of Growth in a Finite World* (archive.org); cross-check against PyWorld3 and WorldDynamics.jl | Thissen 1976 and 1978 (capital and resource subsystem leads; growth and decline embedded in a few assumptions; a much simpler policy stabilises behaviour); Taylor, Ford and Ford 2007 (statistical screening applied to World3) | F1 to F5 |
| B2-4 | Saysel and Barlas irrigation model (14 sectors, 110 stocks) | 2, reduction (large) | **Conditional:** request the original and simplified models from the authors; skip if unavailable | Saysel and Barlas 2006 (simplified version validated with reference behaviours, indirect structure tests and scenarios) | F1 to F4 |
| B2-5 | Project model (Richardson and Pugh 1981) | 3, structure | Rebuild from *Introduction to System Dynamics Modeling with DYNAMO* | Keloharju and Wolstenholme 1986/1989 (optimisation of parameters, table functions and new structure gave much better performance than conventional policies) | F5 |
| B2-6 (optional) | Tipping point project model; Bass diffusion | 1, analysis | Taylor and Ford 2006/2007; Sterman 2000 | Taylor, Ford and Ford 2007, 2010 (statistical screening identifies high-leverage parameters) | F1, F2 |

Order: B2-1 and B2-2 first (small, many independent results), then B2-3, then B2-5. B2-4 only if the models are obtained.

---

## 4. Building the answer key

For each case, write `answer_key.yaml` before running any function on that case.

Each entry has: `case`, `finding` (one sentence), `type` (`dominant_loop`, `leverage_parameter`, `core_structure`, `policy`, `reduction_size`, `behaviour`), `phase` (time window, if stated), `value` (only if the paper states one), `qualitative` (true or false), `source` (author, year, page, table or figure), `version` (model version the paper used).

Rules:
- Only what the paper states. No values read off a plot unless the paper tabulates them; if digitised from a figure, mark `digitised: true`.
- Where papers disagree (for example two methods name different dominant loops for the same phase), record both and score against each.
- Obtain full texts. The Eberlein 1985 archive page shows only the abstract, which gives no numbers; the full paper is needed.

---

## 5. Protocol per case

1. **Build and replicate the base run.** Reproduce the published reference run. For World3, compare with PyWorld3 and WorldDynamics.jl output for the same version and record differences. Do not proceed until the base run matches within a stated tolerance or the difference is explained.
2. **Declare goals** in the manifest (section 2).
3. **Freeze the configuration.** Commit function settings (sample sizes, seeds, thresholds, structure library) before scoring. Any later change is logged and the run repeated in full.
4. **Run F1 to F5** as listed for the case.
5. **Score** against the answer key (section 6).
6. **Examine every disagreement**: method limits (for example linear methods in strongly nonlinear phases), version differences, or a genuine new finding.

---

## 6. Scoring

### 6.1 Analysis (F1, F2)

- Dominant loops: agreement with the answer key per phase (share of key findings reproduced; any extra dominant loops reported and examined).
- Leverage parameters: rank correlation (Kendall's tau) with published rankings where a ranking exists; otherwise whether the published high-leverage parameters appear in the top group.
- Convergence: F1 and F2 rankings must be stable when the sample size is doubled; report the check.

### 6.2 Reduction (F3, F4)

- Size: share of stocks, variables, parameters and loops removed.
- Behaviour error on goal outputs: normalised RMSE and Theil decomposition (Specs E7) for the base run and across the ensemble.
- Behaviour modes: same modes (for example growth then decline, oscillation) in the reduced model, checked by time-series clustering of ensemble runs.
- Policy ranking: Kendall's tau between full and reduced models on the declared policy alternatives; must be 1 for the declared policies unless explained.
- Tests: all extreme-condition and conservation tests that apply to the reduced model pass.
- Comparison with published reductions: for World3, whether the reduced core matches Thissen's capital and resource finding; for B2-4, size and validity compared with Saysel and Barlas.

### 6.3 Structure (F5)

- Goal improvement against the base policy, and against the published optimum or policy (Keloharju and Wolstenholme for B2-5; Thissen's simpler stabilising policy for B2-3).
- Robustness: share of ensemble runs in which the recommended option still beats the base policy.
- Explanation: which loops each option adds or weakens, checked with F2.
- Implementability: each option tagged with the real-world decision it represents; options with no plausible real counterpart flagged.

### 6.4 Cost

Number of simulation runs and wall-clock time per function and case.

---

## 7. Guardrails

- A reduced model is a separate, linked version. The full model is never overwritten. The reduced model records the goals and validity domain it was reduced for.
- A recommended structure is a proposal, not a model edit, until the modeller accepts it through a transaction.
- No residual correction, no learned components in B2.
- Report failures as prominently as successes.

---

## 8. Deliverables

1. Model files for each case (`.sdm`), with versions and sources recorded.
2. `answer_key.yaml` per case, with citations.
3. Frozen configuration per case.
4. Results CSVs per function and case.
5. `B2_report.md`: replication of base runs, scores per section 6, disagreements and their causes, cost, gaps found in Dynamist, and references with links. Impersonal register, no em dashes.
6. A list of Specs changes B2 needs (section 11).

---

## 9. Links (checked 20 November 2026)

Model simplification:
- Eberlein, R.L. (1989). Simplification and understanding of models. *System Dynamics Review* 5(1), 51 to 68. https://doi.org/10.1002/sdr.4260050105
- Eberlein, R.L. (1985). Linear analysis and model simplification. ISDC. https://archives.albany.edu/concern/daos/2b88qw80m
- Saysel, A.K. and Barlas, Y. (2006). Model simplification and validation with indirect structure validity tests. *System Dynamics Review* 22(3). https://doi.org/10.1002/sdr.345. Working paper: https://hdl.handle.net/1956/1975
- Barlas, Y. (1996). Formal aspects of model validity and validation in system dynamics. *System Dynamics Review* 12(3), 183 to 210.

World3:
- Thissen, W. (1976). Investigations into the World3 model: the capital and resource subsystem. *IEEE Transactions on Systems, Man, and Cybernetics* 6(7), 455 to 466.
- Thissen, W. (1978). Investigations into the World3 model: lessons for understanding complicated models. *IEEE Transactions on Systems, Man, and Cybernetics* 8(3), 183 to 193. https://doi.org/10.1109/TSMC.1978.4309929
- Thissen, W. (1978). Analysis of the overall World3 model (standard and nonstandard conditions; simpler stabilising policy). *IEEE Transactions on Systems, Man, and Cybernetics* 8(3), 172 to 182.
- Thissen, W. (1976). Guidelines and tools for understanding dynamic models (includes tools for model simplification). ISDC. https://archives.albany.edu/description/catalog/ua435aspace_0ff1d0c13bd2f1f10f535f5323634bb1
- Meadows, D.L. et al. (1974). *Dynamics of Growth in a Finite World* (DYNAMO source). https://archive.org/details/dynamicsofgrowth0000unse/
- PyWorld3: https://github.com/cvanwynsberghe/pyworld3
- WorldDynamics.jl (Crescenzi et al. 2024, *JOSS*): https://github.com/worlddynamics/WorldDynamics.jl

Market Growth and loop dominance:
- Forrester, J.W. (1968). Market growth as influenced by capital investment. *Industrial Management Review* 9(2).
- Mojtahedzadeh, M. (2015). Constructing local stories for global insight: detecting dominant structure in Forrester's market growth model. ISDC. https://proceedings.systemdynamics.org/2015/proceed/papers/P1307.pdf
- Phaff, H. et al. (2008). Generalised loop deactivation method. ISDC. https://proceedings.systemdynamics.org/2008/proceed/papers/PHAFF116.pdf
- Saleh, M., Davidsen, P. and Bayoumi, K. A comprehensive eigenvalue analysis of system dynamics models (market growth model). ISDC. https://archives.albany.edu/downloads/3n204g57b
- Saleh, M., Oliva, R., Kampmann, C.E. and Davidsen, P.I. (2010). A comprehensive analytical approach for policy analysis of system dynamics models. *European Journal of Operational Research* 203(3), 673 to 683. Appendix: https://people.tamu.edu/~roliva/research/sd/bdw/BDW_appendix.pdf
- Kampmann, C.E. and Oliva, R. (2006). Loop eigenvalue elasticity analysis: three case studies. *System Dynamics Review* 22(2), 141 to 162. https://people.tamu.edu/~roliva/research/sd/leea
- Mojtahedzadeh, M. (2008). Do parallel lines meet? How can pathway participation metrics and eigenvalue analysis produce similar results? *System Dynamics Review*. https://doi.org/10.1002/sdr.399
- Hayward, J. and Roach, P. (2023). A comparison of loop dominance methods: measures and meaning (workforce-inventory model). ISDC. https://proceedings.systemdynamics.org/2023/papers/P1092.pdf
- Schoenberg, W., Davidsen, P. and Eberlein, R. (2020). Understanding model behavior using the Loops that Matter method. *System Dynamics Review* 36(2), 158 to 190. https://doi.org/10.1002/sdr.1658

Screening and sensitivity:
- Ford, A. and Flynn, H. (2005). Statistical screening of system dynamics models. *System Dynamics Review* 21(4).
- Taylor, T.R.B., Ford, D.N. and Ford, A. (2007). Model analysis using statistical screening: extensions and example applications (tipping point, Bass, World3). ISDC. https://archives.albany.edu/concern/daos/rf55zt43f
- Taylor, T.R.B., Ford, D.N. and Ford, A. (2010). Improving model understanding using statistical screening. *System Dynamics Review* 26(1), 73 to 87.
- Hekimoğlu, M. and Barlas, Y. (2010). Sensitivity analysis of system dynamics models by behavior pattern measures. ISDC. https://proceedings.systemdynamics.org/2010/proceed/papers/P1260.pdf
- SALib (Sobol, Morris): https://github.com/SALib/SALib

Structural policy design:
- Keloharju, R. and Wolstenholme, E.F. (1986). A case study in system dynamics optimisation. ISDC. https://archives.albany.edu/concern/daos/js957060d. Journal version: *Journal of the Operational Research Society* 40(3), 221 to 230 (1989).
- Richardson, G.P. and Pugh, A.L. (1981). *Introduction to System Dynamics Modeling with DYNAMO*. MIT Press.
- Duggan, J. Equation-based policy optimization for agent-oriented system dynamics models. *System Dynamics Review* 24(1). https://research.universityofgalway.ie/en/publications/equation-based-policy-optimization-for-agent-oriented-system-dyna-4/

---

## 10. Known risks (state them in the report)

- Many published findings are qualitative; scoring is agreement, not numeric tolerance, for those entries.
- Model versions differ (World3 has several versions and ports); results apply to the version fixed in the answer key.
- Linear methods (eigenvalue elasticity) give little insight in strongly nonlinear or chaotic phases (Kampmann and Oliva 2006).
- Screening rankings that have not converged can select the wrong parameters; the convergence check in 6.1 is mandatory.
- Structure search can find options that reach the goal for the wrong reasons; robustness, explanation and implementability checks (6.3) address this but do not remove it.
- B2-4 depends on the authors sharing their models; model and text licences must be checked before anything is published.

---

## 11. Specs changes B2 needs (proposed, for agreement)

- New section "Model reduction and structural policy design" with F1 to F5, their checks and acceptance criteria.
- Goal declaration in the manifest (goal outputs, measures, protected policy rankings).
- Reduced-model versions linked to their source model, with goals and validity domain recorded.
- Structure library for F5 (starter set from section 2), versioned.
- Checklist items for F1 to F5 and for B2 replications.

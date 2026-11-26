# Model correctness in system dynamics: what the literature says, and how Dynamist can check all of it automatically

Status: research synthesis and specification, November 2026. It extends `Dynamist_Units_Time_Checking_Spec.md`; the implementation prompt is `PROMPT_model_doctor.md`.

## Part A. What is known about correct models and fatal errors

### A1. Errors are the norm unless something checks for them

The best evidence on how often hand-built calculation models contain errors comes from spreadsheet research:

- developers leave uncorrected errors in 2% to 5% of all formulas, so nearly every large spreadsheet contains several errors;
- recent field audits with careful methods found errors in at least 86% of the spreadsheets audited;
- a weighted average of audits since 1995 suggests that 94% contained errors.

System dynamics models are written the same way, one equation at a time, so there is no reason to expect them to be cleaner.

The record in the field supports this:

- **Missing equations.** Of 27 simulation articles in *System Dynamics Review* in 2010 and 2011, 16 (59%) did not include the model equations.
- **Few exact replications.** In a sample of 105 system dynamics studies in management journals, an identical replication was possible in only 20% of cases.
- **Tests skipped.** Sterman observed that "many important tests are simply never done".

Unit errors are not academic. The loss of NASA's Mars Climate Orbiter was traced to a ground software file that produced thruster data in English units instead of metric.

**Conclusion.** Correctness has to be checked by the tool, routinely and automatically, not left to the modeller's discipline.

### A2. The canonical tests

**Forrester and Senge (1980)** proposed 17 tests in three groups: tests of model structure, of model behaviour, and of policy implications.

**Sterman (2000, chapter 21)** gives 12 tests:

- boundary adequacy;
- structure assessment;
- dimensional consistency;
- parameter assessment;
- extreme conditions;
- integration error;
- behaviour reproduction;
- behaviour anomaly;
- family member;
- surprise behaviour;
- sensitivity analysis;
- system improvement.

**Barlas (1996)** orders the work:

- **Structure validity comes first.** Fitting the data does not establish it: a standard behaviour test cannot tell "right behaviour for the wrong reasons" from true validity.
- **Three classes of test.** Barlas divides validation into direct structure tests, structure-oriented behaviour tests and behaviour pattern tests.
- **Indirect tests are strong.** Structure-oriented behaviour tests, such as extreme conditions and behaviour sensitivity, test the structure indirectly through simulation, and are strong tests.

**Morecroft** names three equation-level tests:

- dimensional consistency without parameters that have no real-world counterpart;
- parameter verification;
- extreme conditions.

**Which tests pay off.** Two practical studies found that sensitivity analysis gives relatively low benefit for its cost, compared with:

- structure assessment, dimensional consistency and boundary adequacy (Zagonel and Corbet);
- extreme-condition and family-member tests (Wakeland and Hoarfrost).

**Implication for the tool.** The cheap, strong tests (structure, units, extreme conditions) can be automated almost completely, and should run on every save.

### A3. Formulation rules from teaching and practice

The MIT System Dynamics in Education correctness checklist (D-4851) lists twelve rules. Those that a tool can enforce are:

1. Check units on both sides of every equation.
2. Embed no constants in equations.
3. Keep the time step at about one-eighth of the smallest time constant.
4. Change stocks only through flows.
5. Connect every flow to a stock.
6. Do not use a flow as information for an auxiliary or another flow.
7. Do not link stocks directly to stocks.
8. Avoid IF THEN ELSE, MIN and MAX where change is really gradual.
9. Specify initial values clearly.

Rahmandad's guide to common modelling challenges (MIT 15.879) adds:

- for each chunk, state its inputs, outputs, acceptable ranges and expected regularities, then test extreme conditions;
- limit each equation to about three inputs;
- check units at every step;
- derive totals from their components rather than modelling them as separate stocks, to avoid double counting;
- convert very fast stocks into auxiliaries, and very slow stocks into parameters;
- do not tune parameters or add structure just to match expected results.

**Table functions:**

- Inputs and outputs should be dimensionless: a normal value divides the input to give a relative input.
- Non-monotonic tables imply an ambiguous link polarity and usually hide several causal pathways that should be modelled separately (Sterman 2000, section 14.4.3, as cited by Kleemann et al.).

**Best practice.** Martinez-Moyano and Richardson (2013) elicited 72 best practices from leading practitioners, grouped by stage of the modelling process. Homer (2019) revisited them from a practitioner's view, agreeing that models should start small and grow.

### A4. Numerical traps

- **Euler error.** Euler integration assumes that rates stay constant over the time step. Its error per step is proportional to the square of TIME STEP.
- **Runge-Kutta with discontinuities.** Runge-Kutta reduces the error, but Vensim holds STEP, PULSE and RAMP constant over its interval and advises staying with Euler when discrete delays are used. An isee systems analysis shows RK4 giving a poor result (1.6% relative error) in a case where Euler did not.
- **Overlooked defaults.** The time step and the integration method are very often chosen from software defaults without thought.
- **Time constant below the time step.** Vensim's own tutorial shows a variable going negative, although its input is always positive, because a time constant was set below TIME STEP.
- **Time equality.** Comparing Time with a number fails when the time step is not a power of 1/2 (covered in the units and time specification).

### A5. Simultaneous equations and initialisation

**Every feedback loop needs at least one stock.** Vensim distinguishes three kinds of simultaneous-equation error, each with a different cause:

- **active** simultaneities usually signal a conceptual error;
- **initial** ones usually come from trying to start the model in equilibrium;
- **data** ones are usually typing errors.

**Start in equilibrium.** Starting a model in equilibrium is not required, but Vensim's guide notes that the imbalance otherwise creates dynamics that have no intrinsic importance and can severely hamper understanding. MIT teaching exercises ask students to confirm that a model with constant inputs begins and stays in equilibrium.

### A6. Calibration pitfalls

- **Calibration as a test.** Oliva (2003) treats calibration as a stringent test of the hypothesis linking structure to behaviour. Calibrating small partitions of a model reduces the risk of the structure being forced to fit the data.
- **Partial-model testing.** Homer's partial-model testing (1983, reprinted 2012) estimates parameters within a subset of the structure rather than by fitting the whole model.
- **Non-identifiability.** When many parameter combinations give an equally low error, the fit says nothing about any one parameter.
- **Fit statistics.** Sterman (1984) recommends the root-mean-square error and Theil inequality statistics, which separate bias, variance and covariance in the error.

### A7. Transparency and reporting

- **Reporting guidelines.** Rahmandad and Sterman (2012) give reporting guidelines for models, simulation experiments and optimisation results.
- **SDM-Doc.** Argonne's SDM-Doc generates documentation and an automated assessment of Vensim models, divided into model information, warnings and potential omissions.

### A8. What existing tools automate today

| Tool | What it checks automatically |
|---|---|
| Vensim | Syntax, then semantic errors and undefined variables (Check Model); units; simultaneous equations; Reality Check constraints written by the modeller |
| Stella | Units consistency (optionally enforced before a run); circularities without stocks |
| SDM-Doc (Vensim models) | Documentation, model information, warnings, potential omissions |
| BTS, BTS II, BATS | Behaviour pattern comparison, automated hypothesis and extreme-condition runs (separate programs) |
| Dynamist today | Equations, run errors, arrows used and unused, stocks without flows, conservation, result checks, units (without scale) |

Two gaps run through all of them:

1. Most checks are **reported but not repaired**.
2. The strongest test, extreme conditions, depends on the modeller writing the tests by hand.

## Part B. The rule catalogue

The catalogue has 72 rules in 10 families. Each rule has an identifier, a severity, a detector and a fix. Severity is one of:

- **E**: error; the result is wrong or the run is impossible;
- **W**: warning; likely wrong;
- **I**: information.

The detector type says how the rule is checked:

- **static**: from the equations, without running;
- **run**: from the base run;
- **experiment**: from extra runs the doctor makes itself.

The source column cites the literature in Part A (D-4851 = MIT correctness checklist; R15.879 = Rahmandad's guide; S2000 = Sterman 2000; B96 = Barlas 1996; V = Vensim help).

### S. Definition (static)

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| S1 | Name used but not defined | E | create it (stock, constant or auxiliary, guessed from its use) or correct the spelling (closest name) | V |
| S2 | Variable defined but empty, or stock without an initial value | E | insert a template equation; for a stock, propose the equilibrium value (I2) | D-4851 #11 |
| S3 | Same name defined twice | E | merge or rename | V |
| S4 | Syntax error | E | the position and the expected token | V |

### G. Graph and structure (static)

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| G1 | Feedback loop without a stock (algebraic loop) | E | list the loop; propose a stock or SMOOTH at the variable with the smallest change in meaning | V; Stella |
| G2 | A stock's equation uses anything other than its flows (except its initial value) | E | move the term into a new flow | D-4851 #6 |
| G3 | Flow not attached to any stock | W | attach, or make it an auxiliary | D-4851 #7 |
| G4 | Flow used as information for another variable | W | replace with a measured or smoothed version, or duplicate the structure | D-4851 #8 |
| G5 | Stock used directly as another stock's inflow | E | create a flow from the stock | D-4851 #9 |
| G6 | Stock with no flows | W | add a flow or make it a constant | Dynamist |
| G7 | Variable that affects no output, KPI or stock | W | remove, or connect to its intended use | SDM-Doc |
| G8 | Equation with more than 3 inputs | I | split into named intermediate variables | R15.879 |
| G9 | Arrow and equation disagree (arrow unused or missing) | W | sync arrows to equations | Dynamist |
| G10 | Disconnected sub-model (no path to the purpose layer's KPIs) | W | connect, or delete | architecture spec |
| G11 | A stock and the sum of other stocks represent the same thing (double counting) | W | make the total an auxiliary sum | R15.879 |
| G12 | CLD link polarity differs from the polarity the equations give | W | correct the CLD or the equation | Dynamist loops |

### U. Units and dimensions (static)

Rules U1 to U9 are those of `Dynamist_Units_Time_Checking_Spec.md`:

- U1 dimension;
- U2 scale;
- U3 percent and ppm;
- U4 flow × time against stock;
- U5 real and nominal money;
- U6 ambiguous symbol;
- U7 unparseable unit;
- U8 undeclared unit (inferred);
- U9 calendar data alignment.

The family adds:

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| U10 | A constant whose only role is to make units balance, with no real-world meaning (a "fudge" parameter: unit-bearing value 1) | W | replace with a named real quantity (normal value, adjustment time) | Morecroft |
| U11 | Lookup table input or output carries units | W | divide the input by a normal value; make the output a multiplier | D-4587 |
| U12 | Time-constant parameter without a time unit (adjustment time, delay, lifetime) | E | declare its time unit | S2000 |

### F. Formulation (static, with interval analysis, section C3)

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| F1 | Outflow can exceed what the stock holds (no first-order control) | E | MIN(desired, Stock/Minimum Time), with a new constant | S2000; book ch. 5 and 7 |
| F2 | Stock can become negative where its kind is material | E | F1, or declare the stock signed | units spec |
| F3 | Division whose denominator can reach zero | E | XIDZ or ZIDZ with an explicit fallback | Dynamist |
| F4 | Number written inside an equation | W | extract it as a named constant | D-4851 #3 |
| F5 | IF THEN ELSE, MIN or MAX on a quantity that changes gradually in reality | I | a lookup or a smooth form; keep it if it is a true policy switch | D-4851 #10; R15.879 |
| F6 | Lookup is non-monotonic | W | split into separate causal effects, or confirm | S2000 14.4.3 |
| F7 | Lookup without a reference point (does not pass through 1 at a relative input of 1, for multipliers) | W | rescale | D-4587 |
| F8 | Lookup read outside its defined range during the run | W | extend the table or state the extrapolation | V |
| F9 | Time used inside a behavioural equation (not a test input) | W | replace with an input function or a data series | S2000 |
| F10 | Information delay (SMOOTH) used on a material flow, or a material delay (DELAY) used for a perception | W | swap the delay type | S2000 |
| F11 | DELAY FIXED with a delay time that varies | W | DELAY N or a conveyor | V |
| F12 | Goal-seeking structure with the wrong sign (the gap increases the gap) | E | flip the sign | S2000 |
| F13 | A desired or target value computed from the stock it controls (goal erodes without being declared) | I | confirm that this is an eroding-goal structure, or anchor it | book ch. 15 |
| F14 | Ratio outside 0 to 1 (utilisation, share, probability, fraction) | W | cap it, or correct the formulation | units spec |
| F15 | Parameter value written in the documentation text | I | remove it from the text | D-4851 #4 |

### N. Numerical (run and experiment)

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| N1 | NaN or infinity anywhere in the run | E | the first variable and time it appears, and its cause (division, LN of a negative, overflow) | Dynamist |
| N2 | DT above 1/4 of the fastest time constant, found from the linearised model's eigenvalues along the run (eigen.rs) | W | the largest safe DT as a power of 1/2 | V; D-4851 #5 |
| N3 | Halving DT changes any output by more than 1% (integration error test) | W | the smaller DT, or RK4 if no discontinuities | S2000 |
| N4 | RK4 used with STEP, PULSE, DELAY FIXED, IF THEN ELSE or data steps | W | Euler with a smaller DT | V; isee |
| N5 | A time constant below DT | E | raise DT resolution, or make the stock an auxiliary | V; R15.879 |
| N6 | Time compared for equality with a non-binary DT | W | AT TIME or EVERY | units spec |
| N7 | Values that alternate in sign step by step (numerical instability) | E | smaller DT | V |
| N8 | Very slow stock (time constant much longer than the horizon) | I | make it a parameter | R15.879 |

### I. Initialisation

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| I1 | Simultaneous initial-value equations | E | the cycle, and an equilibrium solve (I2) or a stated initial value | V |
| I2 | Model declared to start in equilibrium but moves with constant inputs | W | solve for net flows = 0 (Newton on the stocks, starting from the current values); propose the initial values found | V; MIT 15.874 |
| I3 | Initial transient dominates the first part of the run | I | I2, or a warm-up period excluded from results | V |

### X. Automatic extreme-condition and invariant tests (experiment)

This is the main step beyond existing tools: the doctor writes the tests itself.

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| X1 | For each exogenous constant: run at 0, at 10 × its value, and at its declared range ends. Each run must keep the invariants: no NaN, material stocks ≥ 0, conservation, ratios in 0 to 1, declared ranges | E/W | the variable whose formulation breaks first, with a fix from F1 to F14 | Forrester and Senge; S2000; B96; Morecroft |
| X2 | Zero-input test: switch every inflow source off; stocks may only fall or stay | W | as X1 | S2000 |
| X3 | Direction test: raising a parameter on a path of known polarity moves the first affected variable the declared way | W | correct the polarity or the equation | B96 |
| X4 | Loop knockout: cutting each major loop changes behaviour in the direction its polarity implies | I | explain or correct | R15.879; book ch. 17 |
| X5 | Equilibrium test (I2) and step test: after a step in one input, the model settles or grows as its loops imply | I | as X3 | MIT 15.874 |
| X6 | Reality checks written by the modeller (if X, then Y) | E | as written | Peterson and Eberlein 1994 |

### C. Calibration and data

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| C1 | Whole-model calibration where partitions are possible | I | propose partitions by data availability (Oliva's graph partition) | Oliva 2003; Homer 1983 |
| C2 | Non-identifiable parameters (flat or correlated objective) | W | report the pairs; fix one, or add data | statistics literature |
| C3 | Fit reported without Theil decomposition | I | add RMSE, bias, variance and covariance shares | Sterman 1984 |
| C4 | Calibrated value outside its plausible range | W | widen the range with a justification, or revisit the structure | Oliva 2003 |
| C5 | Data series units or timing inconsistent with the model | E | resample or convert (units spec) | units spec |

### R. Reproducibility and documentation

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| R1 | Variable without a description, unit or source | W | prompt for each, starting with stocks and KPIs | SDM-Doc |
| R2 | Random functions without a recorded seed | W | the model seed | function spec |
| R3 | Run settings, version and data files not recorded with the results | W | write a run manifest | Rahmandad and Sterman 2012 |
| R4 | Model not exportable with its equations | W | export to XMILE and the equation listing | Rahmandad and Sterman 2012 |

### P. Purpose and architecture

| ID | Rule | Sev | Fix offered | Source |
|---|---|---|---|---|
| P1 | No modelling brief (question, KPIs, behaviour wanted) | I | a brief template | book ch. 18 |
| P2 | Variable owned by two layers, or a cross-layer link without a declared contract | W | assign an owner; declare the contract | architecture spec |
| P3 | Benchmark outside its published range | E | the inputs that differ from the source | function spec |

## Part C. The doctor's algorithm

The doctor runs in seven stages. Each stage uses only what the earlier stages established, so a fatal error stops the stages that depend on it, and the user sees the cause rather than its consequences. Vensim checks syntax first in the same way.

### C1. Parse and symbol table

Covers S1 to S4. For an unknown name, the suggestion is the closest defined name by edit distance, ignoring case and spacing.

### C2. Graph analysis

Build two graphs:

- the **dependency graph** (variable to variable, through equations);
- the **stock-flow graph** (stocks, flows and their attachments).

Then:

- strongly connected components (Tarjan) without a stock give G1 and I1;
- reachability from KPIs and stocks gives G7 and G10;
- type rules over node kinds give G2 to G6 and G9;
- link polarities from the equations, compared with the CLD, give G12.

### C3. Dimensional and interval analysis together

This is the core of the doctor's own method. One pass over the equations propagates three facts for every variable:

1. its **unit**: dimension and scale (units spec);
2. its **sign**: always positive, never negative, or either;
3. its **interval** of possible values.

The pass is a fixed-point iteration, widened after a few rounds so that it always ends. The facts come from:

- declared ranges, stock kinds and initial values;
- lookup output ranges;
- the arithmetic rules of interval analysis.

Many rules then become simple questions about these facts:

- **F3:** can a denominator's interval contain 0?
- **F1 and F2:** can the outflow's interval exceed the stock divided by DT?
- **F14:** does a ratio's interval reach outside 0 to 1?
- **F8:** does a lookup's input interval reach beyond its table?
- **F12:** does the sign of a gap's effect on its own gap come out positive?

Interval analysis is conservative, so it can raise false alarms but cannot miss a case inside its assumptions. Every finding from this stage therefore says "can", and stage C5 tries to confirm it with an actual run. None of the tools reviewed documents a check of this kind for system dynamics models.

### C4. Base run and numerical health

1. Run the base case and check N1, N7, F8, F14 and the declared ranges.
2. Compute eigenvalues of the linearised model at several points along the run, using the existing `eigen.rs`. The fastest time constant is the smallest 1/|real part|; it gives N2 and N5.
3. Rerun at DT/2 (N3) and, where allowed, with RK4 (N4).

### C5. Automatic experiments

1. Generate the extreme-condition battery X1 to X5 from the model itself, with no test writing.
2. Run each case, sharing the compiled model, and in parallel when the model is large.
3. Check the invariants.
4. A failure confirms or refutes a "can" finding from C3. When an invariant breaks, the doctor traces it to the first variable that left its interval and offers that variable's fix.
5. Run the modeller's own reality checks (X6) here too.

### C6. Data and calibration

When data or a calibration are present, run C1 to C5 of the calibration family. The existing calibration module supplies parameter correlations for C2.

### C7. Report and gate

The report gives the findings with ranked fixes, plus a health summary in the style of SDM-Doc:

- model size and stocks;
- loops;
- units coverage;
- documentation coverage;
- findings by family.

It is graded against three gates:

| Gate | Requirement | Use |
|---|---|---|
| Draft | No E in families S, G, U | while building |
| Review | No E anywhere; W explained or fixed | before sharing with a client or colleague |
| Publish | Review, plus X1 to X5 passed, N3 passed, R1 to R4 complete, benchmarks met | for a report, paper or the book |

## Part D. Fix generation rules

1. **Every finding carries at least one concrete fix** as an operation list, or a clear statement that only the modeller can decide (for example F13, whether the goal really should erode).
2. **Fixes that keep the numbers rank first;** then the smallest edits.
3. **Fixes are never applied silently.** `--fix-all-safe` applies only first-ranked fixes that keep the numbers, as in the units spec.
4. **Applying a fix reruns the affected stages.** Fixes that introduce new findings are rolled back and reported.
5. **Each fix teaches.** Its text names the principle and the source, for example "First-order control: an outflow cannot take more than the stock holds (Sterman 2000; book section 5.x)". The doctor thereby doubles as a tutor.

## Part E. Testing the doctor

1. **A broken model for every rule.** For each of the 72 rules, the test corpus holds the smallest model that triggers it. The doctor must report that rule, and its first-ranked fix must remove the finding without creating another.
2. **False alarms.** Every model in the book must reach at least the Review gate, or its finding must be a genuine issue that the book then corrects. These models are the false-alarm benchmark.
3. **Known errors.** The units trap model and the earlier non-negative stock case must be caught.
4. **Speed.** Stages C1 to C3 must take under 200 ms for a model of 500 variables, so they can run on every edit. C4 and C5 run on demand, in the background.

## Sources

- Panko, R.R. Spreadsheet errors: what we know (arXiv 0802.3457), and Recommended practices for spreadsheet testing (arXiv 0712.0109); Caulkins, Morrison and Weidemann, Spreadsheet errors and decision making.
- Rahmandad, H. and Sterman, J.D. (2012) Reporting guidelines for simulation-based research in social sciences. System Dynamics Review 28(4), 396-411.
- Replicability of system dynamics studies, International System Dynamics Conference 2014, abstract 1056.
- Wakeland, W. and Hoarfrost, M. (2005) The case for thoroughly testing complex system dynamics models; Zagonel, A.A. and Corbet, T.F., Levels of confidence in system dynamics modeling (Sandia).
- Forrester, J.W. and Senge, P.M. (1980) Tests for building confidence in system dynamics models; Sterman, J.D. (2000) Business Dynamics, chapter 21.
- Barlas, Y. (1989) Tests of model behavior that can detect structural flaws; Barlas, Y. (1996) Formal aspects of model validity and validation in system dynamics. System Dynamics Review 12(3), 183-210; Barlas, Y. and Kanar, K. (2000) Structure-oriented behavior tests in model validation.
- Morecroft, J. (2007) Strategic Modelling and Business Dynamics, sections 10.3 and 10.6.
- Lai, D. and Wahba, R. (2001) System Dynamics Model Correctness Checklist, D-4851, MIT System Dynamics in Education Project.
- Rahmandad, H. Common modeling challenges, MIT OpenCourseWare 15.879 (2025).
- Lei, L. and Choge, N. (2001) A table function example, D-4587, MIT; Kleemann, Dianati and Pedercini (2022) Elasticity formulations, International System Dynamics Conference.
- Martinez-Moyano, I.J. and Richardson, G.P. (2013) Best practices in system dynamics modeling. System Dynamics Review 29(2), 102-123; Homer, J. (2019) Best practices in system dynamics modeling, revisited. System Dynamics Review 35(2).
- Vensim help: Euler integration; Runge-Kutta integration; Selecting an integration technique; Simultaneous equations; Check Model; Initialisation in equilibrium; Time step.
- isee systems: Integration methods and DT (blog); Troubleshooting circularities; Checking unit consistency.
- Oliva, R. (2003) Model calibration as a testing strategy for system dynamics models. European Journal of Operational Research 151(3), 552-568; Oliva, R. (2001) Formalization of model partition heuristics through graph theory.
- Homer, J.B. (1983, reprinted 2012) Partial-model testing as a validation tool for system dynamics. System Dynamics Review.
- Sterman, J.D. (1984) Appropriate summary statistics for evaluating the historical fit of system dynamics models. Dynamica 10(2).
- Peterson, D.W. and Eberlein, R.L. (1994) Reality check: a bridge between systems thinking and system dynamics. System Dynamics Review 10(2).
- Martinez-Moyano, I.J. (2012) Documentation for model transparency. System Dynamics Review 28(2) (SDM-Doc).
- Barlas, Topaloğlu and Yılankaya (1997) BTS; Boğ and Barlas (2005) Automated dynamic pattern testing; Sücüllü and Yücel (2014) BATS.
- NASA (1999) Mars Climate Orbiter Mishap Investigation Board, Phase I report.

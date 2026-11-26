//! F3 `reduce`: a smaller model that answers the same questions, by cut, verify, add back.
//!
//! The reduced model is a **new** model, linked to its source (`manifest.reduced_from`); the source is
//! never touched.
//!
//! # Gates (before any cut)
//!
//! For every goal: (1) Morris screening ranks the parameters the same with `r` and `2r` trajectories
//! (Kendall tau at least `morris_tau`, 0.9); if not, `r` is doubled once, then the goal is refused.
//! (2) Sobol total indices are reported with bootstrap intervals, lie in [0, 1] plus an estimator
//! tolerance, and keep their ranking when N doubles. If a goal fails, the transform declared for it in
//! the manifest (`transform`: "log") is applied and the gates rerun; with no transform declared, or
//! if it still fails, reduction is **refused** for that goal and, since the reduced model must serve
//! every goal, for the whole reduction (nothing is cut, no model is written).
//!
//! # Cuts, in this order
//!
//! 1. **unreachable elements**: variables with no causal path to any goal output (structure);
//! 2. **pass-through auxiliaries**: a variable whose formula is one other variable or one number is
//!    replaced by it (structure; lossless);
//! 3. **levers** whose total Sobol index is below `st_threshold` for every goal, by ascending index:
//!    the lever is fixed at its current value and kept as a named constant with its units (a *parameter fixed*, not structure);
//! 4. **loops** that never dominate are listed as proposals, never cut: removing a loop is a
//!    structural choice the engine cannot make safely; a proposal is only marked eligible when no loop
//!    of the model is unscored.
//!
//! Cuts are made in batches. After each batch the candidate model goes through [`crate::verify::verify`]
//! against the full model. A failing batch is bisected; a single cut that fails is **re-admitted**
//! permanently. The loop stops when nothing more can be cut. Small effects that add up are caught
//! this way: each of several levers may sit below the threshold while their joint removal fails.
//!
//! # Output
//!
//! Structure removed (elements, stocks, links, loops) is reported separately from parameters fixed
//! because they were inert in the scenarios tested (named in the result); the headline reduction
//! figure counts structure only.

use serde::Serialize;

use crate::expr::normalize;
use crate::manifest::Goal;
use crate::model::{Kind, Model};
use crate::screen::{Param, ScreenSpec};
use crate::sobol::{Sobol, SobolSpec};
use crate::verify::{Verdict, VerifySpec};

#[derive(Clone, Debug)]
pub struct ReduceSpec {
    /// a lever is fixed when its total Sobol index is below this for every goal
    pub st_threshold: f64,
    /// a loop whose largest relative score never exceeds this is listed as never mattering
    pub loop_threshold: f64,
    /// Kendall tau between the Morris rankings from r and 2r trajectories that the gate demands
    pub morris_tau: f64,
    /// ranking agreement of the Sobol total indices between N and 2N that the gate demands
    pub sobol_rank_tau: f64,
    /// cuts per verified batch for levers
    pub batch: usize,
    pub screen: ScreenSpec,
    pub sobol: SobolSpec,
    pub verify: VerifySpec,
}

impl Default for ReduceSpec {
    fn default() -> Self {
        ReduceSpec {
            st_threshold: 0.01,
            loop_threshold: 0.01,
            morris_tau: 0.9,
            sobol_rank_tau: 0.9,
            batch: 4,
            screen: ScreenSpec::default(),
            sobol: SobolSpec::default(),
            verify: VerifySpec { ensemble: 30, ..VerifySpec::default() },
        }
    }
}

/// One line of the log. `category` is `structure`, `parameter`, `readmitted` or `loop_proposal`.
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub category: String,
    pub step: String,
    pub target: String,
    pub action: String,
    pub reason: String,
    /// for `parameter`: the scenarios in which the parameter was found inert
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Counts {
    pub variables: usize,
    pub stocks: usize,
    pub levers: usize,
    pub loops: usize,
    pub links: usize,
}

/// Structure removed by the cuts that were made, as counts and names.
#[derive(Clone, Debug, Serialize)]
pub struct StructureRemoved {
    pub elements: Vec<String>,
    pub stocks: Vec<String>,
    pub links: usize,
    pub loops: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Headline {
    pub elements_before: usize,
    pub structure_removed: usize,
    /// structure only: parameters fixed do not count
    pub structure_reduction_pct: f64,
    pub parameters_fixed: usize,
}

/// What the gates found for one goal.
#[derive(Clone, Debug, Serialize)]
pub struct GateReport {
    pub goal: String,
    /// the transform the goal's numbers went through for these gates
    pub transform: String,
    pub morris_trajectories: usize,
    pub morris_tau: f64,
    pub morris_ok: bool,
    pub sobol_converged: bool,
    pub sobol_max_change_n_to_2n: f64,
    pub sobol_rank_tau: f64,
    pub sobol_indices_in_range: bool,
    /// largest bootstrap half-width of a total index
    pub sobol_max_st_conf: f64,
    pub passed: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Refusal {
    pub goal: String,
    pub reason: String,
}

/// A cut that was tried.
#[derive(Clone, Debug, Serialize)]
pub struct CutTry {
    pub cut: String,
    pub kind: String,
    pub batch: usize,
    /// `accepted` or `readmitted`
    pub outcome: String,
}

pub struct Reduction {
    pub model: Model,
    /// every cut made and every proposal, in the order made
    pub changes: Vec<Change>,
    pub structure_changes: Vec<Change>,
    pub parameter_changes: Vec<Change>,
    pub readmitted: Vec<Change>,
    pub loop_proposals: Vec<Change>,
    pub cuts_tried: Vec<CutTry>,
    pub structure_removed: StructureRemoved,
    pub headline: Headline,
    /// the scenarios the inertness of fixed parameters was tested in
    pub scenarios_tested: Vec<String>,
    pub before: Counts,
    pub after: Counts,
    pub gates: Vec<GateReport>,
    pub refused: Vec<Refusal>,
    /// number of calls of `verify_reduction`
    pub verifications: usize,
    /// model runs those calls made (full and candidate)
    pub verification_runs: usize,
    /// the last passing verdict (None when nothing was cut)
    pub verdict: Option<Verdict>,
    /// the Sobol result the lever ranking came from
    pub sobol: Option<Sobol>,
}

impl Reduction {
    pub fn refused(&self) -> bool {
        !self.refused.is_empty()
    }
}

fn counts(m: &Model) -> Counts {
    let loops = crate::loopscore::loop_scores(m).map(|s| s.loops.len()).unwrap_or(0);
    let links = crate::sim::compile(m).map(|c| crate::sim::influence_graph(&c).1.len()).unwrap_or(0);
    Counts { variables: m.vars.len(), stocks: m.vars.iter().filter(|v| v.kind == Kind::Stock).count(), levers: m.manifest.levers.len(), loops, links }
}

/// Replace every use of `name` in all equations by `with`, then drop the variable.
fn inline(m: &mut Model, name: &str, with: &str) {
    let key = normalize(name);
    let lit = if with.trim().parse::<f64>().is_ok_and(|x| x < 0.0) { format!("({})", with.trim()) } else if with.trim().parse::<f64>().is_ok() { with.trim().to_string() } else { format!("({})", with.trim()) };
    for v in m.vars.iter_mut() {
        v.equation = crate::expr::rename_in(&v.equation, &key, &lit);
        for it in v.by_item.iter_mut() {
            it.equation = crate::expr::rename_in(&it.equation, &key, &lit);
        }
    }
    if let Some(v) = m.by_name(&key) {
        let id = v.id;
        m.remove_var(id);
    }
}

/// `e` without balanced outer parentheses.
fn strip_parens(e: &str) -> &str {
    let mut s = e.trim();
    while s.starts_with('(') && s.ends_with(')') {
        let mut depth = 0i32;
        let mut closes_at_end = true;
        for (i, ch) in s.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && i != s.len() - 1 {
                        closes_at_end = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if closes_at_end { s = s[1..s.len() - 1].trim() } else { break }
    }
    s
}

/// What a pass-through auxiliary stands for: a number or the name of an existing variable.
fn pass_through(m: &Model, name: &str, protected: &[String]) -> Option<String> {
    let v = m.by_name(&normalize(name))?;
    if v.kind != Kind::Aux || v.set.is_some() || !v.by_item.is_empty() || protected.contains(&normalize(&v.name)) {
        return None;
    }
    if m.checks.iter().any(|c| normalize(&c.var) == normalize(&v.name)) || m.manifest.levers.iter().any(|l| normalize(&l.name) == normalize(&v.name)) || m.manifest.outputs.iter().any(|o| normalize(&o.variable) == normalize(&v.name)) {
        return None;
    }
    let e = strip_parens(&v.equation);
    if e.parse::<f64>().is_ok() {
        return Some(e.to_string());
    }
    let single = crate::expr::tokenize(e).map(|t| t.len() == 1 && matches!(t[0].0, crate::expr::Tok::Ident(_))).unwrap_or(false);
    (single && normalize(e) != normalize(name) && m.by_name(&normalize(e)).is_some()).then(|| e.to_string())
}

fn idents_of(v: &crate::model::Var) -> Vec<String> {
    let texts = std::iter::once(v.equation.clone()).chain(v.by_item.iter().map(|x| x.equation.clone()));
    let mut out = Vec::new();
    for t in texts {
        if let Ok(toks) = crate::expr::tokenize(&t) {
            for (tok, _) in toks {
                if let crate::expr::Tok::Ident(id) = tok {
                    out.push(normalize(&id));
                }
            }
        }
    }
    out
}

/// One thing that can be cut.
#[derive(Clone, Debug)]
enum Cut {
    /// an element with no path to a goal, with the unreachable elements that read it (removed first)
    Prune { name: String, dependents: Vec<String> },
    Merge { name: String },
    Fix { name: String },
}

impl Cut {
    fn name(&self) -> &str {
        match self {
            Cut::Prune { name, .. } | Cut::Merge { name } | Cut::Fix { name } => name,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Cut::Prune { .. } => "prune",
            Cut::Merge { .. } => "merge",
            Cut::Fix { .. } => "fix",
        }
    }
    fn is_structure(&self) -> bool {
        !matches!(self, Cut::Fix { .. })
    }
}

/// Apply a cut to `m`. `Ok(Some(action))` when it changed something, `Ok(None)` when it no longer applies.
fn apply(m: &mut Model, cut: &Cut, protected: &[String]) -> Option<String> {
    match cut {
        Cut::Prune { name, dependents } => {
            m.by_name(&normalize(name))?;
            for d in dependents.iter().rev().chain(std::iter::once(name)) {
                if let Some(v) = m.by_name(&normalize(d)) {
                    let id = v.id;
                    m.remove_var(id);
                }
            }
            Some("removed".into())
        }
        Cut::Merge { name } => {
            let with = pass_through(m, name, protected)?;
            inline(m, name, &with);
            Some(format!("replaced by {with}"))
        }
        Cut::Fix { name } => {
            let v = m.by_name(&normalize(name))?;
            if v.kind != Kind::Constant || v.set.is_some() || v.equation.trim().parse::<f64>().is_err() {
                return None;
            }
            let value = v.equation.trim().to_string();
            // the constant stays, with its name, its units and its value: only the lever goes (a number written into an
            // equation would lose the units the rest of the model reads it in)
            let key = normalize(name);
            m.manifest.levers.retain(|l| normalize(&l.name) != key);
            for sc in m.manifest.scenarios.iter_mut() {
                sc.values.retain(|k, _| normalize(k) != key);
            }
            if let Some(v) = m.by_name(&key) {
                let id = v.id;
                if let Some(v) = m.var_mut(id) {
                    v.lever = false;
                    v.slider = None;
                }
            }
            Some(format!("fixed at {value}, kept as a named constant"))
        }
    }
}

/// Remove from the manifest, checks and tests what no longer exists.
fn clean(m: &mut Model) -> Option<usize> {
    let have = |m: &Model, n: &str| m.by_name(&normalize(n)).is_some();
    let dropped_levers: Vec<String> = m.manifest.levers.iter().filter(|l| !have(m, &l.name)).map(|l| l.name.clone()).collect();
    let kept: Vec<_> = m.manifest.levers.iter().filter(|l| have(m, &l.name)).cloned().collect();
    m.manifest.levers = kept;
    for sc in m.manifest.scenarios.iter_mut() {
        sc.values.retain(|k, _| !dropped_levers.iter().any(|d| normalize(d) == normalize(k)));
    }
    let nvars = m.vars.clone();
    m.manifest.outputs.retain(|o| nvars.iter().any(|v| normalize(&v.name) == normalize(&o.variable)));
    let before = m.checks.len();
    m.checks.retain(|c| nvars.iter().any(|v| normalize(&v.name) == normalize(&c.var)));
    let removed = before - m.checks.len();
    m.manifest.tests.retain(|t| m.checks.iter().any(|c| normalize(&c.name) == normalize(t)));
    (removed > 0).then_some(removed)
}

/// Run the gates for every goal. `eff` carries the transform each goal is read through.
fn run_gates(model: &Model, params: &[Param], eff: &[Goal], spec: &ReduceSpec) -> Result<(Vec<GateReport>, Option<Sobol>), String> {
    if params.is_empty() {
        let reports = eff.iter().map(|g| GateReport { goal: g.name.clone(), transform: tr(g), morris_trajectories: 0, morris_tau: f64::NAN, morris_ok: true, sobol_converged: true, sobol_max_change_n_to_2n: 0.0, sobol_rank_tau: 1.0, sobol_indices_in_range: true, sobol_max_st_conf: 0.0, passed: true, reason: "no levers declared: no importance gates apply, only unreachable elements and pass-through variables are cut".into() }).collect();
        return Ok((reports, None));
    }
    let mut sspec = spec.screen.clone();
    sspec.converge_tau = spec.morris_tau;
    let mut scr = crate::screen::screen(model, params, eff, &sspec)?;
    let mut r = sspec.trajectories.max(2);
    if scr.goals.iter().any(|g| !g.converged) {
        r *= 2;
        sspec.trajectories = r;
        scr = crate::screen::screen(model, params, eff, &sspec)?;
    }
    let sob = crate::sobol::sobol(model, params, eff, &spec.sobol);
    let mut reports = Vec::new();
    for (gi, g) in eff.iter().enumerate() {
        let gs = &scr.goals[gi];
        let morris_ok = gs.converged;
        let mut rep = GateReport { goal: g.name.clone(), transform: tr(g), morris_trajectories: r, morris_tau: gs.kendall_r_vs_2r, morris_ok, sobol_converged: false, sobol_max_change_n_to_2n: f64::NAN, sobol_rank_tau: f64::NAN, sobol_indices_in_range: false, sobol_max_st_conf: f64::NAN, passed: false, reason: String::new() };
        let mut why: Vec<String> = Vec::new();
        if !morris_ok {
            why.push(format!("Morris ranking not stable: Kendall tau {:.2} between r and 2r trajectories is below {} even after doubling r to {r}", gs.kendall_r_vs_2r, spec.morris_tau));
        }
        match &sob {
            Ok(s) => {
                let gsb = &s.goals[gi];
                let in_range = gsb.indices.iter().all(|i| i.st.is_finite() && i.st >= -(0.05f64).max(i.st_conf) && i.st <= 1.0 + (0.05f64).max(i.st_conf));
                rep.sobol_converged = gsb.converged;
                rep.sobol_max_change_n_to_2n = gsb.max_change_n_to_2n;
                rep.sobol_rank_tau = gsb.rank_tau_n_to_2n;
                rep.sobol_indices_in_range = in_range;
                rep.sobol_max_st_conf = gsb.indices.iter().map(|i| i.st_conf).filter(|c| c.is_finite()).fold(0.0, f64::max);
                if !in_range {
                    let worst = gsb.indices.iter().map(|i| i.st).filter(|x| x.is_finite()).fold(0.0f64, f64::max);
                    why.push(format!("Sobol total indices outside [0, 1] beyond estimator tolerance (largest {worst:.2}): the output distribution is too heavy-tailed for variance-based indices"));
                }
                if !gsb.converged {
                    why.push(format!("Sobol indices moved by {:.3} when N doubled (tolerance {})", gsb.max_change_n_to_2n, spec.sobol.converge_tol));
                }
                if gsb.rank_tau_n_to_2n < spec.sobol_rank_tau {
                    why.push(format!("Sobol ranking of total indices changed when N doubled (Kendall tau {:.2} below {})", gsb.rank_tau_n_to_2n, spec.sobol_rank_tau));
                }
            }
            Err(e) => why.push(format!("Sobol analysis failed: {e}")),
        }
        rep.passed = why.is_empty();
        rep.reason = if rep.passed { "gates passed".into() } else { why.join("; ") };
        reports.push(rep);
    }
    Ok((reports, sob.ok()))
}

fn tr(g: &Goal) -> String {
    let t = g.transform.trim().to_lowercase();
    if t.is_empty() { "none".into() } else { t }
}

/// Is the goal's own numbers strictly positive and heavy tailed? Used only to say so in a refusal.
fn heavy_tail_hint(model: &Model, params: &[Param], g: &Goal) -> Option<String> {
    let mut rng = crate::screen::Rng(12345);
    let mut v: Vec<f64> = Vec::new();
    let raw = Goal { transform: "none".into(), ..g.clone() };
    for _ in 0..60 {
        let over: Vec<(String, String)> = params.iter().map(|p| (p.name.clone(), format!("{}", p.min + rng.next() * (p.max - p.min)))).collect();
        if let Some(x) = crate::runner::run_with(model, &over).ok().and_then(|r| crate::manifest::goal_value(&r, &raw)) {
            if x.is_finite() {
                v.push(x);
            }
        }
    }
    if v.len() < 20 || v.iter().any(|x| *x <= 0.0) {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let (med, max) = (v[v.len() / 2], v[v.len() - 1]);
    (max / med > 10.0).then(|| format!("goal “{}” is strictly positive and heavy tailed (largest value {:.1} times the median): declare `transform: \"log\"` for it in the manifest", g.name, max / med))
}

pub fn reduce(model: &Model, goals: &[Goal], spec: &ReduceSpec) -> Result<Reduction, String> {
    for g in goals {
        if !crate::manifest::valid_transform(&g.transform) {
            return Err(format!("goal “{}” has an unknown transform “{}” (none, log)", g.name, g.transform));
        }
    }
    let before = counts(model);
    let protected: Vec<String> = goals.iter().map(|g| normalize(&g.output)).collect();
    let params: Vec<Param> = model.manifest.levers.iter().map(|l| Param { name: l.name.clone(), min: l.min, max: l.max }).collect();

    // ---- gates: untransformed first, then the declared transform for the goals that fail
    let mut eff: Vec<Goal> = goals.iter().map(|g| Goal { transform: "none".into(), ..g.clone() }).collect();
    let (mut gates, mut sob) = run_gates(model, &params, &eff, spec)?;
    if gates.iter().any(|g| !g.passed) {
        let mut changed = false;
        for (i, g) in goals.iter().enumerate() {
            if !gates[i].passed && g.transform.trim().eq_ignore_ascii_case("log") {
                eff[i].transform = "log".into();
                changed = true;
            }
        }
        if changed {
            (gates, sob) = run_gates(model, &params, &eff, spec)?;
        }
    }
    let mut refused: Vec<Refusal> = Vec::new();
    for (i, g) in gates.iter().enumerate() {
        if !g.passed {
            let mut reason = g.reason.clone();
            let declared = goals[i].transform.trim().eq_ignore_ascii_case("log");
            if declared {
                reason += "; the declared log transform was applied and the gates still failed";
            } else {
                reason += "; no transform is declared for this goal";
                if let Some(h) = heavy_tail_hint(model, &params, &goals[i]) {
                    reason += &format!(" ({h})");
                }
            }
            refused.push(Refusal { goal: g.goal.clone(), reason });
        }
    }
    let mk_empty = |gates: Vec<GateReport>, refused: Vec<Refusal>, sob: Option<Sobol>| Reduction {
        model: model.clone(),
        changes: vec![],
        structure_changes: vec![],
        parameter_changes: vec![],
        readmitted: vec![],
        loop_proposals: vec![],
        cuts_tried: vec![],
        structure_removed: StructureRemoved { elements: vec![], stocks: vec![], links: 0, loops: 0 },
        headline: Headline { elements_before: before.variables, structure_removed: 0, structure_reduction_pct: 0.0, parameters_fixed: 0 },
        scenarios_tested: vec![],
        before: before.clone(),
        after: before.clone(),
        gates,
        refused,
        verifications: 0,
        verification_runs: 0,
        verdict: None,
        sobol: sob,
    };
    if !refused.is_empty() {
        return Ok(mk_empty(gates, refused, sob));
    }

    // ---- candidate cuts, ranked
    let outputs: Vec<String> = goals.iter().map(|g| g.output.clone()).collect();
    let dead_names: Vec<String> = crate::loopscore::unreachable_from_goals(model, &outputs)?.into_iter().filter(|n| model.by_name(&normalize(n)).is_some_and(|v| v.kind != Kind::Lookup)).collect();
    let dead_keys: Vec<String> = dead_names.iter().map(|n| normalize(n)).collect();
    let mut prune_cuts: Vec<Cut> = Vec::new();
    for n in &dead_names {
        // transitive readers among the dead elements, nearest first
        let mut deps: Vec<String> = Vec::new();
        let mut frontier = vec![normalize(n)];
        while let Some(x) = frontier.pop() {
            for (d, dk) in dead_names.iter().zip(&dead_keys) {
                if *dk != normalize(n) && !deps.contains(d) && model.by_name(dk).is_some_and(|v| idents_of(v).contains(&x)) {
                    deps.push(d.clone());
                    frontier.push(dk.clone());
                }
            }
        }
        prune_cuts.push(Cut::Prune { name: n.clone(), dependents: deps });
    }

    let mut current = model.clone();
    let mut accepted: Vec<Cut> = Vec::new();
    let mut tried: Vec<CutTry> = Vec::new();
    let mut readmitted: Vec<Change> = Vec::new();
    let mut verifications = 0usize;
    let mut verification_runs = 0usize;
    let mut last_verdict: Option<Verdict> = None;
    let mut batch_no = 0usize;
    let mut sc_tested: Vec<String> = vec!["base run".into()];
    if let Some(s) = &sob {
        sc_tested.push(format!("Sobol design over every lever's declared range ({} runs)", s.runs));
    }
    sc_tested.push(format!("verification ensemble of {} random lever draws over the declared ranges", spec.verify.ensemble));
    for g in goals.iter().filter(|g| g.protected_ranking) {
        for p in &g.policies {
            let l = format!("policy scenario “{p}”");
            if !sc_tested.contains(&l) {
                sc_tested.push(l);
            }
        }
    }

    struct Ctx<'a> {
        full: &'a Model,
        goals: &'a [Goal],
        spec: &'a ReduceSpec,
        protected: &'a [String],
    }
    let ctx = Ctx { full: model, goals: &eff, spec, protected: &protected };

    // try one batch on top of `current`; returns the candidate and the verdict when it passes
    fn attempt(ctx: &Ctx, current: &Model, batch: &[Cut], verifications: &mut usize, runs: &mut usize) -> Option<(Model, Verdict)> {
        let mut cand = current.clone();
        let mut changed = false;
        for c in batch {
            changed |= apply(&mut cand, c, ctx.protected).is_some();
        }
        if !changed {
            return None;
        }
        clean(&mut cand);
        if crate::sim::compile(&cand).is_err() {
            return None;
        }
        *verifications += 1;
        match crate::verify::verify(ctx.full, &cand, ctx.goals, &ctx.spec.verify) {
            Ok(v) => {
                *runs += 2 * v.runs;
                v.pass.then_some((cand, v))
            }
            Err(_) => None,
        }
    }

    // process a batch: accept whole, or bisect and re-admit single culprits
    #[allow(clippy::too_many_arguments)]
    fn process(ctx: &Ctx, batch: Vec<Cut>, current: &mut Model, accepted: &mut Vec<Cut>, tried: &mut Vec<CutTry>, readmitted: &mut Vec<Change>, verifications: &mut usize, runs: &mut usize, last: &mut Option<Verdict>, batch_no: &mut usize, sob: &Option<Sobol>) {
        if batch.is_empty() {
            return;
        }
        // cuts that no longer apply are dropped silently (a merge made redundant by an earlier one)
        let live: Vec<Cut> = batch.into_iter().filter(|c| { let mut t = current.clone(); apply(&mut t, c, ctx.protected).is_some() }).collect();
        if live.is_empty() {
            return;
        }
        *batch_no += 1;
        let no = *batch_no;
        if let Some((cand, v)) = attempt(ctx, current, &live, verifications, runs) {
            *current = cand;
            *last = Some(v);
            for c in live {
                tried.push(CutTry { cut: c.name().to_string(), kind: c.kind().into(), batch: no, outcome: "accepted".into() });
                accepted.push(c);
            }
            return;
        }
        if live.len() == 1 {
            let c = &live[0];
            tried.push(CutTry { cut: c.name().to_string(), kind: c.kind().into(), batch: no, outcome: "readmitted".into() });
            let st = sob.as_ref().and_then(|s| lever_st(s, c.name()));
            readmitted.push(Change { category: "readmitted".into(), step: c.kind().into(), target: c.name().to_string(), action: "kept".into(), reason: format!("cutting it alone failed verification{}", st.map(|x| format!(" (its total Sobol index {x:.4} was below the threshold)")).unwrap_or_default()), scenarios: vec![] });
            return;
        }
        let mid = live.len() / 2;
        let (l, r) = (live[..mid].to_vec(), live[mid..].to_vec());
        process(ctx, l, current, accepted, tried, readmitted, verifications, runs, last, batch_no, sob);
        process(ctx, r, current, accepted, tried, readmitted, verifications, runs, last, batch_no, sob);
    }

    // 1. unreachable elements, in one batch
    process(&ctx, prune_cuts, &mut current, &mut accepted, &mut tried, &mut readmitted, &mut verifications, &mut verification_runs, &mut last_verdict, &mut batch_no, &sob);

    // 2. pass-through auxiliaries until none is left that can be merged
    let mut merge_seen: Vec<String> = Vec::new();
    loop {
        let cands: Vec<Cut> = current.vars.iter().filter(|v| !merge_seen.contains(&normalize(&v.name)) && pass_through(&current, &v.name, &protected).is_some()).map(|v| Cut::Merge { name: v.name.clone() }).collect();
        if cands.is_empty() {
            break;
        }
        merge_seen.extend(cands.iter().map(|c| normalize(c.name())));
        process(&ctx, cands, &mut current, &mut accepted, &mut tried, &mut readmitted, &mut verifications, &mut verification_runs, &mut last_verdict, &mut batch_no, &sob);
    }

    // 3. levers by ascending total Sobol index (largest over goals), in batches
    if let Some(s) = &sob {
        let mut lev: Vec<(String, f64)> = Vec::new();
        for l in &model.manifest.levers {
            if current.by_name(&normalize(&l.name)).is_none() {
                continue;
            }
            let st: Vec<f64> = s.goals.iter().filter_map(|g| g.indices.iter().find(|i| normalize(&i.param) == normalize(&l.name)).map(|i| i.st)).collect();
            if st.is_empty() || st.iter().any(|x| *x >= spec.st_threshold) {
                continue;
            }
            lev.push((l.name.clone(), st.iter().cloned().fold(f64::NEG_INFINITY, f64::max)));
        }
        lev.sort_by(|a, b| a.1.total_cmp(&b.1));
        for chunk in lev.chunks(spec.batch.max(1)) {
            let cuts: Vec<Cut> = chunk.iter().map(|(n, _)| Cut::Fix { name: n.clone() }).collect();
            process(&ctx, cuts, &mut current, &mut accepted, &mut tried, &mut readmitted, &mut verifications, &mut verification_runs, &mut last_verdict, &mut batch_no, &sob);
        }
    }

    // ---- the log, split into structure and parameters
    let mut structure_changes: Vec<Change> = Vec::new();
    let mut parameter_changes: Vec<Change> = Vec::new();
    // replay on a scratch copy to log the action text and the structure-only model
    let mut replay = model.clone();
    let mut struct_only = model.clone();
    let mut removed_elements: Vec<String> = Vec::new();
    let mut removed_stocks: Vec<String> = Vec::new();
    for c in &accepted {
        let names_before: Vec<(String, bool)> = replay.vars.iter().map(|v| (v.name.clone(), v.kind == Kind::Stock)).collect();
        let Some(action) = apply(&mut replay, c, &protected) else { continue };
        let st = sob.as_ref().and_then(|s| lever_st(s, c.name()));
        if c.is_structure() {
            apply(&mut struct_only, c, &protected);
            let (reason, step) = match c {
                Cut::Prune { .. } => ("no causal path to any goal output", "prune"),
                _ => ("its formula is a single variable or number", "merge"),
            };
            for (n, is_stock) in names_before {
                if replay.by_name(&normalize(&n)).is_none() {
                    removed_elements.push(n.clone());
                    if is_stock {
                        removed_stocks.push(n.clone());
                    }
                    let act = if matches!(c, Cut::Merge { .. }) { action.clone() } else { "removed".to_string() };
                    structure_changes.push(Change { category: "structure".into(), step: step.into(), target: n, action: act, reason: reason.into(), scenarios: vec![] });
                }
            }
        } else {
            let reason = format!("total Sobol index {:.4} (largest over goals) is below {}; inert in the scenarios tested", st.unwrap_or(f64::NAN), spec.st_threshold);
            parameter_changes.push(Change { category: "parameter".into(), step: "fix".into(), target: c.name().to_string(), action, reason, scenarios: sc_tested.clone() });
        }
    }
    removed_elements.dedup();
    clean(&mut struct_only);

    // finish the model
    let dropped_checks = clean(&mut current);
    if let Some(n) = dropped_checks {
        structure_changes.push(Change { category: "structure".into(), step: "prune".into(), target: "checks".into(), action: format!("{n} removed"), reason: "they watch variables that were removed".into(), scenarios: vec![] });
    }
    current.manifest.reduced_from = if model.manifest.title.is_empty() { model.title.clone() } else { model.manifest.title.clone() };
    if !current.title.ends_with("(reduced)") {
        current.title = format!("{} (reduced)", current.title).trim().to_string();
    }
    if let Err(p) = crate::sim::compile(&current) {
        return Err(format!("the reduced model does not compile ({}): nothing is written", p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("; ")));
    }

    // 4. loops that never dominate: proposals only
    let mut loop_proposals: Vec<Change> = Vec::new();
    if let Ok(s) = crate::loopscore::loop_scores(&current) {
        let blocked = s.unscored_loops > 0 || s.truncated;
        for (i, l) in s.loops.iter().enumerate() {
            if !l.scored {
                continue;
            }
            let top = s.relative[i].iter().fold(0.0f64, |a, b| a.max(b.abs()));
            if top < spec.loop_threshold {
                let (action, why) = if blocked {
                    ("blocked", format!("never above {:.1}% of the loop scores, but {} loop(s) of the model are unscored{}: dominance cannot justify a cut", spec.loop_threshold * 100.0, s.unscored_loops, if s.truncated { " and the loop list is truncated" } else { "" }))
                } else {
                    ("proposed", format!("never above {:.1}% of the loop scores in the base run and every loop is scored; removing a loop is a structural choice the engine does not make: test it with verify_reduction", spec.loop_threshold * 100.0))
                };
                loop_proposals.push(Change { category: "loop_proposal".into(), step: "loops".into(), target: l.variables.join(" > "), action: action.into(), reason: why, scenarios: vec![] });
            }
        }
    }

    let after = counts(&current);
    let so = counts(&struct_only);
    let n_struct = removed_elements.len();
    let structure_removed = StructureRemoved { elements: removed_elements, stocks: removed_stocks, links: before.links.saturating_sub(so.links), loops: before.loops.saturating_sub(so.loops) };
    let headline = Headline { elements_before: before.variables, structure_removed: n_struct, structure_reduction_pct: if before.variables > 0 { 100.0 * n_struct as f64 / before.variables as f64 } else { 0.0 }, parameters_fixed: parameter_changes.len() };
    let mut changes: Vec<Change> = structure_changes.iter().chain(parameter_changes.iter()).chain(readmitted.iter()).chain(loop_proposals.iter()).cloned().collect();
    changes.shrink_to_fit();
    Ok(Reduction {
        model: current,
        changes,
        structure_changes,
        parameter_changes,
        readmitted,
        loop_proposals,
        cuts_tried: tried,
        structure_removed,
        headline,
        scenarios_tested: sc_tested,
        before,
        after,
        gates,
        refused,
        verifications,
        verification_runs,
        verdict: last_verdict,
        sobol: sob,
    })
}

fn lever_st(s: &Sobol, name: &str) -> Option<f64> {
    let v: Vec<f64> = s.goals.iter().filter_map(|g| g.indices.iter().find(|i| normalize(&i.param) == normalize(name)).map(|i| i.st)).collect();
    (!v.is_empty()).then(|| v.into_iter().fold(f64::NEG_INFINITY, f64::max))
}

pub fn changes_csv(c: &[Change]) -> String {
    let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut o = String::from("category,step,target,action,reason,scenarios\n");
    for x in c {
        o += &format!("{},{},{},{},{},{}\n", x.category, x.step, q(&x.target), q(&x.action), q(&x.reason), q(&x.scenarios.join("; ")));
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outer_parentheses_are_stripped_only_when_they_pair() {
        assert_eq!(strip_parens("((A))"), "A");
        assert_eq!(strip_parens("(A) + (B)"), "(A) + (B)");
        assert_eq!(strip_parens(" ( -3 ) "), "-3");
    }

    #[test]
    fn tolerance_ties_make_equal_effects_rank_stable() {
        use crate::screen::{kendall_tau, kendall_tau_tied};
        let (a, b) = ([1.0, 1.001, 0.999, 0.1], [1.002, 0.998, 1.0, 0.1]);
        assert!(kendall_tau(&a, &b) < 0.9);
        let e = [0.02; 4];
        assert!((kendall_tau_tied(&a, &b, &e, &e) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_fixed_constant_stays_a_named_constant_with_its_units() {
        let mut m = Model::default();
        let mut t = crate::model::Var::new(1, "Production Time", Kind::Constant, [0.0, 0.0]);
        t.equation = "4".into();
        t.units = "Week".into();
        let mut a = crate::model::Var::new(2, "Output", Kind::Aux, [0.0, 0.0]);
        a.equation = "Production Time * 2".into();
        m.vars = vec![t, a];
        m.next_id = 10;
        let got = apply(&mut m, &Cut::Fix { name: "Production Time".into() }, &[]).unwrap();
        assert!(got.contains("named constant"), "{got}");
        assert_eq!(m.by_name("production time").unwrap().units, "Week");
        assert_eq!(m.by_name("output").unwrap().equation, "Production Time * 2");
    }
}

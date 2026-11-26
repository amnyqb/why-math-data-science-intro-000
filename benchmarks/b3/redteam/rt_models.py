"""Red-team models for dyn reduce / dyn verify (round 2). Run on the server:
  FC=/opt/dynamist/target/release/dyn python3 rt_models.py [case ...]
Writes models and results to ./out/ . Never edits core/cli/python."""
import json, os, subprocess, sys, tempfile
FC = os.environ.get("FC", "/opt/dynamist/target/release/dyn")
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "out")
os.makedirs(OUT, exist_ok=True)


def build(name, ops, levers, goals, scenarios=None, final=1, step=1):
    path = os.path.join(OUT, name + ".sdm")
    if os.path.exists(path):
        os.unlink(path)
    allops = [{"op": "set_setting", "key": "final_time", "value": str(final)}, {"op": "set_setting", "key": "time_step", "value": str(step)}] + ops
    mf = {"levers": levers, "goals": goals}
    if scenarios:
        mf["scenarios"] = scenarios
    allops.append({"op": "set_manifest", "manifest": mf})
    f = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False)
    json.dump(allops, f)
    f.close()
    r = subprocess.run([FC, "ops", path, f.name], capture_output=True, text=True)
    os.unlink(f.name)
    if '"ok": true' not in r.stdout:
        raise SystemExit(name + ": " + r.stdout[:600])
    return path


def const(n, v): return {"op": "add_var", "name": n, "kind": "constant", "equation": str(v)}
def aux(n, e): return {"op": "add_var", "name": n, "kind": "aux", "equation": e}
def stock(n, e): return {"op": "add_var", "name": n, "kind": "stock", "equation": e}


def flow(n, e, to=None, frm=None):
    d = {"op": "add_var", "name": n, "kind": "flow", "equation": e}
    if to:
        d["to"] = to
    if frm:
        d["from"] = frm
    return d


def lev(n, lo, hi): return {"name": n, "min": lo, "max": hi}
def goal(out="Y", name="y", **kw): return dict({"name": name, "output": out, "measure": "final", "direction": "max"}, **kw)


def run(*args, timeout=240):
    cmd = ["nice", FC] + [str(a) for a in args]
    if args[0] == "reduce":
        cmd += ["--workers", "4"]
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    try:
        return r.returncode, json.loads(r.stdout)
    except Exception:
        return r.returncode, {"raw": (r.stdout + r.stderr)[:800]}


def summarize(rc, r):
    if "gates" not in r:
        return {"rc": rc, **r}
    return {"rc": rc, "ok": r.get("ok"), "refused": r.get("refused"),
            "gates": [{k: g[k] for k in ("goal", "transform", "passed", "morris_tau", "sobol_rank_tau", "sobol_converged", "reason")} for g in r["gates"]],
            "struct": [(c["target"], c["step"]) for c in r["structure_removed"]["changes"]],
            "fixed": [c["target"] for c in r["parameters_fixed"]["changes"]],
            "readmitted": [c["target"] for c in r["readmitted"]],
            "loops": [(c["target"], c["action"]) for c in r["loop_proposals"]],
            "headline": r["headline"], "verifs": r["verifications"], "verdict_pass": (r.get("verdict") or {}).get("pass")}


def reduce_case(tag, path, *extra, indep=True):
    out = os.path.join(OUT, tag + "_reduced.sdm")
    if os.path.exists(out):
        os.unlink(out)
    rc, r = run("reduce", path, "--out", out, *extra)
    s = summarize(rc, r)
    if rc == 0 and os.path.exists(out) and indep:
        vrc, v = run("verify", path, out, "--ensemble", 300, "--seed", 7)
        if "goals" in v:
            s["indep_verify"] = {"rc": vrc, "pass": v.get("pass"),
                                 "goals": [{k: g.get(k) for k in ("goal", "ensemble_nrmse_mean", "mode_agreement", "goal_value_tau", "pass_error", "pass_tau", "pass_modes")} for g in v["goals"]],
                                 "policies": [{k: p.get(k) for k in ("tau", "pass", "skipped")} for p in v.get("policies", [])]}
        else:
            s["indep_verify"] = v
    json.dump(r, open(os.path.join(OUT, tag + "_full.json"), "w"))
    print(tag, json.dumps(s)[:2500])
    sys.stdout.flush()
    return s


CASES = {}


def case(f):
    CASES[f.__name__] = f
    return f


@case
def c1_inert():
    ops = [const(f"R{i}", 0.5) for i in range(3)] + [const(f"Z{i}", 0.5) for i in range(6)]
    ops += [aux("Dead1", "R0 * 2"), aux("Dead2", "Dead1 + Z0"), aux("Dead3", "Dead2 * R1"),
            stock("DeadS", "1"), flow("DeadF", "DeadS * 0.1 + Z1", to="DeadS"),
            aux("Zero Term", "0 * (Z2 + Z3 + Z4 + Z5)"),
            aux("Y", "10 * R0 + 6 * R1 + 3 * R2 + Zero Term")]
    levers = [lev(f"R{i}", 0, 1) for i in range(3)] + [lev(f"Z{i}", 0, 1) for i in range(6)]
    p = build("c1_inert", ops, levers, [goal()])
    reduce_case("c1", p, "--samples", 1024)


@case
def c2_many_small():
    n = 80
    c = 0.32
    xs = ["X0"] + [f"X{i}" for i in range(1, n + 1)]
    ops = [const(x, 0.5) for x in xs] + [aux("Y", "10 * X0 + %s * (%s)" % (c, " + ".join(xs[1:])))]
    p = build("c2_many_small", ops, [lev(x, 0, 1) for x in xs], [goal()])
    reduce_case("c2", p, "--samples", 1024, "--ensemble", 40)
    reduce_case("c2b_thr1", p, "--samples", 1024, "--ensemble", 30, "--st-threshold", 1.1)


@case
def c3_policy():
    ops = [const("A", 0.5), const("S", 0.0), const("B", 0.5), aux("Y", "10 * A + 5 * B + 0.3 * S")]
    sc = [{"name": "Base", "values": {"A": 0.5}}, {"name": "Switch on", "values": {"A": 0.5, "S": 1.0}}]
    lv = [lev("A", 0, 1), lev("S", 0, 1), lev("B", 0, 1)]
    p = build("c3_policy", ops, lv, [goal(policies=["Base", "Switch on"], protected_ranking=True)], sc)
    reduce_case("c3", p, "--samples", 2048)
    p2 = build("c3_nopolicy", ops, lv, [goal()], sc)
    reduce_case("c3_noprot", p2, "--samples", 2048)
    # a policy that ranks by an effect only the fixed lever carries, through a lever the reduced model keeps:
    # P1 and P2 differ in A only, but Y has a tiny interaction A*S that reverses their order when S is at its top
    ops4 = [const("A", 0.5), const("S", 0.0), aux("Y", "10 * A - 0.2 * A * S * 10 + 0.4 * S")]
    sc4 = [{"name": "P1", "values": {"A": 0.2, "S": 1.0}}, {"name": "P2", "values": {"A": 0.3, "S": 1.0}}]
    p4 = build("c3_rank", ops4, [lev("A", 0, 1), lev("S", 0, 1)], [goal(policies=["P1", "P2"], protected_ranking=True)], sc4)
    reduce_case("c3_rank", p4, "--samples", 2048)


@case
def c4_heavy():
    for ft in (20, 40, 60, 80):
        for tr in (None, "log"):
            names = ["R1", "R2", "R3", "R4", "Noise"]
            ops = [const(n, 0.5 if n == "Noise" else 0.03) for n in names]
            ops += [aux("Rate", "R1 + R2 + R3 + R4 + 0.00001 * Noise"), stock("Pop", "1"), flow("Growth", "Pop * Rate", to="Pop")]
            g = goal("Pop", "pop", **({"transform": tr} if tr else {}))
            p = build(f"c4_ft{ft}_{tr}", ops, [lev(n, 0, 1 if n == "Noise" else 0.06) for n in names], [g], final=ft, step=0.25)
            reduce_case(f"c4_ft{ft}_{tr}", p, "--samples", 512)


@case
def c5_nonpositive():
    for off, lo in ((1.5, -0.05), (0.9, -0.01), (0.5, 0.0)):
        ops = [const("R1", 0.02), const("Noise", 0.5), stock("Pop", "1"), flow("Growth", "Pop * (R1 + 0.00001 * Noise)", to="Pop"), aux("Y", f"Pop - {off}")]
        p = build(f"c5_off{off}", ops, [lev("R1", lo, 0.05), lev("Noise", 0, 1)], [goal("Y", "y", transform="log")], final=60, step=0.25)
        reduce_case(f"c5_off{off}", p, "--samples", 512)


@case
def c6_stateful():
    ops = [const("Gain", 0.05), const("Lag", 4), const("Init", 2), const("Tw", 3), const("Q Rate", 0.5), const("Dead In", 1), const("Side W", 0.0),
           stock("Pop", "10"), aux("Late", "DELAY FIXED(Pop, Lag, Init)"), aux("Trd", "TREND(Pop, Tw, 0.02)"),
           flow("Net", "Gain * Late - 0.04 * Pop + Trd * Side W", to="Pop"),
           stock("Backlog", "0"), flow("Arr", "Pop * 0.01", to="Backlog"), flow("Srv", "Backlog * Q Rate", frm="Backlog"),
           aux("Q Mix", "QUEUE FIFO(Arr, Srv, 0)"), aux("Copy", "Late"),
           aux("Dead Delay", "DELAY FIXED(Dead In, 2, 0)"),
           aux("Y", "Pop + 0.001 * Backlog + 0 * Copy")]
    lv = [lev("Gain", 0.02, 0.08), lev("Lag", 2, 8), lev("Init", 0, 20), lev("Tw", 1, 6), lev("Q Rate", 0.1, 1), lev("Dead In", 0, 1), lev("Side W", 0, 1)]
    p = build("c6_stateful", ops, lv, [goal("Y")], final=40, step=0.25)
    rc, r = run("loops", p)
    print("c6 loops:", json.dumps(r)[:600])
    reduce_case("c6", p, "--samples", 512)
    reduce_case("c6_thr1", p, "--samples", 512, "--st-threshold", 1.1)


@case
def c7_interaction():
    ops = [const("X1", 0), const("X2", 0), const("X3", 0.5), const("X4", -2), aux("Y", "X1 * X2 * 10 + 0.01 * X3 + 0.001 * X4 ^ 2")]
    lv = [lev("X1", -1, 1), lev("X2", -1, 1), lev("X3", 0, 1), lev("X4", -3, -1)]
    p = build("c7_interact", ops, lv, [goal()])
    reduce_case("c7", p, "--samples", 1024)
    reduce_case("c7_thr1", p, "--samples", 1024, "--st-threshold", 1.1)
    ops2 = [const("X1", 0.5), const("X2", 0.5), aux("Y", "10 * (X1 - 0.5) * (X2 - 0.5) + 0.01 * X1")]
    p2 = build("c7b", ops2, [lev("X1", 0, 1), lev("X2", 0, 1)], [goal()])
    reduce_case("c7b", p2, "--samples", 1024)
    reduce_case("c7b_4096", p2, "--samples", 4096)


@case
def c8_constant():
    ops = [const("X1", 0.5), const("X2", 0.5), aux("Dead", "X1"), aux("Y", "5 + 0 * X1 * X2")]
    p = build("c8_const", ops, [lev("X1", 0, 1), lev("X2", 0, 1)], [goal()])
    reduce_case("c8", p, "--samples", 512)
    v = run("verify", p, p, "--ensemble", 20)
    print("c8 verify self:", json.dumps(v)[:500])
    p2 = build("c8_const_nolev", [const("X1", 0.5), aux("Y", "5")], [], [goal()])
    reduce_case("c8b_nolev", p2, "--samples", 512)


@case
def c9_init_delay():
    ops = [const("I0", 50), const("Lag", 4), const("K", 0.3), const("Unused", 1),
           stock("S", "I0"), aux("Late", "DELAY FIXED(S, Lag, S)"), flow("Out", "K * Late", frm="S"), aux("Y", "S")]
    lv = [lev("I0", 0, 100), lev("Lag", 3, 5), lev("K", 0.1, 0.2), lev("Unused", 0, 1)]
    p = build("c9_init_delay", ops, lv, [goal("Y", measure="at:12")], final=30, step=0.25)
    reduce_case("c9", p, "--samples", 4096)
    reduce_case("c9_thr1", p, "--samples", 4096, "--st-threshold", 1.1)
    ops2 = [const("I0", 50), const("D", 3), stock("S", "I0"), flow("F", "0", to="S"), aux("Y", "S + 0 * D")]
    p2 = build("c9b", ops2, [lev("I0", 0, 100), lev("D", 1, 5)], [goal()], final=10)
    reduce_case("c9b", p2, "--samples", 512)


@case
def c6b_unscored_path():
    """A weak loop through QUEUE FIFO that reduce is tempted to call never-dominant; a lever only that loop reads."""
    ops = [const("Gain", 0.05), const("QW", 0.0), const("Q Rate", 0.5), stock("Pop", "10"), stock("Backlog", "0"),
           flow("Arr", "Pop * 0.01", to="Backlog"), flow("Srv", "Backlog * Q Rate", frm="Backlog"),
           aux("Q Mix", "QUEUE FIFO(Arr, Srv, 0)"), flow("Net", "Gain * Pop + QW * Q Mix", to="Pop"),
           aux("Y", "Pop")]
    p = build("c6b_unscored", ops, [lev("Gain", 0.01, 0.04), lev("QW", 0, 0.05), lev("Q Rate", 0.1, 1)], [goal("Y")], final=40, step=0.25)
    rc, r = run("loops", p)
    print("c6b loops:", json.dumps(r)[:900])
    reduce_case("c6b", p, "--samples", 2048)
    reduce_case("c6b_thr1", p, "--samples", 2048, "--st-threshold", 1.1)


if __name__ == "__main__":
    for k in (sys.argv[1:] or CASES):
        CASES[k]()

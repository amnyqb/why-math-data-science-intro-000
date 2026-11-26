"""Formula models for testing the analysis functions: constants X1.. over a range, goal Y = formula."""
import json, os, subprocess, tempfile

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
FC = os.environ.get("FC", os.path.join(REPO, "target", "debug", "dyn"))


def formula_model(path, formula, xs, lo=0.0, hi=1.0, nominal=None):
    ops = [{"op": "set_setting", "key": "final_time", "value": "1"}, {"op": "set_setting", "key": "time_step", "value": "1"}]
    for x in xs:
        ops.append({"op": "add_var", "name": x, "kind": "constant", "equation": str(nominal if nominal is not None else (lo + hi) / 2)})
    ops.append({"op": "add_var", "name": "Y", "kind": "aux", "equation": formula})
    ops.append({"op": "set_manifest", "manifest": {
        "levers": [{"name": x, "min": lo, "max": hi} for x in xs],
        "goals": [{"name": "y", "output": "Y", "measure": "final", "direction": "max"}]}})
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(ops, f)
    if os.path.exists(path):
        os.unlink(path)
    r = subprocess.run([FC, "ops", path, f.name], capture_output=True, text=True)
    os.unlink(f.name)
    out = json.loads(r.stdout)
    assert out.get("ok", True) and r.returncode == 0, r.stdout
    return path


def fc_json(*args):
    r = subprocess.run([FC, *map(str, args)], capture_output=True, text=True)
    assert r.returncode == 0, r.stdout + r.stderr
    return json.loads(r.stdout)

"""Drive Dynamist models from Python (notebooks, scripts, CI).

This talks to the `dyn` command-line tool, which prints JSON, so it needs nothing but the
standard library. Point FC_BIN at the binary (default: `dyn` on the PATH).

    import dynamist as fc
    m = fc.Model("tank.sdm")
    m.render("fig.svg", sheet="Model", with_caption=True)      # the picture exactly as the app draws it
    m.apply([{"op": "set_equation", "name": "Burn rate", "equation": "5"}])   # validated, atomic
    print(m.validate()["ok"], m.run(series=["Tank"])["series"][0]["last"])
    band = m.sensitivity("Tank", runs=200, spread=0.1)
"""

import json
import os
import subprocess

FC_BIN = os.environ.get("FC_BIN", "dyn")


class DynamistError(Exception):
    pass


def _fc(*args, check=True):
    p = subprocess.run([FC_BIN, *map(str, args)], capture_output=True, text=True)
    try:
        out = json.loads(p.stdout) if p.stdout.strip() else {}
    except json.JSONDecodeError:
        raise DynamistError(p.stdout + p.stderr)
    if check and p.returncode == 2:
        raise DynamistError(out.get("error", p.stderr))
    return out, p.returncode


class Model:
    def __init__(self, path):
        self.path = str(path)

    def info(self):
        return _fc("info", self.path)[0]

    def validate(self):
        """Issues with severity, code, variable and message. `ok` is False when there are errors."""
        return _fc("validate", self.path)[0]

    def test(self):
        """Only the model's own checks and stock conservation."""
        return _fc("test", self.path)[0]

    def run(self, overrides=None, series=None, every=0):
        args = ["run", self.path]
        for k, v in (overrides or {}).items():
            args += ["--set", f"{k}={v}"]
        if series:
            args += ["--series", ",".join(series)]
        if every:
            args += ["--every", every]
        return _fc(*args)[0]

    def sensitivity(self, var, runs=100, spread=0.2, seed=1):
        return _fc("sens", self.path, "--var", var, "--runs", runs, "--spread", spread, "--seed", seed)[0]

    def screen(self, goals=(), params=(), trajectories=20, levels=4, seed=1, tau=0.1, design=False):
        """Morris screening of the manifest's levers against its goals (see wiki/Find-What-Matters.md)."""
        args = ["screen", self.path, "--trajectories", trajectories, "--levels", levels, "--seed", seed, "--tau", tau]
        for g in goals:
            args += ["--goal", g]
        for p in params:
            args += ["--param", p]
        if design:
            args.append("--design")
        return _fc(*args)[0]

    def importance(self, goals=(), params=(), samples=512, seed=1, ensemble=30, eigen_at=None):
        """Sobol indices, loop dominance, removable variables (see wiki/Find-What-Matters.md)."""
        args = ["importance", self.path, "--samples", samples, "--seed", seed, "--ensemble", ensemble]
        for g in goals:
            args += ["--goal", g]
        for p in params:
            args += ["--param", p]
        if eigen_at is not None:
            args += ["--eigen-at", eigen_at]
        return _fc(*args)[0]

    def dominance(self):
        """Which feedback loop dominates when (Loops That Matter), base run."""
        return _fc("dominance", self.path)[0]

    def reduce(self, out, goals=(), samples=512, st_threshold=0.01, log=None):
        """A smaller, linked model that keeps the goals; writes it to `out` (the source is untouched)."""
        args = ["reduce", self.path, "--out", out, "--samples", samples, "--st-threshold", st_threshold]
        for g in goals:
            args += ["--goal", g]
        if log:
            args += ["--log", log]
        return _fc(*args)[0]

    def verify(self, reduced_path, goals=(), ensemble=50, seed=1):
        """Does the reduced model still answer the same? Returns the verdict (raises if the call fails)."""
        args = ["verify", self.path, reduced_path, "--ensemble", ensemble, "--seed", seed]
        for g in goals:
            args += ["--goal", g]
        return _fc(*args)[0]

    def recommend(self, roles, structures=(), goals=(), ensemble=20):
        """Proposals for structure that would improve the goals (nothing is applied). `roles` maps role names to variables."""
        args = ["recommend", self.path, "--ensemble", ensemble]
        for k, v in roles.items():
            args += ["--role", f"{k}={v}"]
        for st in structures:
            args += ["--structure", st]
        for g in goals:
            args += ["--goal", g]
        return _fc(*args)[0]

    def apply(self, ops, actor="python", gate=True):
        """Apply a batch atomically. Raises DynamistError with the problems if it is refused."""
        import tempfile

        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump(ops, f)
        try:
            args = ["ops", self.path, f.name, "--actor", actor] + ([] if gate else ["--no-gate"])
            out, code = _fc(*args, check=False)
        finally:
            os.unlink(f.name)
        if not out.get("ok"):
            raise DynamistError(json.dumps(out.get("rejected", out), indent=2))
        return out

    def variables(self, kind=None):
        return _fc("list", self.path, *(["--kind", kind] if kind else []))[0]

    def variable(self, name):
        return _fc("get", self.path, name)[0]

    def dependencies(self, name):
        return _fc("deps", self.path, name)[0]

    def loops(self):
        return _fc("loops", self.path)[0]

    def diff(self, other, runs=False):
        return _fc("diff", self.path, str(other), *(["--runs"] if runs else []))[0]

    def convert(self, out_path, format=None):
        """Write the model in another format; returns the loss report."""
        return _fc("convert", self.path, str(out_path), *(["--format", format] if format else []))[0]

    # ------------------------------------------------------------------ figures (headless; see wiki/Figure-Library.md)

    def render(self, out, sheet=None, png=False, scale=3, with_caption=False, style=None):
        """Draw one sheet (stock and flow, or causal loop) as SVG, the way the app draws it.

        `out` ending in .png writes only a PNG; `png=True` with an .svg name writes both. Returns the paths and size."""
        args = ["render", self.path, "--out", out]
        if sheet:
            args += ["--sheet", sheet]
        if png:
            args += ["--png", "--scale", scale]
        if with_caption:
            args.append("--with-caption")
        if style:
            args += ["--style", style]
        return _fc(*args)[0]

    def chart(self, vars, out, start=None, end=None, kind=None, title=None, y_label=None, scenarios=None, set=None,
              png=False, scale=3, style=None):
        """Variables over time as SVG. `start`/`end` pick a time window; `scenarios` is a list of names, "all", or None."""
        args = ["chart", self.path, "--vars", ",".join(vars), "--out", out]
        if start is not None:
            args += ["--from", start]
        if end is not None:
            args += ["--to", end]
        if kind:
            args += ["--kind", kind]
        if title:
            args += ["--title", title]
        if y_label:
            args += ["--y-label", y_label]
        if scenarios:
            args += ["--scenarios", scenarios if isinstance(scenarios, str) else ",".join(scenarios)]
        for k, v in (set or {}).items():
            args += ["--set", f"{k}={v}"]
        if png:
            args += ["--png", "--scale", scale]
        if style:
            args += ["--style", style]
        return _fc(*args)[0]

    def layout(self, sheet=None, mode="tidy", loops=None, out=None):
        """Arrange a sheet and write the positions back. mode: "tidy" (the app's Tidy up) or "loops" (each loop a circle)."""
        args = ["layout", self.path, "--mode", mode]
        if sheet:
            args += ["--sheet", sheet]
        if loops:
            args += ["--loops", ",".join(loops)]
        if out:
            args += ["--out", out]
        return _fc(*args)[0]

    def loop_ids(self):
        """The feedback loops of the model with their R1/B2 names and the words around each."""
        return _fc("cld", self.path, "--list")[0]["loops"]

    def cld(self, out, loops=None, layout=None, sheet_name=None):
        """Write a copy of the model with a causal loop sheet that keeps only `loops` (e.g. ["R1", "B2"]; None = all)."""
        args = ["cld", self.path, "--from-model", "--out", out]
        if loops:
            args += ["--loops", ",".join(loops)]
        if layout:
            args += ["--layout", layout]
        if sheet_name:
            args += ["--sheet-name", sheet_name]
        return _fc(*args)[0]

    def set(self, pos=None, polarity=None, delay=None, loop_name=None, caption=None, source=None, sheet=None, out=None):
        """Edit the figure: pos=[(name, x, y)], polarity=[(a, b, "+")], delay=[(a, b)], loop_name={"B1": "name"}, caption, source."""
        args = ["set", self.path]
        if sheet:
            args += ["--sheet", sheet]
        if out:
            args += ["--out", out]
        for name, x, y in pos or []:
            args += ["--pos", name, x, y]
        for a, b, sign in polarity or []:
            args += ["--polarity", a, b, sign]
        for a, b in delay or []:
            args += ["--delay", a, b]
        for k, v in (loop_name or {}).items():
            args += ["--loop-name", k, v]
        if caption is not None:
            args += ["--caption", caption]
        if source is not None:
            args += ["--source", source]
        return _fc(*args)[0]

    def style(self, apply=None):
        """Store a figure style file (font, colours, line widths…) in the model; without `apply`, show the current style."""
        return _fc("style", self.path, *(["--apply", apply] if apply else ["--show"]))[0]

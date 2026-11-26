#!/usr/bin/env python3
"""Tests of sdai-compare.py on small synthetic result files. Run: python3 scripts/test_sdai_compare.py"""
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("sdai_compare", os.path.join(HERE, "sdai-compare.py"))
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


def row(name, ok, cat="extremeConditions", group="extremeConditions", fail=None, cost=0.5):
    return {"engineConfigName": "merlin", "category": cat, "group": group, "name": name, "pass": ok, "failures": [] if ok else [{"type": fail or "Problem not found by testing"}], "cost": {"total": cost, "reusedGeneration": False}}


def with_calls(r, *names):
    r = dict(r)
    r["generatedResponse"] = {"toolLog": [{"toolName": n} for n in names]}
    return r


def write(d, name, rows):
    p = os.path.join(d, name)
    json.dump({"results": rows}, open(p, "w"))
    return p


class Compare(unittest.TestCase):
    def test_wilson_matches_known_values(self):
        lo, hi = mod.wilson(8, 10)
        self.assertAlmostEqual(lo, 0.4902, places=3)
        self.assertAlmostEqual(hi, 0.9433, places=3)
        self.assertEqual(mod.wilson(0, 0)[0] != mod.wilson(0, 0)[0], True)
        lo, hi = mod.wilson(0, 10)
        self.assertEqual(lo, 0.0)
        self.assertAlmostEqual(hi, 0.2775, places=3)

    def test_newcombe_difference(self):
        lo, hi = mod.newcombe(5, 10, 8, 10)
        self.assertLess(lo, 0.3)
        self.assertGreater(hi, 0.3)
        # by hand: 0.3 - sqrt((0.8 - 0.4902)^2 + (0.7634 - 0.5)^2) = -0.1067
        self.assertAlmostEqual(lo, -0.1067, places=3)

    def test_the_table_has_losses_first_and_the_same_files_give_the_same_text(self):
        with tempfile.TemporaryDirectory() as d:
            off = [write(d, "off1.json", [row("a", True), row("b", True), row("c", False, cat="formulationCritique", group="formulationCritique", fail="Plant missed")])]
            on = [write(d, "on1.json", [row("a", False, fail="Fixed model changed behavior"), row("b", True), row("c", True, cat="formulationCritique", group="formulationCritique")])]
            one = mod.render(off, on, "abc123")
            two = mod.render(off, on, "abc123")
            self.assertEqual(one, two)
            body = one.split("## Pass rate by category and group (losses first)")[1].split("## Tests that changed")[0]
            self.assertLess(body.index("| extremeConditions |"), body.index("| formulationCritique |"))
            self.assertIn("sd-ai commit: abc123", one)
            self.assertIn("1 test(s) did worse with the tools, 1 did better", one)
            self.assertIn("Fixed model changed behavior", one)
            self.assertIn("Without the tools: $1.50. With the tools: $1.50.", one)

    def test_tool_use_is_counted_per_arm(self):
        with tempfile.TemporaryDirectory() as d:
            off = [write(d, "o.json", [row("a", True), row("b", True)])]
            on = [write(d, "n.json", [with_calls(row("a", True), "dynamist_check", "client_dynamist_check", "run_scenario"), row("b", True)])]
            text = mod.render(off, on, None)
            self.assertIn("| With | 2 | 1 | 2 | 0 |", text)
            self.assertIn("| Without | 2 | 0 | 0 | 0 |", text)

    def test_a_held_out_group_is_refused(self):
        with tempfile.TemporaryDirectory() as d:
            p = write(d, "x.json", [row("a", True, group="extremeConditionsHeldOut")])
            r = subprocess.run([sys.executable, os.path.join(HERE, "sdai-compare.py"), "--off", p, "--on", p], capture_output=True, text=True)
            self.assertNotEqual(r.returncode, 0)
            self.assertIn("held-out", r.stderr)


if __name__ == "__main__":
    unittest.main()

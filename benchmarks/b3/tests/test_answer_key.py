import os
import subprocess
import sys
import tempfile
import textwrap

HERE = os.path.dirname(os.path.abspath(__file__))
CHECK = os.path.join(HERE, "..", "answer_key.py")

GOOD = """
- case: TEST
  finding: Loop R1 dominates early growth
  type: dominant_loop
  phase: 0-10
  qualitative: true
  source: Example 2020, Fig. 3
  version: Example model v1
- case: TEST
  finding: Peak is about 42
  type: behaviour
  value: 42
  qualitative: false
  digitised: true
  source: Example 2020, Fig. 3
  version: Example model v1
"""

BAD = {
    "no source locator": "- {case: T, finding: f, type: policy, qualitative: true, source: Smith 2020, version: v}",
    "qualitative with value": "- {case: T, finding: f, type: policy, qualitative: true, value: 3, source: Smith 2020 p. 4, version: v}",
    "figure value not digitised": "- {case: T, finding: f, type: behaviour, qualitative: false, value: 3, source: Smith 2020 Fig. 2, version: v}",
    "unknown type": "- {case: T, finding: f, type: vibes, qualitative: true, source: Smith 2020 p. 4, version: v}",
    "missing version": "- {case: T, finding: f, type: policy, qualitative: true, source: Smith 2020 p. 4}",
}


def run(text):
    with tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False) as f:
        f.write(textwrap.dedent(text))
        name = f.name
    try:
        return subprocess.run([sys.executable, CHECK, name], capture_output=True, text=True)
    finally:
        os.unlink(name)


def test_good_file_passes():
    r = run(GOOD)
    assert r.returncode == 0, r.stdout


def test_each_broken_file_fails():
    for why, text in BAD.items():
        r = run(text)
        assert r.returncode == 1, f"{why}: expected failure, got {r.stdout}"


def test_shipped_keys_are_valid():
    r = subprocess.run([sys.executable, CHECK], capture_output=True, text=True)
    assert r.returncode == 0, r.stdout


if __name__ == "__main__":
    test_good_file_passes(); test_each_broken_file_fails(); test_shipped_keys_are_valid(); print("ok")

import os, shutil, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); B3 = os.path.join(HERE, "..")
RUN = os.path.join(B3, "run_b3.py")
def go(*a):
    return subprocess.run([sys.executable, RUN, "toycase", "--dry", *a], capture_output=True, text=True)
def test_freeze_and_refusal():
    cfg = os.path.join(B3, "config", "toycase.yaml"); res = os.path.join(B3, "results", "toycase")
    try:
        open(cfg, "w").write("model: models/toy.sdm\nsteps:\n  screen: {trajectories: 10}\n")
        r = go(); assert r.returncode == 0 and "frozen" in r.stdout, r.stdout + r.stderr
        assert os.path.exists(os.path.join(res, "CONFIG.sha256"))
        open(cfg, "w").write("model: models/toy.sdm\nsteps:\n  screen: {trajectories: 20}\n")
        r = go(); assert r.returncode != 0 and "REFUSED" in (r.stdout + r.stderr)
        r = go("--refreeze", "test"); assert r.returncode == 0
        assert "refrozen" in open(os.path.join(res, "FREEZE_LOG.md")).read()
    finally:
        if os.path.exists(cfg): os.unlink(cfg)
        shutil.rmtree(res, ignore_errors=True)
if __name__ == "__main__":
    test_freeze_and_refusal(); print("ok")

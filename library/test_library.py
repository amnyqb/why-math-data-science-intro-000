"""Library and tutorial tests.   FC_BIN=target/release/dyn python3 -m pytest library/test_library.py

Skipped without FC_BIN (the built `dyn`). The figure rebuild also needs seaborn (python/requirements-plots.txt).
"""
import json, os, pathlib, re, subprocess, sys

import pytest

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
FC = os.environ.get("FC_BIN") or str(REPO / "target" / "release" / "dyn")
pytestmark = pytest.mark.skipif(not os.path.exists(FC), reason="needs FC_BIN (cargo build --release -p dynamist-cli)")
INDEX = json.load(open(HERE / "index.json"))["models"]
WIKI = REPO / "wiki"


def fc(*a):
    return subprocess.run([FC, *map(str, a)], capture_output=True, text=True)


def test_index_describes_every_file_and_every_page():
    files = sorted(p.name for p in (HERE / "lessons").glob("*.sdm"))
    assert files == sorted(pathlib.Path(m["file"]).name for m in INDEX)
    slugs = [m["slug"] for m in INDEX]
    assert slugs == sorted(slugs) and len(slugs) == 7
    for m in INDEX:
        for k in ("title", "level", "concepts", "question", "source", "licence"):
            assert m[k], (m["slug"], k)
        assert m["level"] in ("beginner", "intermediate", "advanced")
        assert set(m["prerequisites"]) <= set(slugs[:slugs.index(m["slug"])]), "prerequisites come earlier"
        assert (REPO / m["tutorial"]).exists()


def test_no_unlicensed_model_is_shipped():
    banned = re.compile(r"market growth|warren farr|farr", re.I)
    for p in list((HERE / "lessons").glob("*")) + [HERE / "index.json"]:
        assert not banned.search(p.read_text()), p
    assert not list((HERE / "lessons").glob("*Market*"))


@pytest.mark.parametrize("m", INDEX, ids=lambda m: m["slug"])
def test_model_loads_validates_and_runs(m):
    path = REPO / "library" / m["file"]
    v = json.loads(fc("validate", path).stdout)
    assert v["ok"], v
    assert not [i for i in v.get("issues", []) if i["severity"] == "error"]
    assert fc("test", path).returncode == 0
    r = json.loads(fc("run", path).stdout)
    assert r["steps"] > 10
    for s in r["series"]:
        assert s["last"] == s["last"] and abs(s["last"]) < 1e9, s["name"]    # a number, not NaN or blown up
    mf = json.loads(fc("manifest", path).stdout)
    mf = mf.get("manifest", mf)
    assert mf["levers"] and mf["goals"]
    assert fc("loops", path).returncode == 0


def test_pages_link_to_existing_files():
    pages = [WIKI / "Tutorial.md"] + [WIKI / pathlib.Path(m["tutorial"]).relative_to("wiki") for m in INDEX]
    for page in pages:
        for target in re.findall(r"\]\(([^)#]+?)(?:#[^)]*)?\)", page.read_text()):
            if target.startswith("http"):
                continue
            assert (page.parent / target).exists(), f"{page.name} links to missing {target}"


def test_each_page_has_the_required_sections():
    for m in INDEX:
        t = (REPO / m["tutorial"]).read_text()
        for h in ("## The question", "## The model", "## What to click or call", "## What you should see", "## Check yourself", "## Next", "**Answers.**"):
            assert h in t or (h == "## Next" and "## Where next" in t), (m["slug"], h)
        assert "](img/" in t


def formats(v):
    r = round(v)
    return {f"{v:.2f}", f"{v:g}", str(r), f"{r:,}".replace(",", " "), f"{v:.1f}", f"{v:.0%}".replace("%", "")}


# the numbers each page quotes (facts.json is written by library/build_tutorial.py from real runs)
QUOTED = {
    "01": ["l01_peak", "l01_end", "l01_ex_drain6_end", "l01_ex_tap10_peak"],
    "02": ["l02_end", "l02_end_deaths5", "l02_end_births2"],
    "03": ["l03_fixed_jump_time", "l03_mixed_t110", "l03_mixed_t119", "l03_transit_end", "l03_transit_end_d8", "l03_mixed_t110_d8"],
    "04": ["l04_trough", "l04_trough_time", "l04_peak_after", "l04_peak_after_time", "l04_wf_peak", "l04_wf_end", "l04_trough_hire8", "l04_trough_hire1"],
    "05": ["l05_end", "l05_peak_growth", "l05_peak_growth_time", "l05_pop_at_peak_growth", "l05_switch_time", "l05_end_cap2000",
           "l05_peak_growth_cap2000", "l05_peak_growth_time_cap2000", "l05_end_rate025"],
    "06": ["l06_best_boats", "l06_best_catch", "l06_catch_b16", "l06_fish_b16", "l06_land_best_boats_slow", "l06_land_best_boats_fast",
           "l06_land_best_catch", "l06_catch_b10", "l06_catch_b40", "l06_fish_b20"],
    "07": ["l07_base_trough", "l07_mustar_Desired Inventory", "l07_mustar_Demand Step", "l07_mustar_Inventory Correction Time",
           "l07_mustar_Hiring Time", "l07_swing_Demand Step", "l07_swing_Hiring Time", "l07_stwf_Productivity per Worker"],
}


def test_pages_quote_the_numbers_the_models_produce():
    facts = json.load(open(WIKI / "tutorial" / "img" / "facts.json"))
    for m in INDEX:
        num = m["slug"][:2]
        text = (REPO / m["tutorial"]).read_text()
        for key in QUOTED[num]:
            assert any(f in text for f in formats(facts[key])), f"{m['slug']} does not quote {key} = {facts[key]}"


def test_facts_are_current_and_figures_rebuild(tmp_path):
    """One script rebuilds every figure; the numbers it measures equal the saved ones and the same figures appear."""
    pytest.importorskip("seaborn")
    env = {**os.environ, "FIG_OUT": str(tmp_path), "FC_BIN": FC}
    r = subprocess.run([sys.executable, str(HERE / "build_tutorial.py")], capture_output=True, text=True, env=env)
    assert r.returncode == 0, r.stderr[-800:]
    saved = WIKI / "tutorial" / "img"
    assert sorted(p.name for p in tmp_path.glob("*")) == sorted(p.name for p in saved.glob("*"))
    assert json.load(open(tmp_path / "facts.json")) == json.load(open(saved / "facts.json"))
    for p in tmp_path.glob("*.svg"):
        assert p.stat().st_size > 1000, p.name
        if os.environ.get("FIG_STRICT"):
            assert p.read_bytes() == (saved / p.name).read_bytes(), p.name
    # every figure the pages show is one of the rebuilt ones
    shown = {n for m in INDEX for n in re.findall(r"\]\(img/([^)]+)\)", (REPO / m["tutorial"]).read_text())}
    assert shown <= {p.name for p in tmp_path.glob("*")}, shown - {p.name for p in tmp_path.glob("*")}


def test_catalog_is_consistent_and_searchable():
    """library/catalog.json passes `dyn library --check` and a filter finds what it should."""
    cat = str(HERE / "catalog.json")
    r = fc("library", "--catalog", cat, "--check")
    assert r.returncode == 0, r.stdout
    r = fc("library", "--catalog", cat, "--domain", "supply-chain", "--skill", "delays")
    slugs = [m["slug"] for m in json.loads(r.stdout)["models"]]
    assert "03-delays" in slugs and "beer-game" in slugs

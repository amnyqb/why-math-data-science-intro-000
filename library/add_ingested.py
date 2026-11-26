"""Add models produced by `dyn library --ingest` to library/catalog.json with mechanical tags.
Usage: python3 library/add_ingested.py <ingest-out-dir> ...   (each holds <set>.json and sdm-<set>/)
Only models that import and run are added. Tags are derived from the model's own structure; authors, year
and a real question stay marked as needing a human (special tag 'needs-review')."""
import json, shutil, sys, pathlib

root = pathlib.Path(__file__).parent
out = root / "ingested" / "entries.json"
entries = []
have = set()
added = 0
for d in map(pathlib.Path, sys.argv[1:]):
    for rep in d.glob("*.json"):
        name = rep.stem
        for e in json.load(open(rep))["entries"]:
            if not e.get("runs") or not e.get("file"):
                continue
            slug = name + "-" + "".join(c if c.isalnum() else "-" for c in pathlib.Path(e["source_file"]).with_suffix("").as_posix().lower()).strip("-")
            if slug in have:
                continue
            st = e["stats"]
            feats = (["lookup"] if st["lookups"] else []) + (["units"] if e.get("validate_errors", 1) == 0 else [])
            n = st["stocks"]
            simp = 1 if n <= 1 else 2 if n <= 3 else 3 if n <= 8 else 4 if n <= 25 else 5
            dst = root / "ingested" / name
            dst.mkdir(parents=True, exist_ok=True)
            shutil.copy(d / f"sdm-{name}" / pathlib.Path(e["file"]).name, dst / f"{slug}.sdm")
            entries.append({
                "slug": slug, "title": e["title"], "category": ["benchmark"], "domain": ["generic"],
                "skills": ["stocks-and-flows"], "features": feats,
                "special": ["imported", name, "needs-review"], "level": "intermediate",
                "status": "built", "simplicity": simp, "difficulty": simp,
                "question": f'Imported from {e["source_file"]}; question to be written.',
                "authors": [f"{name} contributors"], "year": None,
                "source": f'{e["source"]} ({e["source_file"]})', "licence": e["licence"],
                "file": f"library/ingested/{name}/{slug}.sdm",
                "notes": f'{st["stocks"]} stocks, {st["flows"]} flows, {st["auxiliaries"]} aux; imports and runs; mechanical tags only.',
            })
            have.add(slug)
            added += 1
out.parent.mkdir(exist_ok=True)
json.dump(entries, open(out, "w"), indent=1, ensure_ascii=False)
print("added", added)

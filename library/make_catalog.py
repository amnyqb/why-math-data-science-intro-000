"""Writes library/catalog.json: the searchable description of every model in (or planned for) the library.
Edit this file, run it, then `dyn library --check`. Lessons are taken from index.json; the rest is written here.
Scales: simplicity 1 (a few variables) to 5 (hundreds); difficulty 1 (a first session) to 5 (needs a course)."""
import json, os
HERE = os.path.dirname(os.path.abspath(__file__))

VOCAB = {
 "category": ["lesson", "classic", "textbook", "benchmark", "case-study"],
 "domain": ["population", "epidemiology", "ecology", "resources", "economy", "business", "supply-chain", "workforce",
            "technology-diffusion", "world", "urban", "environment", "energy", "project-management", "generic"],
 "skills": ["stocks-and-flows", "feedback-loops", "reinforcing-loops", "balancing-loops", "delays", "nonlinearity",
            "table-functions", "oscillation", "growth-limits", "policy-design", "decision-rules", "sensitivity",
            "loop-dominance", "calibration", "model-reduction", "units", "boundary-choice"],
 "features": ["lookup", "delay-fixed", "delay-first-order", "smooth", "switch", "levers", "units", "layers",
              "subsystems", "arrays", "modules", "multiple-sectors", "random-input", "if-then-else", "exact-match-reference"],
}
OWN = "Dynamist project"
IDX = {m["slug"]: m for m in json.load(open(os.path.join(HERE, "index.json")))["models"]}

def lesson(slug, domain, skills, features, special, lessons, simp, diff):
    m = IDX[slug]
    return dict(slug=slug, title=m["title"], category=["lesson"], domain=domain, skills=skills, features=features,
                special=special, level=m["level"], status="built", simplicity=simp, difficulty=diff,
                question=m["question"], lessons=lessons, authors=[OWN], year=2026,
                file="library/" + m["file"], tutorial=m["tutorial"], source=m["source"], licence=m["licence"])

E = [
 lesson("01-bathtub", ["generic"], ["stocks-and-flows"], ["units"], ["bathtub", "accumulation"],
        ["A stock changes only through its flows", "Net flow, not the flows alone, sets the direction"], 1, 1),
 lesson("02-population-loops", ["population"], ["feedback-loops", "reinforcing-loops", "balancing-loops"], ["units"], ["births and deaths"],
        ["Each loop is read on its own, then together", "Loop polarity decides growth or decay"], 1, 2),
 lesson("03-delays", ["supply-chain"], ["delays", "stocks-and-flows"], ["delay-fixed", "delay-first-order", "units"], ["pipeline", "material in transit"],
        ["The kind of delay changes the shape of the response, not only its timing"], 1, 2),
 lesson("04-workforce-oscillation", ["workforce", "business"], ["oscillation", "delays", "balancing-loops", "decision-rules"], ["delay-first-order", "units"], ["overshoot", "hiring delay"],
        ["A balancing loop with a delay overshoots", "Damping depends on adjustment times"], 2, 3),
 lesson("05-limits-to-growth", ["population", "resources"], ["growth-limits", "nonlinearity", "loop-dominance", "reinforcing-loops", "balancing-loops"], ["lookup", "units"], ["S-shape", "carrying capacity"],
        ["Dominance shifts from the reinforcing to the balancing loop", "The limit shapes the curve long before it is reached"], 2, 3),
 lesson("06-fishery-policy", ["ecology", "resources"], ["policy-design", "nonlinearity", "decision-rules"], ["levers", "lookup", "units"], ["fishing effort", "sustainable yield"],
        ["Pushing a lever harder is not always better", "A policy is judged on a horizon and a measure"], 3, 3),
 lesson("07-workforce-what-matters", ["workforce", "business"], ["sensitivity", "loop-dominance", "model-reduction", "policy-design"], ["levers", "units"], ["screening", "Sobol indices"],
        ["Most inputs do not matter; find the few that do before studying them", "Loop dominance explains why"], 3, 4),
]

def cl(slug, title, cat, domain, skills, features, special, level, simp, diff, question, lessons, authors, year, citation, url=None, file=None, status="planned", note=""):
    d = dict(slug=slug, title=title, category=cat, domain=domain, skills=skills, features=features, special=special,
             level=level, status=status, simplicity=simp, difficulty=diff, question=question, lessons=lessons,
             authors=authors, year=year, paper=dict(citation=citation), source=note or "To be rebuilt from the published description and checked against the published results.")
    if url: d["paper"]["url"] = url
    if file: d["file"] = file
    return d

E += [
 cl("bass-diffusion", "Bass diffusion", ["classic"], ["technology-diffusion", "business"], ["reinforcing-loops", "balancing-loops", "growth-limits"], ["units"], ["word of mouth", "adopters"],
    "beginner", 1, 2, "How do innovators and imitators together produce an S-shaped adoption curve?", ["Two loops, one stock, an S-curve"],
    ["Frank M. Bass"], 1969, "Bass, F. M. (1969). A new product growth model for consumer durables. Management Science 15(5), 215-227."),
 cl("sir-epidemic", "SIR epidemic", ["classic"], ["epidemiology", "population"], ["stocks-and-flows", "feedback-loops", "nonlinearity"], ["units", "levers"], ["epidemic peak", "herd immunity", "R0"],
    "beginner", 1, 2, "Why does an epidemic peak and fade before everyone is infected?", ["The infection rate falls as susceptibles run out"],
    ["W. O. Kermack", "A. G. McKendrick"], 1927, "Kermack, W. O. and McKendrick, A. G. (1927). A contribution to the mathematical theory of epidemics. Proc. Royal Society A 115, 700-721."),
 cl("predator-prey", "Predator and prey", ["classic"], ["ecology", "population"], ["oscillation", "feedback-loops", "nonlinearity"], ["units"], ["Lotka-Volterra", "cycles"],
    "beginner", 1, 3, "Why do two interacting populations cycle?", ["Coupled loops give sustained oscillation"],
    ["Alfred J. Lotka", "Vito Volterra"], 1926, "Volterra, V. (1926). Fluctuations in the abundance of a species considered mathematically. Nature 118, 558-560."),
 cl("beer-game", "The beer distribution game", ["classic", "benchmark"], ["supply-chain", "business"], ["decision-rules", "delays", "oscillation", "policy-design"], ["delay-fixed", "multiple-sectors", "levers", "units"], ["bullwhip", "anchoring and adjustment", "misperceptions of feedback"],
    "advanced", 3, 4, "Why do orders swing more at each step up a supply chain, even when customer demand barely moves?", ["Ordering rules that ignore the supply line cause amplification"],
    ["John D. Sterman"], 1989, "Sterman, J. D. (1989). Modeling managerial behavior: misperceptions of feedback in a dynamic decision making experiment. Management Science 35(3), 321-339.",
    file="benchmarks/b1/beer_game_m5.sdm", status="built", note="Built in Dynamist (benchmark B1); the order rule is the published anchoring-and-adjustment rule."),
 cl("workforce-inventory", "Workforce and inventory", ["textbook", "benchmark"], ["workforce", "business", "supply-chain"], ["oscillation", "delays", "balancing-loops", "decision-rules", "sensitivity"], ["delay-first-order", "if-then-else", "levers", "units"], ["production cycle", "capacity"],
    "intermediate", 3, 3, "Why do production, workforce and inventory cycle after a demand change?", ["Coupled stock-management loops with delays produce sustained cycles"],
    ["John D. Sterman"], 2000, "Sterman, J. D. (2000). Business Dynamics: Systems Thinking and Modeling for a Complex World. Irwin/McGraw-Hill, ch. 18.",
    file="benchmarks/b3/models/b2-1-workforce-inventory.sdm", status="built", note="Built in Dynamist (benchmark B3, case b2-1)."),
 cl("industrial-dynamics", "Industrial dynamics: the production-distribution chain", ["classic"], ["business", "supply-chain"], ["delays", "oscillation", "decision-rules", "boundary-choice"], ["multiple-sectors", "smooth", "units"], ["factory, distributor, retailer"],
    "intermediate", 3, 4, "How do decision rules and delays inside a firm create instability from steady demand?", ["Instability comes from structure, not from outside shocks"],
    ["Jay W. Forrester"], 1961, "Forrester, J. W. (1961). Industrial Dynamics. MIT Press."),
 cl("market-growth", "Market growth", ["classic", "benchmark"], ["business", "economy"], ["growth-limits", "reinforcing-loops", "balancing-loops", "loop-dominance", "policy-design"], ["lookup", "delay-first-order", "units"], ["capital investment", "sales force", "delivery delay"],
    "intermediate", 3, 4, "Why does a firm's growth stall, and what does capital investment do about it?", ["Growth loops hit a delivery limit; policy shifts when"],
    ["Jay W. Forrester"], 1968, "Forrester, J. W. (1968). Market growth as influenced by capital investment. Industrial Management Review 9(2), 83-105."),
 cl("urban-dynamics", "Urban dynamics", ["classic"], ["urban", "economy", "population"], ["policy-design", "nonlinearity", "boundary-choice", "loop-dominance"], ["multiple-sectors", "lookup", "units"], ["counterintuitive policy", "low-cost housing"],
    "advanced", 5, 5, "Why do well-meant urban policies often worsen what they target?", ["Policy resistance: the system pushes back through its other loops"],
    ["Jay W. Forrester"], 1969, "Forrester, J. W. (1969). Urban Dynamics. MIT Press."),
 cl("world2", "World dynamics (World2)", ["classic"], ["world", "resources", "environment"], ["growth-limits", "nonlinearity", "boundary-choice", "calibration"], ["lookup", "multiple-sectors", "units"], ["overshoot and collapse"],
    "advanced", 4, 5, "What do population, capital, pollution, resources and food do together over two centuries?", ["Five stocks, many limits: growth overshoots and declines"],
    ["Jay W. Forrester"], 1971, "Forrester, J. W. (1971). World Dynamics. Wright-Allen Press."),
 cl("world3-1974", "World3 (1974 edition)", ["classic", "benchmark"], ["world", "resources", "environment", "economy"], ["growth-limits", "nonlinearity", "calibration", "model-reduction", "sensitivity", "boundary-choice"], ["lookup", "delay-fixed", "smooth", "multiple-sectors", "units", "exact-match-reference"], ["limits to growth", "overshoot and collapse"],
    "advanced", 5, 5, "Under which conditions does the world economy overshoot its limits, and which policies change that?", ["Many interlocking limits; scenario choices matter more than any one number"],
    ["Dennis L. Meadows", "Donella H. Meadows", "Jorgen Randers", "William W. Behrens III"], 1974, "Meadows, D. L., Behrens, W. W., Meadows, D. H., Naill, R. F., Randers, J. and Zahn, E. K. O. (1974). Dynamics of Growth in a Finite World. Wright-Allen Press.",
    note="Built in Dynamist from the published equations (benchmark B3) and compared with the PyWorld3 reference."),
 cl("world3-2003", "World3-03", ["classic", "benchmark"], ["world", "resources", "environment", "economy"], ["growth-limits", "nonlinearity", "boundary-choice", "sensitivity"], ["lookup", "delay-fixed", "smooth", "multiple-sectors", "units", "exact-match-reference"], ["limits to growth", "30-year update"],
    "advanced", 5, 5, "How do the 2004 update's scenarios differ from the 1972 results?", ["The same structure with updated data; the scenarios carry the message"],
    ["Donella H. Meadows", "Jorgen Randers", "Dennis L. Meadows"], 2004, "Meadows, D., Randers, J. and Meadows, D. (2004). Limits to Growth: The 30-Year Update. Chelsea Green.",
    note="Imported from the Vensim file and run in benchmark B2."),
 cl("fishery-commons", "Fishery and the commons", ["classic"], ["ecology", "resources", "economy"], ["policy-design", "balancing-loops", "decision-rules"], ["levers", "lookup", "units"], ["tragedy of the commons", "overfishing"],
    "intermediate", 2, 3, "Why do many boats overfish a stock that all of them depend on?", ["Individually rational effort ruins the shared stock"],
    ["H. Scott Gordon"], 1954, "Gordon, H. S. (1954). The economic theory of a common-property resource: the fishery. Journal of Political Economy 62(2), 124-142."),
]

# Models built by us under library/classics/<slug>/ (make.py, check.py, NOTES.md): mark them built.
for e in E:
    d = os.path.join(HERE, "classics", e["slug"])
    f = os.path.join(d, e["slug"] + ".sdm")
    if os.path.exists(f):
        e["status"] = "built"
        e["file"] = "library/classics/%s/%s.sdm" % (e["slug"], e["slug"])
        e["notes"] = "library/classics/%s/NOTES.md" % e["slug"]
        e["source"] = "Own implementation of the published structure (own parameter values), checked by library/classics/%s/check.py." % e["slug"]
        e["licence"] = "Same licence as the Dynamist project (own work); free to use, copy and teach with."

_ing = os.path.join(HERE, "ingested", "entries.json")
if os.path.exists(_ing):  # written by add_ingested.py
    E += json.load(open(_ing))
cat = {"about": "Dynamist model library catalogue. Generic tags come from `vocab`; `special` tags are free. Search with `dyn library`.",
       "vocab": VOCAB, "entries": E}
json.dump(cat, open(os.path.join(HERE, "catalog.json"), "w"), indent=1)
print(len(E), "entries")

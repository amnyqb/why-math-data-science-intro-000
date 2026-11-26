#!/usr/bin/env python3
"""Builds World3 as in 'Dynamics of Growth in a Finite World' (1974), transcribed from PyWorld3
(https://github.com/cvanwynsberghe/pyworld3, clone in benchmarks/b3/build/pyworld3), as a Dynamist model through `dyn ops`.
Writes the model and the source table benchmarks/b3/models/W3_1974_SOURCES.md (file:line in PyWorld3 for every equation).

Usage: build_world3_1974.py [--out PATH] [--dt 0.5] [--smooth-init pw3|book] [--delay3-init book|pw3] [--sources PATH]

Three initial-value choices that PyWorld3 makes in a way that is not DYNAMO's (see W3_1974_SOURCES.md, section Differences):
  --smooth-init  : start of the three smooths that sit inside an initial-value loop (LUFD, AI, PFR).
                   pw3  = the values PyWorld3 stores at k=0 (lufd[0], ai[0], pfr[0]; they equal the input at k=0 in its own pass)
                   book = the book's own start values (LUFD=1 so CUF=1, AI=5e9, PFR=1)
  --delay3-init  : start of DELAY3(PPGR, PPTD).
                   book = outflow starts at the input (DYNAMO, Dynamist DELAY3)
                   pw3  = PyWorld3's Delay3 starts every stage at input*3/delay (outflow 0.15 x input at k=0)
Defaults: pw3 / book. Nothing else is tuned: every constant is PyWorld3's default (world3.py init_world3_constants)."""
import argparse, json, os, re, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
B3 = os.path.join(HERE, "..")
PW = os.path.join(B3, "build", "pyworld3", "pyworld3")
REPO = os.path.abspath(os.path.join(B3, "..", ".."))

ap = argparse.ArgumentParser()
ap.add_argument("--out", default=os.path.join(B3, "build", "w3_1974.sdm"))
ap.add_argument("--dt", default="0.5")
ap.add_argument("--smooth-init", default="pw3", choices=["pw3", "book"])
ap.add_argument("--delay3-init", default="book", choices=["book", "pw3"])
ap.add_argument("--sources", default=os.path.join(HERE, "W3_1974_SOURCES.md"))
ap.add_argument("--fc", default=os.environ.get("FC"))
args = ap.parse_args()


def find_fc():
    if args.fc:
        return args.fc
    for root in (REPO, os.path.abspath(os.path.join(REPO, "..", "..", ".."))):
        p = os.path.join(root, "target", "release", "dyn")
        if os.path.exists(p):
            return p
    sys.exit("dyn binary not found: build with `cargo build --release -p dynamist-cli` or set FC")


# ---- PyWorld3 source lookup (text only, nothing is executed) ----------------------------------------------------------------
FILES = ["population.py", "capital.py", "agriculture.py", "pollution.py", "resource.py"]
ASSIGN = {}   # python variable -> "file:line" of its update equation
for fn in FILES:
    cur = None
    for i, line in enumerate(open(os.path.join(PW, fn)).read().split("\n"), 1):
        m = re.match(r"\s+def (\w+)\(", line)
        if m:
            cur = m.group(1)
        m = re.match(r"\s+self\.(\w+)\[(k|kl)\]\s*=", line)
        if m and cur and cur.startswith("_update_"):
            ASSIGN.setdefault(m.group(1), f"{fn}:{i}")
W3 = open(os.path.join(PW, "world3.py")).read().split("\n")
def const_src(py):
    for i, line in enumerate(W3[105:122], 106):
        if re.search(r"\b%s=" % re.escape(py), line):
            return f"world3.py:{i}"
    return "?"
TABLES = json.load(open(os.path.join(PW, "functions_table_world3.json")))
JSONLINES = {}
for i, line in enumerate(open(os.path.join(PW, "functions_table_world3.json")).read().split("\n"), 1):
    m = re.search(r'"y.name": "(\w+)"', line)
    if m:
        JSONLINES[m.group(1)] = i

# ---- model description -----------------------------------------------------------------------------------------------------
ops, rows = [], []   # ops for dyn; rows for the sources table: (name, kind, equation, source, note)


def add(name, kind, eq, src, note="", **kw):
    o = {"op": "add_var", "name": name, "kind": kind, "equation": eq}
    o.update(kw)
    if note:
        o["doc"] = note
    ops.append(o)
    rows.append((name, kind, eq, src, note))


def const(name, value, py=None, note=""):
    add(name, "constant", repr(value) if isinstance(value, float) else str(value), const_src(py or name.lower()), note)


def aux(name, eq, py=None, note=""):
    add(name, "aux", eq, ASSIGN.get(py or name.lower(), "?"), note)


def flow(name, eq, frm=None, to=None, py=None, note=""):
    kw = {}
    if frm:
        kw["from"] = frm
    if to:
        kw["to"] = to
    add(name, "flow", eq, ASSIGN.get(py or name.lower(), "?"), note, **kw)


stocks = []
def stock(name, init, py=None, note=""):
    # stock equation = its start value; the update line is the state update in PyWorld3
    stocks.append((name, init, ASSIGN.get(py or name.lower(), "?"), note))


def sw(f2, f1, tsw="PYEAR"):
    """PyWorld3 clip(func2, func1, t, t_switch): func1 up to and including t_switch, func2 after."""
    return f"IF THEN ELSE(TIME > {tsw}, {f2}, {f1})"


tabs = []
def table(name):
    t = next(t for t in TABLES if t["y.name"] == name)
    assert all(a < b for a, b in zip(t["x.values"], t["x.values"][1:])), name
    tabs.append((name, list(zip(t["x.values"], t["y.values"])), f"functions_table_world3.json:{JSONLINES[name]}", t["x.name"], t["sector"]))
    return f"T_{name}"


# constants (world3.py:106-121, the defaults of init_world3_constants)
C = {
    "P1I": 65e7, "P2I": 70e7, "P3I": 19e7, "P4I": 6e7, "DCFSN": 4, "FCEST": 4000, "HSID": 20, "IEAT": 3, "LEN": 28, "LPD": 20,
    "MTFN": 12, "PET": 4000, "RLT": 30, "SAD": 20, "ZPGT": 4000,
    "ICI": 2.1e11, "SCI": 1.44e11, "IET": 4000, "IOPCD": 400, "LFPF": 0.75, "LUFDT": 2, "ICOR1": 3, "ICOR2": 3, "SCOR1": 1, "SCOR2": 1,
    "ALIC1": 14, "ALIC2": 14, "ALSC1": 20, "ALSC2": 20, "FIOAC1": 0.43, "FIOAC2": 0.43,
    "ALI": 0.9e9, "PALI": 2.3e9, "LFH": 0.7, "PALT": 3.2e9, "PL": 0.1, "ALAI1": 2, "ALAI2": 2, "IO70": 7.9e11, "LYF1": 1, "LYF2": 1,
    "SD": 0.07, "UILI": 8.2e6, "ALLN": 6000, "UILDT": 10, "LFERTI": 600, "ILF": 600, "FSPD": 2, "SFPC": 230,
    "PPOLI": 2.5e7, "PPOL70": 1.36e8, "AHL70": 1.5, "AMTI": 1, "IMTI": 10, "IMEF": 0.1, "FIPM": 0.001, "FRPM": 0.02,
    "PPGF1": 1, "PPGF2": 1, "PPTD1": 20, "PPTD2": 20, "NRI": 1e12, "NRUF1": 1, "NRUF2": 1,
}
for k_, v in C.items():
    const(k_, float(v) if isinstance(v, float) else v)
# PPGF21 is an argument of init_world3_constants but PPGF2 is what PyWorld3 uses in the full model (pollution.py ppgf); policy dates:
const("PYEAR", 1975, py="pyear", note="implementation date of new policies; world3.py:94 (constructor default)")
rows[-1] = rows[-1][:3] + ("world3.py:94",) + rows[-1][4:]
const("IPHST", 1940, py="iphst", note="implementation date of new health service policy; world3.py:94")
rows[-1] = rows[-1][:3] + ("world3.py:94",) + rows[-1][4:]

# start values of the three smooths that sit in an initial-value loop (see the module docstring)
if args.smooth_init == "pw3":
    I0 = {"LUFD0": 1.336179775280899, "AI0": 6210132349.328245, "PFR0": 1.1709782608695651}
    note0 = "PyWorld3's stored k=0 value (output of run_pyworld3.py, column %s)"
else:
    I0 = {"LUFD0": 1.0, "AI0": 5e9, "PFR0": 1.0}
    note0 = "start value of the 1974 book (LUFD=1 gives CUF=1; AI=5e9; PFR=1); PyWorld3 uses it only for its first pass (agriculture.py:474-475, capital.py:392)"
for n_, v in I0.items():
    add(n_, "constant", repr(float(v)), {"LUFD0": "capital.py:392 / smooth", "AI0": "agriculture.py:474", "PFR0": "agriculture.py:475"}[n_], note0 % n_.lower().replace("0", "") if "%s" in note0 else note0)

# ---- tables
T = {n: table(n) for n in ["M1", "M2", "M3", "M4", "LMF", "HSAPC", "LMHS1", "LMHS2", "FPU", "CMI", "LMP", "FM", "CMPLE", "SFSN", "FRSN",
                           "FCE_TOCLIP", "FSAFC", "FIOACV", "ISOPC1", "ISOPC2", "FIOAS1", "FIOAS2", "JPICU", "JPSCU", "JPH", "CUF",
                           "IFPC1", "IFPC2", "FIOAA1", "FIOAA2", "DCPH", "LYMC", "LYMAP1", "LYMAP2", "FIALD", "MLYMC", "LLMY1", "LLMY2",
                           "UILPC", "LFDR", "LFRT", "FALM", "PCRUM", "FCAOR1", "FCAOR2", "AHLM"]}
for name, pts, src, xname, sector in tabs:
    ops.append({"op": "add_var", "name": f"T_{name}", "kind": "lookup", "doc": f"{sector}: {name} as a function of {xname}"})
    ops.append({"op": "set_table", "name": f"T_{name}", "points": [[x, y] for x, y in pts]})
    rows.append((f"T_{name}", "lookup", f"{len(pts)} points, x = {xname}", src, "interp1d, clamped at both ends = Dynamist lookup"))

# ---- stocks (start values; flows are added below)
stock("P1", "P1I", note="population 0-14"); stock("P2", "P2I", note="population 15-44"); stock("P3", "P3I", note="population 45-64")
stock("P4", "P4I", note="population 65+")
stock("IC", "ICI", note="industrial capital"); stock("SC", "SCI", note="service capital")
stock("AL", "ALI", note="arable land"); stock("PAL", "PALI", note="potentially arable land"); stock("UIL", "UILI", note="urban-industrial land")
stock("LFERT", "LFERTI", note="land fertility")
stock("PPOL", "PPOLI", note="persistent pollution"); stock("NR", "NRI", note="nonrenewable resources")
for name, init, src, note in stocks:
    ops.append({"op": "add_var", "name": name, "kind": "stock", "equation": init, "doc": note})
    rows.append((name, "stock", f"start {init}", src, note))

# ---- population sector
aux("POP", "P1 + P2 + P3 + P4")
aux("FPU", f"{T['FPU']}(POP)")
aux("LMP", f"{T['LMP']}(PPOLX)")
aux("LMF", f"{T['LMF']}(FPC / SFPC)")
aux("CMI", f"{T['CMI']}(IOPC)")
aux("HSAPC", f"{T['HSAPC']}(SOPC)")
aux("EHSPC", "SMOOTH(HSAPC, HSID)", note="specials.py:128 Smooth (first order, euler)")
aux("LMHS1", f"{T['LMHS1']}(EHSPC)"); aux("LMHS2", f"{T['LMHS2']}(EHSPC)")
aux("LMHS", sw("LMHS2", "LMHS1", "IPHST"))
aux("LMC", "1 - CMI * FPU")
aux("LE", "LEN * LMF * LMHS * LMP * LMC")
for i in "1234":
    aux(f"M{i}", f"{T['M'+i]}(LE)")
flow("MAT1", "P1 * (1 - M1) / 15", "P1", "P2"); flow("MAT2", "P2 * (1 - M2) / 30", "P2", "P3"); flow("MAT3", "P3 * (1 - M3) / 20", "P3", "P4")
for i in "1234":
    flow(f"D{i}", f"P{i} * M{i}", f"P{i}")
aux("AIOPC", "SMOOTH(IOPC, IEAT)", note="specials.py:128 Smooth")
aux("DIOPC", "SMOOTH3(IOPC, SAD)", note="specials.py:261 Dlinf3 (3rd-order information delay, all stages start at the input) = SMOOTH3")
aux("FIE", "(IOPC - AIOPC) / AIOPC")
aux("SFSN", f"{T['SFSN']}(DIOPC)")
aux("FRSN", f"{T['FRSN']}(FIE)", note="PyWorld3 sets frsn[0]=0.82 first, then recomputes it at the end of loop0; table value at FIE=0")
aux("DCFS", sw("2", "DCFSN * FRSN * SFSN", "ZPGT"), note="clip(2.0, dcfsn*frsn*sfsn, t, zpgt)")
aux("PLE", "SMOOTH3(LE, LPD)", note="specials.py:261 Dlinf3")
aux("CMPLE", f"{T['CMPLE']}(PLE)")
aux("DTF", "DCFS * CMPLE")
aux("FM", f"{T['FM']}(LE)")
aux("MTF", "MTFN * FM")
aux("NFC", "MTF / DTF - 1")
aux("FSAFC", f"{T['FSAFC']}(NFC)")
aux("FCAPC", "FSAFC * SOPC")
aux("FCFPC", "SMOOTH3(FCAPC, HSID)", note="specials.py:261 Dlinf3 with delay hsid (population.py:913)")
aux("FCE", sw("1", f"{T['FCE_TOCLIP']}(FCFPC)", "FCEST"), note="clip(1.0, fce_toclip_f(fcfpc), t, fcest)")
aux("TF", "MIN(MTF, MTF * (1 - FCE) + DTF * FCE)")
flow("B", "TF * P2 * 0.5 / RLT", None, "P1",
     note="PyWorld3: clip(d[k], tf*p2*0.5/rlt, t, pet); PET=4000, so the death branch never applies and is left out")
aux("D", "D1 + D2 + D3 + D4", py="d", note="information only; PyWorld3's d[k] is the deaths of the previous step (population.py:800), not used by anything")
aux("CDR", "1000 * D / POP", note="information only")
aux("CBR", "1000 * B / POP", note="information only")

# ---- capital sector
aux("ALIC", sw("ALIC2", "ALIC1")); flow("ICDR", "IC / ALIC", "IC")
aux("ICOR", sw("ICOR2", "ICOR1"))
aux("LUF", "J / LF")
aux("LUFD", "SMOOTHI(LUF, LUFDT, LUFD0)", note="specials.py:128 Smooth; start value breaks the initial-value loop CUF-IO-IOPC-JPICU-PJIS-J-LUF-LUFD-CUF")
aux("CUF", f"{T['CUF']}(LUFD)")
aux("IO", "IC * (1 - FCAOR) * CUF / ICOR")
aux("IOPC", "IO / POP")
aux("FIOACV", f"{T['FIOACV']}(IOPC / IOPCD)")
aux("FIOACC", sw("FIOAC2", "FIOAC1"))
aux("FIOAC", sw("FIOACV", "FIOACC", "IET"), note="clip(fioacv, fioacc, t, iet); IET=4000 so FIOAC stays at FIOACC")
aux("ISOPC1", f"{T['ISOPC1']}(IOPC)"); aux("ISOPC2", f"{T['ISOPC2']}(IOPC)"); aux("ISOPC", sw("ISOPC2", "ISOPC1"))
aux("ALSC", sw("ALSC2", "ALSC1")); flow("SCDR", "SC / ALSC", "SC")
aux("SCOR", sw("SCOR2", "SCOR1"))
aux("SO", "SC * CUF / SCOR")
aux("SOPC", "SO / POP")
aux("FIOAS1", f"{T['FIOAS1']}(SOPC / ISOPC)"); aux("FIOAS2", f"{T['FIOAS2']}(SOPC / ISOPC)"); aux("FIOAS", sw("FIOAS2", "FIOAS1"))
flow("SCIR", "IO * FIOAS", None, "SC")
aux("FIOAI", "1 - FIOAA - FIOAS - FIOAC")
flow("ICIR", "IO * FIOAI", None, "IC")
aux("JPICU", f"{T['JPICU']}(IOPC)"); aux("PJIS", "IC * JPICU")
aux("JPSCU", f"{T['JPSCU']}(SOPC)"); aux("PJSS", "SC * JPSCU")
aux("JPH", f"{T['JPH']}(AIPH)"); aux("PJAS", "JPH * AL")
aux("J", "PJIS + PJAS + PJSS")
aux("LF", "(P2 + P3) * LFPF")

# ---- agriculture sector
aux("LFC", "AL / PALT")
aux("F", "LY * AL * LFH * (1 - PL)")
aux("FPC", "F / POP")
aux("IFPC1", f"{T['IFPC1']}(IOPC)"); aux("IFPC2", f"{T['IFPC2']}(IOPC)"); aux("IFPC", sw("IFPC2", "IFPC1"))
aux("TAI", "IO * FIOAA")
aux("FIOAA1", f"{T['FIOAA1']}(FPC / IFPC)"); aux("FIOAA2", f"{T['FIOAA2']}(FPC / IFPC)"); aux("FIOAA", sw("FIOAA2", "FIOAA1"))
flow("LDR", "TAI * FIALD / DCPH", "PAL", "AL")
aux("DCPH", f"{T['DCPH']}(PAL / PALT)")
aux("CAI", "TAI * (1 - FIALD)")
aux("AI", "SMOOTHI(CAI, ALAI, AI0)", note="agriculture.py:693 smooth_cai(k, alai[k]); start value breaks the initial-value loop AI-AIPH-LYMC-LY-MPLD-FIALD-CAI")
aux("ALAI", sw("ALAI2", "ALAI1"))
aux("AIPH", "AI * (1 - FALM) / AL")
aux("LYMC", f"{T['LYMC']}(AIPH)")
aux("LY", "LYF * LFERT * LYMC * LYMAP")
aux("LYF", sw("LYF2", "LYF1"))
aux("LYMAP1", f"{T['LYMAP1']}(IO / IO70)"); aux("LYMAP2", f"{T['LYMAP2']}(IO / IO70)"); aux("LYMAP", sw("LYMAP2", "LYMAP1"))
aux("FIALD", f"{T['FIALD']}(MPLD / MPAI)")
aux("MPLD", "LY / (DCPH * SD)")
aux("MPAI", "ALAI * LY * MLYMC / LYMC")
aux("MLYMC", f"{T['MLYMC']}(AIPH)")
aux("ALL", "ALLN * LLMY")
aux("LLMY1", f"{T['LLMY1']}(LY / ILF)"); aux("LLMY2", f"{T['LLMY2']}(LY / ILF)"); aux("LLMY", sw("LLMY2", "LLMY1"))
flow("LER", "AL / ALL", "AL")
aux("UILPC", f"{T['UILPC']}(IOPC)")
aux("UILR", "UILPC * POP")
flow("LRUI", "MAX(0, (UILR - UIL) / UILDT)", "AL", "UIL")
aux("LFDR", f"{T['LFDR']}(PPOLX)")
flow("LFD", "LFERT * LFDR", "LFERT")
flow("LFR", "(ILF - LFERT) / LFRT", None, "LFERT")
aux("LFRT", f"{T['LFRT']}(FALM)")
aux("FALM", f"{T['FALM']}(PFR)")
aux("FR", "FPC / SFPC")
aux("PFR", "SMOOTHI(FR, FSPD, PFR0)", note="agriculture.py:877 smooth_fr(k, fspd); start value breaks the initial-value loop PFR-FALM-AIPH-LYMC-LY-F-FPC-FR")

# ---- pollution sector
aux("PPOLX", "PPOL / PPOL70")
aux("PPGIO", "PCRUM * POP * FRPM * IMEF * IMTI")
aux("PPGAO", "AIPH * AL * FIPM * AMTI")
aux("PPGF", sw("PPGF2", "PPGF1"))
aux("PPGR", "(PPGIO + PPGAO) * PPGF")
aux("PPTD", sw("PPTD2", "PPTD1"))
if args.delay3_init == "book":
    flow("PPAPR", "DELAY3(PPGR, PPTD)", None, "PPOL", note="specials.py:199 Delay3; start: outflow = input (DYNAMO, Dynamist DELAY3)")
else:
    flow("PPAPR", "DELAY3I(PPGR, PPTD, PPGR * 3 / PPTD)", None, "PPOL",
         note="specials.py:199 Delay3 with its start (specials.py:241-242): every stage = input*3/delay, so the outflow starts at 0.15 x PPGR")
aux("AHLM", f"{T['AHLM']}(PPOLX)")
aux("AHL", "AHLM * AHL70")
flow("PPASR", "PPOL / (AHL * 1.4)", "PPOL")

# ---- resource sector
aux("NRFR", "NR / NRI")
aux("FCAOR1", f"{T['FCAOR1']}(NRFR)"); aux("FCAOR2", f"{T['FCAOR2']}(NRFR)"); aux("FCAOR", sw("FCAOR2", "FCAOR1"))
aux("NRUF", sw("NRUF2", "NRUF1"))
aux("PCRUM", f"{T['PCRUM']}(IOPC)")
flow("NRUR", "POP * PCRUM * NRUF", "NR")

# ---- settings
settings = [("initial_time", "1900"), ("final_time", "2100"), ("time_step", args.dt), ("method", "euler"), ("time_units", "year"),
            ("title", "World3 (1974), transcribed from PyWorld3")]
head = [{"op": "set_setting", "key": k, "value": v} for k, v in settings]
# stocks first (flows name them in from/to), then everything else in the order written; constants and lookups need no order
order = {"constant": 0, "lookup": 0, "stock": 1}
body = head + [o for o in ops if o.get("kind") in ("constant", "stock") or o["op"] == "set_table" or o.get("kind") == "lookup"] \
       + [o for o in ops if o.get("kind") in ("aux", "flow")]
fc = find_fc()
out = os.path.abspath(args.out)
os.makedirs(os.path.dirname(out), exist_ok=True)
for p in (out, out + ".oplog.jsonl"):
    if os.path.exists(p):
        os.unlink(p)
with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
    json.dump(body, f)
r = subprocess.run([fc, "ops", out, f.name, "--reason", "build World3 1974 from PyWorld3"], capture_output=True, text=True)
os.unlink(f.name)
ok = r.returncode == 0 and '"ok": true' in r.stdout.replace('":true', '": true')
print("ops applied" if ok else r.stdout[:3000] + r.stderr[:1000])
if not ok:
    sys.exit(1)

# ---- sources table
with open(args.sources, "w") as fh:
    fh.write("# World3 (1974) in Dynamist: where every equation comes from\n\n")
    fh.write("Generated by `benchmarks/b3/models/build_world3_1974.py` (do not edit by hand). Source: PyWorld3 (https://github.com/cvanwynsberghe/pyworld3, "
             "files under `pyworld3/pyworld3/`, `file:line` is the line of the update equation, of the default constant, or of the table's `y.name` in the JSON). "
             "Dynamist equations are the Python equations written once; the time step is Euler as in PyWorld3.\n\n")
    fh.write("## Differences between the two schemes\n\n")
    fh.write(open(os.path.join(HERE, "W3_1974_DIFFERENCES.md")).read() if os.path.exists(os.path.join(HERE, "W3_1974_DIFFERENCES.md")) else "")
    fh.write("\n## Equations\n\n| Variable | Kind | Dynamist equation | PyWorld3 source | Note |\n|---|---|---|---|---|\n")
    for n_, k_, e_, s_, nt in rows:
        fh.write("| %s | %s | `%s` | %s | %s |\n" % (n_, k_, e_.replace("|", "\\|"), s_, nt))
print("sources written", args.sources, "variables", len(rows))

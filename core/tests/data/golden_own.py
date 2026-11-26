"""Independent computation of the reference values used by the Dynamist function-library tests.
Plain Python (no third-party packages), written from the textbook formulas and Excel's documented conventions,
not from the Rust code. Re-create golden.json with: python3 golden.py > golden.json"""
import json, math
from statistics import NormalDist

def pmt(r, n, pv, fv=0.0, typ=0):
    if r == 0: return -(pv + fv) / n
    return -(pv * (1 + r) ** n + fv) * r / ((1 + r * typ) * ((1 + r) ** n - 1))

def fv_(r, n, p, pv=0.0, typ=0):
    if r == 0: return -(pv + p * n)
    return -(pv * (1 + r) ** n + p * (1 + r * typ) * ((1 + r) ** n - 1) / r)

def pv_(r, n, p, fv=0.0, typ=0):
    if r == 0: return -(fv + p * n)
    return -(fv + p * (1 + r * typ) * ((1 + r) ** n - 1) / r) / (1 + r) ** n

def nper(r, p, pv, fv=0.0, typ=0):
    if r == 0: return -(pv + fv) / p
    return math.log((p * (1 + r * typ) - fv * r) / (p * (1 + r * typ) + pv * r)) / math.log(1 + r)

def ipmt(r, per, n, pv, fv=0.0, typ=0):
    p = pmt(r, n, pv, fv, typ)
    if typ == 1 and per == 1: return 0.0
    i = fv_(r, per - 1, p, pv, typ) * r
    return i / (1 + r) if typ == 1 else i

def ppmt(r, per, n, pv, fv=0.0, typ=0):
    return pmt(r, n, pv, fv, typ) - ipmt(r, per, n, pv, fv, typ)

def rate(n, p, pv, fv=0.0, typ=0):
    f = lambda r: pv * (1 + r) ** n + p * (1 + r * typ) * ((1 + r) ** n - 1) / r + fv if r else pv + p * n + fv
    lo, hi = -0.99, 10.0
    for _ in range(300):
        mid = (lo + hi) / 2
        if (f(lo) < 0) == (f(mid) < 0): lo = mid
        else: hi = mid
    return (lo + hi) / 2

def annuity(r, n):
    return 1 / n if r == 0 else r / (1 - (1 + r) ** -n)

def real_rate(nom, inf): return (1 + nom) / (1 + inf) - 1
def effective(nom, m): return (1 + nom / m) ** m - 1
def wacc(d, kd, ke, t=0.0): return d * kd * (1 - t) + (1 - d) * ke

def lcoe(capex, om, yield_, years, rate_, degr=0.0):
    pvc = capex + sum(om / (1 + rate_) ** k for k in range(1, years + 1))
    pve = sum(yield_ * (1 - degr) ** (k - 1) / (1 + rate_) ** k for k in range(1, years + 1))
    return pvc / pve

if __name__ != '__main__':
    pass
nrel_r = real_rate(wacc(0.70, 0.07, 0.085, 0.257), 0.025)
frau_r = real_rate(wacc(0.80, 0.05, 0.065, 0.0), 0.018)
default_r = real_rate(wacc(0.8, 0.05, 0.08, 0.0), 0.02)
nd = NormalDist()
out = {
  "pmt_loan": pmt(0.05/12, 360, 200000),
  "ipmt_first": ipmt(0.05/12, 1, 360, 200000),
  "ppmt_first": ppmt(0.05/12, 1, 360, 200000),
  "ipmt_12": ipmt(0.05/12, 12, 360, 200000),
  "ipmt_due_2": ipmt(0.05/12, 2, 360, 200000, 0, 1),
  "pmt_due": pmt(0.05/12, 360, 200000, 0, 1),
  "pmt_zero_rate": pmt(0, 10, 1000),
  "fv": fv_(0.06/12, 120, -100, -1000),
  "pv": pv_(0.08, 20, -500, 1000),
  "nper": nper(0.07/12, -200, 5000),
  "rate": rate(48, -200, 8000),
  "annuity_factor": annuity(0.08, 20),
  "annuity_factor_zero": annuity(0, 20),
  "real_rate": real_rate(0.07, 0.025),
  "nominal_rate": (1 + 0.04) * (1 + 0.02) - 1,
  "effective_rate": effective(0.12, 12),
  "wacc": wacc(0.7, 0.07, 0.085, 0.257),
  "nrel_real_wacc": nrel_r,
  "fraunhofer_real_wacc": frau_r,
  "default_real_wacc": default_r,
  "lcoe_nrel_usd_per_mwh": lcoe(1430, 24, 0.283 * 8760, 30, nrel_r) ,
  "lcoe_fraunhofer_eur_per_mwh": lcoe(700, 13.3, 1280, 30, frau_r, 0.0025) * 1000,
  "lcoe_default_usd_per_mwh": lcoe(750, 12, 1800, 30, default_r, 0.005) * 1000,
  "normal_cdf_1": nd.cdf(1.0),
  "normal_cdf_m1_96": nd.cdf(-1.96),
  "normal_cdf_x": nd.cdf(12.0, 10.0, 2.0) if False else NormalDist(10, 2).cdf(12.0),
  "normal_inv_975": nd.inv_cdf(0.975),
  "normal_inv_01": nd.inv_cdf(0.01),
  "normal_inv_x": NormalDist(10, 2).inv_cdf(0.9),
  "lognormal_cdf": nd.cdf((math.log(3.0) - 0.5) / 0.4),
  "lognormal_inv": math.exp(0.5 + 0.4 * nd.inv_cdf(0.8)),
}
print(json.dumps(out, indent=2))

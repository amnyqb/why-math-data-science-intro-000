//! Scalar finance, as Excel defines it. All rates are per period; money follows Excel's sign convention (paid out is
//! negative). Arguments `typ` is 0 for payments at the end of each period and 1 for the start (an annuity due).
//!
//! Formulas: Microsoft's documentation of PMT, FV, PV, NPER, IPMT, PPMT and RATE; the closed forms are the standard
//! time-value-of-money identities. The reference values the tests use were computed independently in Python
//! (`core/tests/data/golden.py`).

/// (1 + nominal) / (1 + inflation) - 1 (the Fisher equation).
pub fn real_rate(nominal: f64, inflation: f64) -> f64 {
    (1.0 + nominal) / (1.0 + inflation) - 1.0
}

/// (1 + real) * (1 + inflation) - 1.
pub fn nominal_rate(real: f64, inflation: f64) -> f64 {
    (1.0 + real) * (1.0 + inflation) - 1.0
}

/// (1 + nominal / m)^m - 1 (Excel's EFFECT).
pub fn effective_rate(nominal: f64, m: f64) -> f64 {
    (1.0 + nominal / m).powf(m) - 1.0
}

/// D * kd * (1 - T) + (1 - D) * ke, with D the debt share, kd the cost of debt, ke the cost of equity and T the tax rate.
pub fn wacc(d: f64, kd: f64, ke: f64, t: f64) -> f64 {
    d * kd * (1.0 - t) + (1.0 - d) * ke
}

/// The capital recovery factor r / (1 - (1 + r)^-n); 1 / n at r = 0.
pub fn annuity_factor(r: f64, n: f64) -> f64 {
    if r == 0.0 { 1.0 / n } else { r / (1.0 - (1.0 + r).powf(-n)) }
}

pub fn pmt(r: f64, n: f64, pv: f64, fv: f64, typ: f64) -> f64 {
    if r == 0.0 {
        return -(pv + fv) / n;
    }
    -(pv * (1.0 + r).powf(n) + fv) * r / ((1.0 + r * typ) * ((1.0 + r).powf(n) - 1.0))
}

pub fn fv(r: f64, n: f64, pmt: f64, pv: f64, typ: f64) -> f64 {
    if r == 0.0 {
        return -(pv + pmt * n);
    }
    -(pv * (1.0 + r).powf(n) + pmt * (1.0 + r * typ) * ((1.0 + r).powf(n) - 1.0) / r)
}

pub fn pv(r: f64, n: f64, pmt: f64, fv: f64, typ: f64) -> f64 {
    if r == 0.0 {
        return -(fv + pmt * n);
    }
    -(fv + pmt * (1.0 + r * typ) * ((1.0 + r).powf(n) - 1.0) / r) / (1.0 + r).powf(n)
}

pub fn nper(r: f64, pmt: f64, pv: f64, fv: f64, typ: f64) -> f64 {
    if r == 0.0 {
        return -(pv + fv) / pmt;
    }
    ((pmt * (1.0 + r * typ) - fv * r) / (pmt * (1.0 + r * typ) + pv * r)).ln() / (1.0 + r).ln()
}

/// The interest part of payment number `per` (1-based).
pub fn ipmt(r: f64, per: f64, n: f64, pv_: f64, fv_: f64, typ: f64) -> f64 {
    let p = pmt(r, n, pv_, fv_, typ);
    if typ == 1.0 && per == 1.0 {
        return 0.0;
    }
    let i = fv(r, per - 1.0, p, pv_, typ) * r;
    if typ == 1.0 { i / (1.0 + r) } else { i }
}

pub fn ppmt(r: f64, per: f64, n: f64, pv_: f64, fv_: f64, typ: f64) -> f64 {
    pmt(r, n, pv_, fv_, typ) - ipmt(r, per, n, pv_, fv_, typ)
}

/// The rate per period that solves pv (1+r)^n + pmt (1 + r typ)((1+r)^n - 1)/r + fv = 0. Newton from `guess` (tolerance
/// 1e-10, at most 100 steps); if that does not converge, bisection on [-0.99, 10]; None if there is no root there.
pub fn rate(n: f64, pmt: f64, pv: f64, fv: f64, typ: f64, guess: f64) -> Option<f64> {
    let f = |r: f64| -> f64 {
        if r.abs() < 1e-12 { pv + pmt * n + fv } else { pv * (1.0 + r).powf(n) + pmt * (1.0 + r * typ) * ((1.0 + r).powf(n) - 1.0) / r + fv }
    };
    let mut r = guess;
    for _ in 0..100 {
        let h = 1e-7 * r.abs().max(1e-4);
        let (y, d) = (f(r), (f(r + h) - f(r - h)) / (2.0 * h));
        if !y.is_finite() || !d.is_finite() || d == 0.0 {
            break;
        }
        let next = r - y / d;
        if next <= -1.0 {
            break;
        }
        if (next - r).abs() < 1e-10 {
            return Some(next);
        }
        r = next;
    }
    let (mut lo, mut hi) = (-0.99, 10.0);
    let (flo, fhi) = (f(lo), f(hi));
    if !flo.is_finite() || !fhi.is_finite() || flo.signum() == fhi.signum() {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(mid).signum() == f(lo).signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// cash / debt service, or `when_zero` when there is no debt service.
pub fn dscr(cash: f64, debt_service: f64, when_zero: f64) -> f64 {
    if debt_service.abs() < 1e-6 { when_zero } else { cash / debt_service }
}

// ------------------------------------------------------------------------------------------- cash streams (dyn finance)
//
// A stream is the flows of the steps of a run (a rate times the step). The rate is per model time unit, whatever the
// step: the flow of step k, at the time (k + off) * dt after the start, is divided by (1 + r)^((k + off) * dt).
// `off` is 0 for period_start (Excel: NPV(r, flows[1..]) + flows[0]), 0.5 for mid_period and 1 for period_end (the flow
// of step 0 is discounted by one period).

/// The present value of a stream of flows at the rate `r` per time unit.
pub fn stream_pv(flows: &[f64], r: f64, dt: f64, off: f64) -> f64 {
    flows.iter().enumerate().map(|(k, v)| v / (1.0 + r).powf((k as f64 + off) * dt)).sum()
}

/// The rate per time unit that makes the present value of the stream zero. The rates are scanned on a grid from -99% to
/// 10 000 000% for a change of sign (the root nearest to zero wins) and refined by bisection; None if there is none.
pub fn stream_irr(flows: &[f64], dt: f64, off: f64) -> Option<f64> {
    if flows.len() < 2 || !flows.iter().any(|x| *x > 0.0) || !flows.iter().any(|x| *x < 0.0) {
        return None;
    }
    let mut grid: Vec<f64> = (0..=99).map(|i| -0.99 + 0.01 * i as f64).collect();
    grid.push(0.0);
    grid.extend((0..=600).map(|i| 1e-6 * 10f64.powf(i as f64 / 50.0)));
    grid.sort_by(|a, b| a.total_cmp(b));
    grid.dedup();
    let f = |r: f64| stream_pv(flows, r, dt, off);
    let mut best: Option<f64> = None;
    for w in grid.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (fa, fb) = (f(a), f(b));
        if !fa.is_finite() || !fb.is_finite() {
            continue;
        }
        let root = if fa == 0.0 {
            Some(a)
        } else if fa.signum() != fb.signum() && fb != 0.0 {
            let (mut lo, mut hi) = (a, b);
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if f(mid).signum() == f(lo).signum() { lo = mid } else { hi = mid }
            }
            Some(0.5 * (lo + hi))
        } else {
            None
        };
        if let Some(r) = root {
            if best.is_none_or(|x| r.abs() < x.abs()) {
                best = Some(r);
            }
        }
    }
    best
}

/// Excel's MIRR on a stream: positives compounded at `reinvest`, negatives discounted at `finance` (both per time unit),
/// the result per time unit.
pub fn stream_mirr(flows: &[f64], finance: f64, reinvest: f64, dt: f64) -> Option<f64> {
    let n = flows.len();
    if n < 2 {
        return None;
    }
    let pos: f64 = flows.iter().enumerate().filter(|(_, v)| **v > 0.0).map(|(k, v)| v * (1.0 + reinvest).powf((n - 1 - k) as f64 * dt)).sum();
    let neg: f64 = flows.iter().enumerate().filter(|(_, v)| **v < 0.0).map(|(k, v)| v / (1.0 + finance).powf(k as f64 * dt)).sum();
    if neg < 0.0 && pos > 0.0 { Some((pos / -neg).powf(1.0 / ((n - 1) as f64 * dt)) - 1.0) } else { None }
}

/// The functions of the equation language: name, code, fewest and most arguments.
pub const FUNCTIONS: &[(&str, u8, usize, usize)] = &[
    ("REAL RATE", 0, 2, 2),
    ("NOMINAL RATE", 1, 2, 2),
    ("EFFECTIVE RATE", 2, 2, 2),
    ("WACC", 3, 3, 4),
    ("ANNUITY FACTOR", 4, 2, 2),
    ("PMT", 5, 3, 5),
    ("FV", 6, 3, 5),
    ("PV", 7, 3, 5),
    ("NPER", 8, 3, 5),
    ("IPMT", 9, 4, 6),
    ("PPMT", 10, 4, 6),
    ("RATE", 11, 3, 6),
    ("DSCR", 12, 2, 2),
];

pub fn code_of(name: &str) -> Option<u8> {
    FUNCTIONS.iter().find(|f| f.0 == name).map(|f| f.1)
}

/// Evaluate the function with this code; optional arguments take Excel's defaults (fv 0, type 0, guess 0.1, tax 0).
pub fn eval(code: u8, v: &[f64]) -> f64 {
    let a = |i: usize, d: f64| v.get(i).copied().unwrap_or(d);
    match code {
        0 => real_rate(v[0], v[1]),
        1 => nominal_rate(v[0], v[1]),
        2 => effective_rate(v[0], v[1]),
        3 => wacc(v[0], v[1], v[2], a(3, 0.0)),
        4 => annuity_factor(v[0], v[1]),
        5 => pmt(v[0], v[1], v[2], a(3, 0.0), a(4, 0.0)),
        6 => fv(v[0], v[1], v[2], a(3, 0.0), a(4, 0.0)),
        7 => pv(v[0], v[1], v[2], a(3, 0.0), a(4, 0.0)),
        8 => nper(v[0], v[1], v[2], a(3, 0.0), a(4, 0.0)),
        9 => ipmt(v[0], v[1], v[2], v[3], a(4, 0.0), a(5, 0.0)),
        10 => ppmt(v[0], v[1], v[2], v[3], a(4, 0.0), a(5, 0.0)),
        11 => rate(v[0], v[1], v[2], a(3, 0.0), a(4, 0.0), a(5, 0.1)).unwrap_or(f64::NAN),
        12 => dscr(v[0], v[1], 1e9),
        _ => f64::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, rel: f64) {
        assert!((a - b).abs() <= rel * b.abs().max(1e-12), "{a} vs {b}");
    }

    // golden values: core/tests/data/golden.json (golden.py)
    #[test]
    fn excel_loan_functions() {
        let r = 0.05 / 12.0;
        close(pmt(r, 360.0, 200_000.0, 0.0, 0.0), -1073.6432460242795, 1e-9);
        close(ipmt(r, 1.0, 360.0, 200_000.0, 0.0, 0.0), -833.3333333333334, 1e-9);
        close(ppmt(r, 1.0, 360.0, 200_000.0, 0.0, 0.0), -240.30991269094613, 1e-9);
        close(ipmt(r, 12.0, 360.0, 200_000.0, 0.0, 0.0), -822.0867740866777, 1e-9);
        close(pmt(r, 360.0, 200_000.0, 0.0, 1.0), -1069.188294795963, 1e-9);
        assert_eq!(ipmt(r, 1.0, 360.0, 200_000.0, 0.0, 1.0), 0.0);
        close(ipmt(r, 2.0, 360.0, 200_000.0, 0.0, 1.0), -828.8783821050168, 1e-9);
        close(pmt(0.0, 10.0, 1000.0, 0.0, 0.0), -100.0, 1e-12);
    }

    #[test]
    fn excel_time_value_functions() {
        close(fv(0.06 / 12.0, 120.0, -100.0, -1000.0, 0.0), 18207.33141467809, 1e-9);
        close(pv(0.08, 20.0, -500.0, 1000.0, 0.0), 4694.52549632059, 1e-9);
        close(nper(0.07 / 12.0, -200.0, 5000.0, 0.0, 0.0), 27.100842790202556, 1e-9);
        close(rate(48.0, -200.0, 8000.0, 0.0, 0.0, 0.1).unwrap(), 0.007701472488201888, 1e-8);
        close(annuity_factor(0.08, 20.0), 0.10185220882315058, 1e-9);
        close(annuity_factor(0.0, 20.0), 0.05, 1e-12);
    }

    #[test]
    fn rates_and_wacc() {
        close(real_rate(0.07, 0.025), 0.043902439024390505, 1e-9);
        close(nominal_rate(0.04, 0.02), 0.0608, 1e-12);
        close(effective_rate(0.12, 12.0), 0.12682503013196977, 1e-9);
        close(wacc(0.7, 0.07, 0.085, 0.257), 0.061907, 1e-9);
        close(real_rate(wacc(0.8, 0.05, 0.08, 0.0), 0.02), 0.03529411764705892, 1e-9);
    }

    #[test]
    fn rate_falls_back_to_bisection_and_gives_up_when_there_is_no_root() {
        // a bad guess still finds the root
        close(rate(48.0, -200.0, 8000.0, 0.0, 0.0, 5.0).unwrap(), 0.007701472488201888, 1e-8);
        // payments that cannot repay the loan at any rate in range
        assert!(rate(10.0, 1.0, 1000.0, 0.0, 0.0, 0.1).is_none());
    }

    #[test]
    fn streams_match_excel_at_one_period_and_scale_with_the_step() {
        let flows = [-1000.0, 300.0, 400.0, 500.0, 200.0];
        // golden.json: NPV excel-style(0.08; 300,400,500,200) - 1000, and IRR(-1000,300,400,500,200)
        close(stream_pv(&flows, 0.08, 1.0, 0.0), 164.6353969679, 1e-9);
        close(stream_irr(&flows, 1.0, 0.0).unwrap(), 0.1532213788, 1e-9);
        close(stream_mirr(&flows, 0.10, 0.12, 1.0).unwrap(), 0.1390332647, 1e-9);
        // the same flows on a half-year step, the rate per year: the flows are 0.5 years apart
        close(stream_pv(&flows, 0.08, 0.5, 0.0), -1000.0 + flows[1..].iter().enumerate().map(|(k, v)| v / 1.08f64.powf((k + 1) as f64 * 0.5)).sum::<f64>(), 1e-9);
        let r = stream_irr(&flows, 0.5, 0.0).unwrap();
        assert!(stream_pv(&flows, r, 0.5, 0.0).abs() < 1e-6);
        close((1.0 + r).powf(0.5), 1.1532213788, 1e-9);
        // period_end discounts the first flow by one period
        close(stream_pv(&[100.0], 0.1, 1.0, 1.0), 100.0 / 1.1, 1e-12);
    }

    #[test]
    fn an_irr_below_zero_or_far_above_is_found() {
        close(stream_irr(&[-100.0, 50.0, 30.0], 1.0, 0.0).unwrap(), -0.14792027106038524, 1e-9);
        let r = stream_irr(&[-1.0, 0.0, 0.0, 1e6], 1.0, 0.0).unwrap();
        assert!(stream_pv(&[-1.0, 0.0, 0.0, 1e6], r, 1.0, 0.0).abs() < 1e-6);
        assert!(stream_irr(&[1.0, 2.0, 3.0], 1.0, 0.0).is_none());
    }
}

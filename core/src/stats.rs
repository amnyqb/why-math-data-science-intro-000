//! Statistics written by hand (no crates): the normal distribution (cdf and inverse), the log-normal, a seedable random
//! generator, and the samplers behind the `RANDOM …` functions.
//!
//! * `erf` and `erfc`: the Maclaurin series of `erf` in the all-positive form `erf(x) = 2/sqrt(pi) e^{-x^2} sum 2^n x^{2n+1} /
//!   (1 3 5 ... (2n+1))` (Abramowitz and Stegun 7.1.6) for |x| <= 3, and the continued fraction of `erfc` (A&S 7.1.14) beyond.
//!   Both are accurate to about 1e-15.
//! * `normal_inv`: Peter Acklam's rational approximation (relative error 1.15e-9, <https://stackedboxes.org/2017/05/01/acklams-normal-quantile-function/>)
//!   followed by two Halley steps on the accurate cdf, which brings it to full double precision.
//! * The random generator is SplitMix64 (Steele, Lea and Flood 2014) for seeding and xoshiro-free: SplitMix64 itself is
//!   the stream (period 2^64, passes BigCrush), which is plenty for system dynamics runs.

use std::f64::consts::PI;

/// The error function.
pub fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x.abs() > 3.0 {
        return x.signum() * (1.0 - erfc_cf(x.abs()));
    }
    if x == 0.0 {
        return 0.0;
    }
    let mut term = x;
    let mut sum = x;
    let mut n = 0.0;
    loop {
        n += 1.0;
        term *= 2.0 * x * x / (2.0 * n + 1.0);
        sum += term;
        if term.abs() <= 1e-17 * sum.abs() || n > 200.0 {
            break;
        }
    }
    2.0 / PI.sqrt() * (-x * x).exp() * sum
}

/// erfc for x > 3 by the continued fraction (evaluated from the tail, 80 terms).
fn erfc_cf(x: f64) -> f64 {
    let mut f = x;
    for k in (1..=80).rev() {
        f = x + (k as f64 / 2.0) / f;
    }
    (-x * x).exp() / (PI.sqrt() * f)
}

pub fn erfc(x: f64) -> f64 {
    if x > 3.0 { erfc_cf(x) } else if x < -3.0 { 2.0 - erfc_cf(-x) } else { 1.0 - erf(x) }
}

/// The standard normal cdf.
pub fn normal_cdf_std(z: f64) -> f64 {
    0.5 * erfc(-z / std::f64::consts::SQRT_2)
}

pub fn normal_cdf(x: f64, mean: f64, sd: f64) -> f64 {
    normal_cdf_std((x - mean) / sd)
}

/// The inverse of the standard normal cdf (the quantile function).
pub fn normal_inv_std(p: f64) -> f64 {
    if p.is_nan() || !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    // Acklam's coefficients
    const A: [f64; 6] = [-3.969683028665376e+01, 2.209460984245205e+02, -2.759285104469687e+02, 1.383577518672690e+02, -3.066479806614716e+01, 2.506628277459239e+00];
    const B: [f64; 5] = [-5.447609879822406e+01, 1.615858368580409e+02, -1.556989798598866e+02, 6.680131188771972e+01, -1.328068155288572e+01];
    const C: [f64; 6] = [-7.784894002430293e-03, -3.223964580411365e-01, -2.400758277161838e+00, -2.549732539343734e+00, 4.374664141464968e+00, 2.938163982698783e+00];
    const D: [f64; 4] = [7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00, 3.754408661907416e+00];
    let plow = 0.02425;
    let mut x = if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - plow {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    // two Halley steps on the accurate cdf
    for _ in 0..2 {
        let e = normal_cdf_std(x) - p;
        let u = e * (2.0 * PI).sqrt() * (x * x / 2.0).exp();
        x -= u / (1.0 + x * u / 2.0);
    }
    x
}

pub fn normal_inv(p: f64, mean: f64, sd: f64) -> f64 {
    mean + sd * normal_inv_std(p)
}

/// Log-normal cdf with parameters mu and sigma of the underlying normal.
pub fn lognormal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if x <= 0.0 { 0.0 } else { normal_cdf_std((x.ln() - mu) / sigma) }
}

pub fn lognormal_inv(p: f64, mu: f64, sigma: f64) -> f64 {
    (mu + sigma * normal_inv_std(p)).exp()
}

// ---------------------------------------------------------------------------------------------------- the generator

/// SplitMix64: a 64-bit state, one multiply-xorshift mix per draw.
#[derive(Clone, Debug)]
pub struct Rng {
    pub state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A uniform number in [0, 1) with 53 bits.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A uniform number in (0, 1): never exactly 0.
    pub fn uniform_open(&mut self) -> f64 {
        loop {
            let u = self.uniform();
            if u > 0.0 {
                return u;
            }
        }
    }
    /// A standard normal draw (Marsaglia's polar method: two uniforms per pair; one is kept for the next draw is not
    /// needed here, so each call uses a fresh pair, keeping the stream a pure function of the call count).
    pub fn normal(&mut self) -> f64 {
        loop {
            let (u, v) = (2.0 * self.uniform() - 1.0, 2.0 * self.uniform() - 1.0);
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                return u * (-2.0 * s.ln() / s).sqrt();
            }
        }
    }
    pub fn exponential(&mut self) -> f64 {
        -self.uniform_open().ln()
    }
    /// Gamma(shape, scale 1): Marsaglia and Tsang (2000); shape < 1 by the boost `G(a) = G(a+1) U^(1/a)`.
    pub fn gamma(&mut self, shape: f64) -> f64 {
        if shape < 1.0 {
            let u = self.uniform_open();
            return self.gamma(shape + 1.0) * u.powf(1.0 / shape);
        }
        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();
        loop {
            let x = self.normal();
            let v = (1.0 + c * x).powi(3);
            if v <= 0.0 {
                continue;
            }
            let u = self.uniform_open();
            if u.ln() < 0.5 * x * x + d - d * v + d * v.ln() {
                return d * v;
            }
        }
    }
    pub fn beta(&mut self, a: f64, b: f64) -> f64 {
        let (x, y) = (self.gamma(a), self.gamma(b));
        x / (x + y)
    }
    /// Poisson(mean): Knuth's product method below 30; the normal approximation with continuity correction is not used,
    /// instead the sum of Poisson(mean/k) pieces keeps the exact distribution for large means.
    pub fn poisson(&mut self, mean: f64) -> f64 {
        if mean <= 0.0 {
            return 0.0;
        }
        if mean > 30.0 {
            let k = (mean / 30.0).ceil();
            return (0..k as usize).map(|_| self.poisson(mean / k)).sum();
        }
        let l = (-mean).exp();
        let (mut k, mut p) = (0.0, 1.0);
        loop {
            p *= self.uniform();
            if p <= l {
                return k;
            }
            k += 1.0;
        }
    }
    /// Binomial(n, p): the sum of n Bernoulli draws (n is a count of trials, kept exact).
    pub fn binomial(&mut self, n: f64, p: f64) -> f64 {
        (0..n.max(0.0) as usize).filter(|_| self.uniform() < p).count() as f64
    }
    /// Number of failures before the r-th success: a Gamma-Poisson mixture.
    pub fn negative_binomial(&mut self, r: f64, p: f64) -> f64 {
        if p >= 1.0 {
            return 0.0;
        }
        let lambda = self.gamma(r) * (1.0 - p) / p;
        self.poisson(lambda)
    }
    /// Number of failures before the first success (support 0, 1, 2, ...).
    pub fn geometric(&mut self, p: f64) -> f64 {
        if p >= 1.0 {
            return 0.0;
        }
        (self.uniform_open().ln() / (1.0 - p).ln()).floor()
    }
}

/// The inverse cdf of a triangular distribution.
pub fn triangular_inv(u: f64, low: f64, mode: f64, high: f64) -> f64 {
    let f = (mode - low) / (high - low);
    if u < f { low + (u * (high - low) * (mode - low)).sqrt() } else { high - ((1.0 - u) * (high - low) * (high - mode)).sqrt() }
}

/// A 64-bit hash (FNV-1a over the bytes) of a model seed, a name and a call index: the seed of a stream, so adding a
/// variable never changes the draws of another.
pub fn stream_seed(model_seed: u64, name: &str, call: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for b in model_seed.to_le_bytes() {
        eat(b);
    }
    for b in name.to_lowercase().bytes() {
        eat(b);
    }
    for b in call.to_le_bytes() {
        eat(b);
    }
    // one SplitMix64 round to spread the bits
    Rng::new(h).next_u64()
}

/// The random functions of the equation language: (name, code, number of parameters before the optional seed).
/// The argument order of the `RANDOM …` family is minimum, maximum, then the distribution's parameters, as Vensim's
/// `RANDOM NORMAL(min, max, mean, sd, seed)`; the short forms and the Stella names take the parameters only.
pub const RANDOMS: &[(&str, u8, usize)] = &[
    ("RANDOM TRIANGULAR", 0, 3),
    ("RANDOM LOGNORMAL", 1, 4),
    ("RANDOM GAMMA", 2, 4),
    ("RANDOM BETA", 3, 4),
    ("RANDOM WEIBULL", 4, 4),
    ("RANDOM POISSON", 5, 3),
    ("RANDOM BINOMIAL", 6, 4),
    ("RANDOM NEGATIVE BINOMIAL", 7, 4),
    ("RANDOM GEOMETRIC", 8, 3),
    ("RANDOM PARETO", 9, 4),
    ("RANDOM PERT", 10, 4),
    ("RANDOM 0 1", 11, 0),
    ("TRIANGULAR", 12, 3),
    ("LOGNORMAL", 13, 2),
    ("WEIBULL", 14, 2),
    ("POISSON", 15, 1),
    ("NORMAL", 16, 2),
    ("UNIFORM", 17, 2),
    ("EXPRND", 18, 1),
];

pub fn random_of(name: &str) -> Option<(u8, usize)> {
    RANDOMS.iter().find(|r| r.0 == name).map(|r| (r.1, r.2))
}

/// One draw. `p` holds the parameters in the order of `RANDOMS`.
pub fn draw(code: u8, p: &[f64], r: &mut Rng) -> f64 {
    let clamp = |x: f64| x.clamp(p[0].min(p[1]), p[0].max(p[1]));
    match code {
        0 => clamp(triangular_inv(r.uniform(), p[0], p[2], p[1])),
        1 => {
            // mean and sd of the variable itself
            let s2 = (1.0 + p[3] * p[3] / (p[2] * p[2])).ln();
            clamp((p[2].ln() - s2 / 2.0 + s2.sqrt() * r.normal()).exp())
        }
        2 => clamp(p[3] * r.gamma(p[2])),
        3 => p[0] + (p[1] - p[0]) * r.beta(p[2], p[3]),
        4 => clamp(p[3] * r.exponential().powf(1.0 / p[2])),
        5 => clamp(r.poisson(p[2])),
        6 => clamp(r.binomial(p[2], p[3])),
        7 => clamp(r.negative_binomial(p[2], p[3])),
        8 => clamp(r.geometric(p[2])),
        9 => clamp(p[3] * r.uniform_open().powf(-1.0 / p[2])),
        10 => {
            // PERT: a beta stretched between min and max with its mean pulled to the mode by `shape` (4 is the usual)
            let (lo, hi, mode, shape) = (p[0], p[1], p[2], p[3]);
            let a = 1.0 + shape * (mode - lo) / (hi - lo);
            let b = 1.0 + shape * (hi - mode) / (hi - lo);
            lo + (hi - lo) * r.beta(a, b)
        }
        11 => r.uniform(),
        12 => triangular_inv(r.uniform(), p[0], p[1], p[2]),
        13 => {
            let s2 = (1.0 + p[1] * p[1] / (p[0] * p[0])).ln();
            (p[0].ln() - s2 / 2.0 + s2.sqrt() * r.normal()).exp()
        }
        14 => p[1] * r.exponential().powf(1.0 / p[0]),
        15 => r.poisson(p[0]),
        16 => p[0] + p[1] * r.normal(),
        17 => p[0] + (p[1] - p[0]) * r.uniform(),
        _ => p[0] * r.exponential(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, rel: f64) {
        assert!((a - b).abs() <= rel * b.abs().max(1e-300), "{a} vs {b}");
    }

    // golden values: core/tests/data/golden.json (golden.py, Python's statistics.NormalDist)
    #[test]
    fn normal_cdf_and_inverse_match_the_golden_values() {
        close(normal_cdf_std(1.0), 0.8413447460685429, 1e-9);
        close(normal_cdf_std(-1.96), 0.02499789514822043, 1e-9);
        close(normal_cdf(12.0, 10.0, 2.0), 0.8413447460685429, 1e-9);
        close(normal_inv_std(0.975), 1.9599639845400534, 1e-9);
        close(normal_inv_std(0.01), -2.3263478740408408, 1e-9);
        close(normal_inv(0.9, 10.0, 2.0), 12.563103131089202, 1e-9);
        close(lognormal_cdf(3.0, 0.5, 0.4), 0.9327422958725134, 1e-9);
        close(lognormal_inv(0.8, 0.5, 0.4), 2.3086166525239706, 1e-9);
        assert_eq!(normal_inv_std(0.5), 0.0);
        // the tails and the round trip
        for p in [1e-10, 1e-5, 0.001, 0.3, 0.5, 0.9, 0.999, 1.0 - 1e-9] {
            close(normal_cdf_std(normal_inv_std(p)), p, 1e-9);
        }
        close(erf(0.5), 0.5204998778130465, 1e-12);
        close(erfc(4.0), 1.541725790028002e-8, 1e-9);
    }

    #[test]
    fn the_generator_is_repeatable_and_streams_are_independent_of_each_other() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!((0..5).map(|_| a.next_u64()).collect::<Vec<_>>(), (0..5).map(|_| b.next_u64()).collect::<Vec<_>>());
        assert_ne!(stream_seed(1, "Demand", 0), stream_seed(1, "Demand", 1));
        assert_ne!(stream_seed(1, "Demand", 0), stream_seed(1, "Supply", 0));
        assert_ne!(stream_seed(1, "Demand", 0), stream_seed(2, "Demand", 0));
        assert_eq!(stream_seed(1, "Demand", 0), stream_seed(1, "demand", 0));
    }

    /// 200,000 draws: the sample mean lies within 4 standard errors of the theoretical mean.
    fn within_four_se(mean: f64, sd: f64, mut draw: impl FnMut() -> f64) {
        let n = 200_000;
        let xs: Vec<f64> = (0..n).map(|_| draw()).collect();
        let m = xs.iter().sum::<f64>() / n as f64;
        let se = sd / (n as f64).sqrt();
        assert!((m - mean).abs() < 4.0 * se, "sample mean {m} vs {mean} (4 se = {})", 4.0 * se);
    }

    #[test]
    fn every_sampler_has_the_right_mean() {
        let mut r = Rng::new(1);
        within_four_se(0.5, (1.0f64 / 12.0).sqrt(), || r.uniform());
        let mut r = Rng::new(1);
        within_four_se(0.0, 1.0, || r.normal());
        let mut r = Rng::new(1);
        within_four_se(1.0, 1.0, || r.exponential());
        let mut r = Rng::new(1);
        within_four_se(3.0, 3.0f64.sqrt(), || r.gamma(3.0));
        let mut r = Rng::new(1);
        within_four_se(0.4, (0.4f64 * 0.6 / 6.0).sqrt(), || r.beta(2.0, 3.0));
        let mut r = Rng::new(1);
        within_four_se(4.0, 2.0, || r.poisson(4.0));
        let mut r = Rng::new(1);
        within_four_se(50.0, 50.0f64.sqrt(), || r.poisson(50.0));
        let mut r = Rng::new(1);
        within_four_se(3.0, (10.0f64 * 0.3 * 0.7).sqrt(), || r.binomial(10.0, 0.3));
        let mut r = Rng::new(1);
        // failures before the 3rd success, p = 0.4: mean r(1-p)/p = 4.5, variance r(1-p)/p^2 = 11.25
        within_four_se(4.5, 11.25f64.sqrt(), || r.negative_binomial(3.0, 0.4));
        let mut r = Rng::new(1);
        // failures before the first success, p = 0.25: mean 3, variance 12
        within_four_se(3.0, 12.0f64.sqrt(), || r.geometric(0.25));
        let mut r = Rng::new(1);
        // triangular(0, 2, 6): mean 8/3, variance (0 + 4 + 36 - 0 - 0 - 12)/18 = 28/18
        within_four_se(8.0 / 3.0, (28.0f64 / 18.0).sqrt(), || triangular_inv(r.uniform(), 0.0, 2.0, 6.0));
    }
}

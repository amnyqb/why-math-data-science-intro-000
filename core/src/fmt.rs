//! Number formatting shared by the app, the checks and the command line.

pub fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return if v.is_nan() {
            "–".into()
        } else if v > 0.0 {
            "∞".into()
        } else {
            "-∞".into()
        };
    }
    let a = v.abs();
    if a == 0.0 {
        "0".into()
    } else if a >= 1e6 {
        format!("{:.3}M", v / 1e6)
    } else if a >= 1e4 {
        format!("{:.0}", v)
    } else if a >= 100.0 {
        format!("{:.1}", v)
    } else if a >= 1.0 {
        format!("{:.3}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else if a >= 1e-3 {
        format!("{:.4}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        format!("{:.3e}", v)
    }
}

//! Two small charts drawn straight to SVG text, for the reports that need a shape the model-chart code does not have:
//! a waterfall (why a goal changed: `dyn attribute`) and a fan chart (bands, actuals and the cut: `dyn hindcast`).
//! No dependency and no window. The colours are the palette the figures use (blue for what is kept, orange for what is lost).

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn num(v: f64) -> String {
    if !v.is_finite() {
        return "n/a".into();
    }
    let a = v.abs();
    if a != 0.0 && !(1e-3..1e7).contains(&a) {
        format!("{v:.3e}")
    } else if a >= 100.0 {
        format!("{v:.0}")
    } else if a >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.3}").trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// "Nice" tick values covering [lo, hi].
fn ticks(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let span = (hi - lo).max(1e-12);
    let raw = span / n as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    let mut t = (lo / step).ceil() * step;
    let mut out = Vec::new();
    while t <= hi + step * 1e-9 {
        out.push(if t.abs() < step * 1e-9 { 0.0 } else { t });
        t += step;
    }
    out
}

const W: f64 = 760.0;
const H: f64 = 420.0;
const L: f64 = 70.0;
const R: f64 = 20.0;
const T: f64 = 46.0;
const B: f64 = 96.0;

fn open(title: &str) -> String {
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {W} {H}\" width=\"{W}\" height=\"{H}\" font-family=\"Helvetica, Arial, sans-serif\" font-size=\"12\">\n<rect width=\"{W}\" height=\"{H}\" fill=\"#ffffff\"/>\n");
    s.push_str(&format!("<text x=\"{L}\" y=\"26\" font-size=\"15\" font-weight=\"bold\" fill=\"#222\">{}</text>\n", esc(title)));
    s
}

fn axis(s: &mut String, lo: f64, hi: f64, y_of: &dyn Fn(f64) -> f64) {
    for t in ticks(lo, hi, 5) {
        let y = y_of(t);
        s.push_str(&format!("<line x1=\"{L}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"#e3e3e3\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" fill=\"#555\">{}</text>\n", W - R, L - 6.0, y + 4.0, num(t)));
    }
}

/// Why a number changed: the old value, one bar per cause, the new value. A cause that raised it is blue, one that lowered it orange.
pub fn waterfall(title: &str, start_label: &str, start: f64, steps: &[(String, f64)], end_label: &str, end: f64) -> String {
    let mut level = start;
    let mut lo = start.min(end).min(0.0);
    let mut hi = start.max(end).max(0.0);
    for (_, d) in steps {
        level += d;
        lo = lo.min(level);
        hi = hi.max(level);
    }
    let pad = ((hi - lo) * 0.08).max(1e-9);
    let (lo, hi) = (if lo < 0.0 { lo - pad } else { 0.0 }, hi + pad);
    let (top, bottom) = (T, H - B);
    let y_of = move |v: f64| bottom - (v - lo) / (hi - lo) * (bottom - top);
    let mut s = open(title);
    axis(&mut s, lo, hi, &y_of);
    let n = steps.len() + 2;
    let slot = (W - L - R) / n as f64;
    let bw = slot * 0.62;
    let bar = |s: &mut String, i: usize, from: f64, to: f64, fill: &str, label: &str, value: String| {
        let x = L + slot * i as f64 + (slot - bw) / 2.0;
        let (y1, y2) = (y_of(from.max(to)), y_of(from.min(to)));
        s.push_str(&format!("<rect x=\"{x:.1}\" y=\"{y1:.1}\" width=\"{bw:.1}\" height=\"{:.1}\" fill=\"{fill}\"/>\n", (y2 - y1).max(1.0)));
        s.push_str(&format!("<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#222\">{}</text>\n", x + bw / 2.0, y1 - 5.0, esc(&value)));
        let label: String = if label.chars().count() > 16 { format!("{}…", label.chars().take(15).collect::<String>()) } else { label.to_string() };
        s.push_str(&format!("<text transform=\"translate({:.1},{:.1}) rotate(-35)\" text-anchor=\"end\" fill=\"#333\">{}</text>\n", x + bw / 2.0 + 4.0, bottom + 14.0, esc(&label)));
    };
    bar(&mut s, 0, 0.0, start, "#8c8c8c", start_label, num(start));
    let mut level = start;
    for (i, (name, d)) in steps.iter().enumerate() {
        let next = level + d;
        bar(&mut s, i + 1, level, next, if *d >= 0.0 { "#2b6cb0" } else { "#dd6b20" }, name, format!("{}{}", if *d >= 0.0 { "+" } else { "-" }, num(d.abs())));
        // the connector from this bar to the next
        s.push_str(&format!("<line x1=\"{:.1}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"#999\" stroke-dasharray=\"3 3\"/>\n", L + slot * (i + 1) as f64 + (slot + bw) / 2.0, L + slot * (i + 2) as f64 + (slot - bw) / 2.0, y = y_of(next)));
        level = next;
    }
    bar(&mut s, n - 1, 0.0, end, "#4a4a4a", end_label, num(end));
    s.push_str(&format!("<line x1=\"{L}\" y1=\"{b}\" x2=\"{:.1}\" y2=\"{b}\" stroke=\"#444\"/>\n", W - R, b = y_of(0.0f64.max(lo))));
    s.push_str("</svg>\n");
    s
}

/// One band of a fan chart: its share (0.5 = the middle half) and the lower and upper edge at each time.
pub struct Band {
    pub share: f64,
    pub lo: Vec<f64>,
    pub hi: Vec<f64>,
}

/// Bands of an ensemble over time, the median, the actual points and the cut. Actuals before the cut are dark, after it orange.
pub fn fan(title: &str, y_label: &str, times: &[f64], bands: &[Band], median: &[f64], actuals: &[(f64, f64)], cut: f64) -> String {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for b in bands {
        for v in b.lo.iter().chain(&b.hi) {
            if v.is_finite() {
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
        }
    }
    for (_, v) in actuals {
        if v.is_finite() {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
    }
    if !lo.is_finite() || times.len() < 2 {
        return format!("{}<text x=\"{L}\" y=\"80\">nothing to draw</text></svg>\n", open(title));
    }
    let pad = ((hi - lo) * 0.06).max(1e-9);
    let (lo, hi) = (lo - pad, hi + pad);
    let (t0, t1) = (times[0], times[times.len() - 1]);
    let (top, bottom) = (T, H - 54.0);
    let y_of = move |v: f64| bottom - (v - lo) / (hi - lo) * (bottom - top);
    let x_of = move |t: f64| L + (t - t0) / (t1 - t0).max(1e-12) * (W - L - R);
    let mut s = open(title);
    axis(&mut s, lo, hi, &y_of);
    for t in ticks(t0, t1, 6) {
        s.push_str(&format!("<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#555\">{}</text>\n", x_of(t), bottom + 16.0, num(t)));
    }
    // widest band first so the narrower ones sit on top
    let mut order: Vec<&Band> = bands.iter().collect();
    order.sort_by(|a, b| b.share.partial_cmp(&a.share).unwrap_or(std::cmp::Ordering::Equal));
    for (i, b) in order.iter().enumerate() {
        let shade = ["#cfe0f3", "#9fc1e6", "#6aa0d6"][i.min(2)];
        let mut pts: Vec<String> = times.iter().zip(&b.hi).filter(|(_, v)| v.is_finite()).map(|(t, v)| format!("{:.1},{:.1}", x_of(*t), y_of(*v))).collect();
        pts.extend(times.iter().zip(&b.lo).rev().filter(|(_, v)| v.is_finite()).map(|(t, v)| format!("{:.1},{:.1}", x_of(*t), y_of(*v))));
        s.push_str(&format!("<polygon points=\"{}\" fill=\"{shade}\" stroke=\"none\"/>\n", pts.join(" ")));
    }
    let line: Vec<String> = times.iter().zip(median).filter(|(_, v)| v.is_finite()).map(|(t, v)| format!("{:.1},{:.1}", x_of(*t), y_of(*v))).collect();
    s.push_str(&format!("<polyline points=\"{}\" fill=\"none\" stroke=\"#1a4f8a\" stroke-width=\"2\"/>\n", line.join(" ")));
    let cx = x_of(cut);
    s.push_str(&format!("<line x1=\"{cx:.1}\" y1=\"{top}\" x2=\"{cx:.1}\" y2=\"{bottom}\" stroke=\"#444\" stroke-dasharray=\"5 4\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" fill=\"#444\">cut</text>\n", cx + 4.0, top + 12.0));
    for (t, v) in actuals.iter().filter(|(_, v)| v.is_finite()) {
        s.push_str(&format!("<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.2\" fill=\"{}\" stroke=\"#fff\" stroke-width=\"0.8\"/>\n", x_of(*t), y_of(*v), if *t <= cut { "#333" } else { "#dd6b20" }));
    }
    let ly = H - 18.0;
    let legend = ["median", "50% and 90% bands", "actual before the cut", "actual after the cut"];
    let mut x = L;
    for (i, t) in legend.iter().enumerate() {
        match i {
            0 => s.push_str(&format!("<line x1=\"{x}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#1a4f8a\" stroke-width=\"2\"/>\n", ly - 4.0, x + 18.0, ly - 4.0)),
            1 => s.push_str(&format!("<rect x=\"{x}\" y=\"{}\" width=\"18\" height=\"10\" fill=\"#9fc1e6\"/>\n", ly - 9.0)),
            _ => s.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"3.5\" fill=\"{}\"/>\n", x + 9.0, ly - 4.0, if i == 2 { "#333" } else { "#dd6b20" })),
        }
        s.push_str(&format!("<text x=\"{}\" y=\"{ly}\" fill=\"#333\">{t}</text>\n", x + 24.0));
        x += 28.0 + t.len() as f64 * 6.4;
    }
    if !y_label.is_empty() {
        s.push_str(&format!("<text x=\"{L}\" y=\"{}\" fill=\"#555\">{}</text>\n", T - 8.0, esc(y_label)));
    }
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_waterfall_names_every_cause_and_both_ends() {
        let s = waterfall("Peak of Infected", "old", 100.0, &[("New lookup".into(), -12.0), ("Contact rate".into(), 5.0)], "new", 93.0);
        for w in ["New lookup", "Contact rate", "old", "new", "-12", "+5", "<svg", "</svg>"] {
            assert!(s.contains(w), "{w}");
        }
    }

    #[test]
    fn a_fan_draws_bands_median_actuals_and_the_cut() {
        let times: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let b = Band { share: 0.9, lo: times.iter().map(|t| t - 1.0).collect(), hi: times.iter().map(|t| t + 1.0).collect() };
        let s = fan("Fan", "units", &times, &[b], &times, &[(2.0, 2.5), (8.0, 7.0)], 5.0);
        assert!(s.contains("<polygon") && s.contains("<polyline") && s.contains(">cut<") && s.matches("<circle").count() >= 4);
    }
}

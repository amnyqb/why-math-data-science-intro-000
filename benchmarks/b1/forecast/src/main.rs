//! F2 for B1: AutoETS (non-seasonal, from the `augurs` crate) on each weekly series, rolling origin.
//!
//!   b1-forecast data/m5_store_department_weekly.csv OUT.csv [first_origin=225] [step=4] [horizon=4]
//!
//! For every series and origin t it fits on the weeks before t only and writes the point forecast and the
//! interval limits for horizon 1..h. `augurs` returns one symmetric interval per call, so the nine M5
//! quantiles are read from four calls: level 0.5 (0.25 and 0.75), 0.67 (0.165 and 0.835), 0.95 (0.025 and
//! 0.975), 0.99 (0.005 and 0.995); the median is the point forecast. Seasonal models are not implemented
//! in this version of the crate, so there is no seasonal component.

use augurs::ets::AutoETS;
use augurs::prelude::*;
use std::collections::BTreeMap;
use std::io::Write;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (input, output) = (&a[1], &a[2]);
    let first: usize = a.get(3).and_then(|x| x.parse().ok()).unwrap_or(225);
    let step: usize = a.get(4).and_then(|x| x.parse().ok()).unwrap_or(4);
    let h: usize = a.get(5).and_then(|x| x.parse().ok()).unwrap_or(4);
    let text = std::fs::read_to_string(input).expect("input file");
    let mut series: BTreeMap<(String, String), Vec<f64>> = BTreeMap::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        series.entry((f[0].to_string(), f[1].to_string())).or_default().push(f[4].parse().unwrap());
    }
    let mut out = std::fs::File::create(output).expect("output file");
    writeln!(out, "store,dept,origin,horizon,point,q005,q025,q165,q250,q750,q835,q975,q995").unwrap();
    let levels = [0.5, 0.67, 0.95, 0.99];
    let mut failures = 0;
    for ((store, dept), y) in &series {
        let mut t = first;
        while t + h <= y.len() {
            let train = &y[..t];
            // the same scaling as the policy runs: the series' own training mean (weeks before `first`) -> 100
            let mean = y[..first].iter().sum::<f64>() / first as f64;
            let scaled: Vec<f64> = train.iter().map(|v| v * 100.0 / mean).collect();
            let model = AutoETS::new(1, "ZZN").expect("spec").fit(&scaled);
            match model {
                Ok(fit) => {
                    let mut lo = vec![[0.0; 4]; h];
                    let mut hi = vec![[0.0; 4]; h];
                    let mut point = vec![0.0; h];
                    let mut ok = true;
                    for (i, l) in levels.iter().enumerate() {
                        match fit.predict(h, *l) {
                            Ok(f) => {
                                point = f.point.clone();
                                let iv = f.intervals.expect("intervals");
                                for k in 0..h {
                                    lo[k][i] = iv.lower[k];
                                    hi[k][i] = iv.upper[k];
                                }
                            }
                            Err(_) => ok = false,
                        }
                    }
                    if !ok {
                        failures += 1;
                        t += step;
                        continue;
                    }
                    for k in 0..h {
                        writeln!(out, "{store},{dept},{t},{},{},{},{},{},{},{},{},{},{}", k + 1, point[k], lo[k][3], lo[k][2], lo[k][1], lo[k][0], hi[k][0], hi[k][1], hi[k][2], hi[k][3]).unwrap();
                    }
                }
                Err(_) => failures += 1,
            }
            t += step;
        }
    }
    eprintln!("{} series, {} failed fits", series.len(), failures);
}

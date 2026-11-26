//! Eigenvalues of a small real matrix (Hessenberg reduction, then the shifted QR algorithm; after
//! Numerical Recipes `elmhes` and `hqr`). Used for eigenvalue analysis of a linearised model.

/// Eigenvalues as (real, imaginary) pairs; the matrix is given row by row.
pub fn eigenvalues(m: &[Vec<f64>]) -> Result<Vec<(f64, f64)>, String> {
    let n = m.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    // 1-based working copy
    let mut a = vec![vec![0.0f64; n + 1]; n + 1];
    for i in 0..n {
        for j in 0..n {
            a[i + 1][j + 1] = m[i][j];
        }
    }
    // reduction to Hessenberg form by elimination
    for mm in 2..n {
        let mut x = 0.0f64;
        let mut i = mm;
        for j in mm..=n {
            if a[j][mm - 1].abs() > x.abs() {
                x = a[j][mm - 1];
                i = j;
            }
        }
        if i != mm {
            for j in (mm - 1)..=n {
                let t = a[i][j];
                a[i][j] = a[mm][j];
                a[mm][j] = t;
            }
            for j in 1..=n {
                let t = a[j][i];
                a[j][i] = a[j][mm];
                a[j][mm] = t;
            }
        }
        if x != 0.0 {
            for i in (mm + 1)..=n {
                let mut y = a[i][mm - 1];
                if y != 0.0 {
                    y /= x;
                    a[i][mm - 1] = y;
                    for j in mm..=n {
                        let v = a[mm][j];
                        a[i][j] -= y * v;
                    }
                    for j in 1..=n {
                        let v = a[j][i];
                        a[j][mm] += y * v;
                    }
                }
            }
        }
    }
    for i in 3..=n {
        for j in 1..(i - 1) {
            a[i][j] = 0.0;
        }
    }
    // QR iterations
    let mut wr = vec![0.0; n + 1];
    let mut wi = vec![0.0; n + 1];
    let mut anorm = 0.0f64;
    for i in 1..=n {
        for j in (if i > 1 { i - 1 } else { 1 })..=n {
            anorm += a[i][j].abs();
        }
    }
    let sign = |a: f64, b: f64| if b >= 0.0 { a.abs() } else { -a.abs() };
    let mut nn = n;
    let mut t = 0.0f64;
    while nn >= 1 {
        let mut its = 0;
        loop {
            let mut l = nn;
            while l >= 2 {
                let s0 = a[l - 1][l - 1].abs() + a[l][l].abs();
                let s = if s0 == 0.0 { anorm } else { s0 };
                if a[l][l - 1].abs() + s == s {
                    a[l][l - 1] = 0.0;
                    break;
                }
                l -= 1;
            }
            let mut x = a[nn][nn];
            if l == nn {
                wr[nn] = x + t;
                wi[nn] = 0.0;
                nn -= 1;
            } else {
                let mut y = a[nn - 1][nn - 1];
                let mut w = a[nn][nn - 1] * a[nn - 1][nn];
                if l == nn - 1 {
                    let p = 0.5 * (y - x);
                    let q = p * p + w;
                    let mut z = q.abs().sqrt();
                    x += t;
                    if q >= 0.0 {
                        z = p + sign(z, p);
                        wr[nn - 1] = x + z;
                        wr[nn] = x + z;
                        if z != 0.0 {
                            wr[nn] = x - w / z;
                        }
                        wi[nn - 1] = 0.0;
                        wi[nn] = 0.0;
                    } else {
                        wr[nn - 1] = x + p;
                        wr[nn] = x + p;
                        wi[nn] = z;
                        wi[nn - 1] = -z;
                    }
                    nn -= 2;
                } else {
                    if its == 60 {
                        return Err("the eigenvalue iteration did not converge".into());
                    }
                    if its == 10 || its == 20 {
                        t += x;
                        for i in 1..=nn {
                            a[i][i] -= x;
                        }
                        let s = a[nn][nn - 1].abs() + a[nn - 1][nn - 2].abs();
                        x = 0.75 * s;
                        y = x;
                        w = -0.4375 * s * s;
                    }
                    its += 1;
                    let (mut p, mut q, mut r, mut z);
                    let mut mm = nn - 2;
                    loop {
                        z = a[mm][mm];
                        r = x - z;
                        let s = y - z;
                        p = (r * s - w) / a[mm + 1][mm] + a[mm][mm + 1];
                        q = a[mm + 1][mm + 1] - z - r - s;
                        r = a[mm + 2][mm + 1];
                        let s = p.abs() + q.abs() + r.abs();
                        p /= s;
                        q /= s;
                        r /= s;
                        if mm == l {
                            break;
                        }
                        let u = a[mm][mm - 1].abs() * (q.abs() + r.abs());
                        let v = p.abs() * (a[mm - 1][mm - 1].abs() + z.abs() + a[mm + 1][mm + 1].abs());
                        if u + v == v {
                            break;
                        }
                        mm -= 1;
                    }
                    for i in (mm + 2)..=nn {
                        a[i][i - 2] = 0.0;
                        if i != mm + 2 {
                            a[i][i - 3] = 0.0;
                        }
                    }
                    for k in mm..nn {
                        if k != mm {
                            p = a[k][k - 1];
                            q = a[k + 1][k - 1];
                            r = if k != nn - 1 { a[k + 2][k - 1] } else { 0.0 };
                            x = p.abs() + q.abs() + r.abs();
                            if x != 0.0 {
                                p /= x;
                                q /= x;
                                r /= x;
                            }
                        }
                        let s = sign((p * p + q * q + r * r).sqrt(), p);
                        if s != 0.0 {
                            if k == mm {
                                if l != mm {
                                    a[k][k - 1] = -a[k][k - 1];
                                }
                            } else {
                                a[k][k - 1] = -s * x;
                            }
                            p += s;
                            x = p / s;
                            y = q / s;
                            z = r / s;
                            q /= p;
                            r /= p;
                            for j in k..=nn {
                                p = a[k][j] + q * a[k + 1][j];
                                if k != nn - 1 {
                                    p += r * a[k + 2][j];
                                    a[k + 2][j] -= p * z;
                                }
                                a[k + 1][j] -= p * y;
                                a[k][j] -= p * x;
                            }
                            let mmin = if nn < k + 3 { nn } else { k + 3 };
                            for i in l..=mmin {
                                p = x * a[i][k] + y * a[i][k + 1];
                                if k != nn - 1 {
                                    p += z * a[i][k + 2];
                                    a[i][k + 2] -= p * r;
                                }
                                a[i][k + 1] -= p * q;
                                a[i][k] -= p;
                            }
                        }
                    }
                }
            }
            if !(l + 1 < nn) {
                break;
            }
        }
    }
    Ok((1..=n).map(|i| (wr[i], wi[i])).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut v: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        v
    }

    #[test]
    fn known_spectra() {
        let e = sorted(eigenvalues(&[vec![2.0, 0.0], vec![0.0, 3.0]]).unwrap());
        assert_eq!(e, vec![(2.0, 0.0), (3.0, 0.0)]);
        let e = sorted(eigenvalues(&[vec![0.0, -1.0], vec![1.0, 0.0]]).unwrap());
        assert_eq!(e, vec![(0.0, -1.0), (0.0, 1.0)]);
        // companion matrix of (x-1)(x-2)(x-3)(x+4) = x^4 - 2x^3 - 13x^2 + 38x - 24
        let c = vec![vec![0.0, 0.0, 0.0, 24.0], vec![1.0, 0.0, 0.0, -38.0], vec![0.0, 1.0, 0.0, 13.0], vec![0.0, 0.0, 1.0, 2.0]];
        let e = sorted(eigenvalues(&c).unwrap());
        for (got, want) in e.iter().zip([-4.0, 1.0, 2.0, 3.0]) {
            assert!((got.0 - want).abs() < 1e-9 && got.1.abs() < 1e-9, "{e:?}");
        }
        // damped oscillator x'' + 0.2 x' + x = 0: -0.1 +- i sqrt(0.99)
        let e = sorted(eigenvalues(&[vec![0.0, 1.0], vec![-1.0, -0.2]]).unwrap());
        assert!((e[0].0 + 0.1).abs() < 1e-12 && (e[0].1 + 0.99f64.sqrt()).abs() < 1e-12, "{e:?}");
    }

    #[test]
    fn trace_and_determinant_of_a_dense_matrix_are_kept() {
        let n = 7;
        let mut s = 12345u64;
        let mut r = || { s ^= s << 13; s ^= s >> 7; s ^= s << 17; (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
        let m: Vec<Vec<f64>> = (0..n).map(|_| (0..n).map(|_| r()).collect()).collect();
        let e = eigenvalues(&m).unwrap();
        let tr: f64 = (0..n).map(|i| m[i][i]).sum();
        assert!((e.iter().map(|x| x.0).sum::<f64>() - tr).abs() < 1e-9);
        // sum of squares of eigenvalues = trace(M^2)
        let tr2: f64 = (0..n).map(|i| (0..n).map(|j| m[i][j] * m[j][i]).sum::<f64>()).sum();
        let s2: f64 = e.iter().map(|x| x.0 * x.0 - x.1 * x.1).sum();
        assert!((s2 - tr2).abs() < 1e-9, "{s2} {tr2}");
    }
}

/// Eigenvalue analysis of a model linearised at a time of its run, and the elasticity of each
/// eigenvalue to each named constant (Kampmann 1996, Kampmann and Oliva 2006): `(x / λ) ∂λ/∂x`,
/// found by moving the constant by ±1% with the state held where it is. Meaningful where the
/// model is close to linear around that state; it says nothing about strongly non-linear phases.
pub fn eigen_analysis(model: &crate::model::Model, at: f64, params: &[String]) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let base = crate::sim::jacobian_at(model, at, None)?;
    let c = crate::sim::compile(model).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("; "))?;
    let names = crate::sim::state_names(&c);
    let lam = eigenvalues(&base.jac)?;
    let mut rows = Vec::new();
    for p in params {
        let key = crate::expr::normalize(p);
        let v = model.by_name(&key).ok_or_else(|| format!("“{p}” is not a variable of the model"))?;
        let x: f64 = v.equation.trim().parse().map_err(|_| format!("“{p}” is not a plain number, so it cannot be moved"))?;
        if x == 0.0 {
            return Err(format!("“{p}” is zero: an elasticity is not defined"));
        }
        let shifted = |f: f64| -> Result<Vec<(f64, f64)>, String> {
            let mut m = model.clone();
            m.apply_change(p, &format!("{}", x * f));
            eigenvalues(&crate::sim::jacobian_at(&m, at, Some(base.y.clone()))?.jac)
        };
        let (up, dn) = (shifted(1.01)?, shifted(0.99)?);
        let mut el = Vec::new();
        for l in &lam {
            let near = |set: &Vec<(f64, f64)>| *set.iter().min_by(|a, b| ((a.0 - l.0).hypot(a.1 - l.1)).total_cmp(&((b.0 - l.0).hypot(b.1 - l.1)))).unwrap();
            let (u, d) = (near(&up), near(&dn));
            let dl = ((u.0 - d.0) / 0.02, (u.1 - d.1) / 0.02); // d lambda / d ln x
            // divide by lambda (complex)
            let den = l.0 * l.0 + l.1 * l.1;
            let e = if den == 0.0 { (f64::NAN, f64::NAN) } else { ((dl.0 * l.0 + dl.1 * l.1) / den, (dl.1 * l.0 - dl.0 * l.1) / den) };
            el.push(json!({"re": e.0, "im": e.1, "d_re_per_ln": dl.0, "d_im_per_ln": dl.1}));
        }
        rows.push(json!({"param": p, "value": x, "elasticities": el}));
    }
    let ev: Vec<_> = lam.iter().map(|l| json!({"re": l.0, "im": l.1, "period": if l.1.abs() > 1e-12 { json!(2.0 * std::f64::consts::PI / l.1.abs()) } else { json!(null) }, "time_constant": if l.0 != 0.0 { json!(1.0 / l.0.abs()) } else { json!(null) }})).collect();
    Ok(json!({"time": base.time, "states": names, "eigenvalues": ev, "elasticity": rows}))
}

#[cfg(test)]
mod model_tests {
    use super::*;
    use crate::model::Model;
    use crate::ops::{Op, Session};

    #[test]
    fn a_linear_oscillator_has_the_analytic_eigenvalues_and_elasticities() {
        let ops: Vec<Op> = serde_json::from_str(r#"[
          {"op":"set_setting","key":"final_time","value":"10"},{"op":"set_setting","key":"time_step","value":"0.0625"},
          {"op":"add_var","name":"K","kind":"constant","equation":"4"},
          {"op":"add_var","name":"C","kind":"constant","equation":"0.4"},
          {"op":"add_var","name":"X","kind":"stock","equation":"1"},
          {"op":"add_var","name":"V","kind":"stock","equation":"0"},
          {"op":"add_var","name":"dX","kind":"flow","equation":"V","to":"X"},
          {"op":"add_var","name":"dV","kind":"flow","equation":"-K * X - C * V","to":"V"}]"#).unwrap();
        let mut s = Session::new(Model::default());
        s.apply("t", &ops).unwrap();
        let r = eigen_analysis(&s.model, 0.0, &["K".into(), "C".into()]).unwrap();
        // lambda = -C/2 +- i sqrt(K - C^2/4)
        let w = (4.0f64 - 0.04).sqrt();
        let ev = r["eigenvalues"].as_array().unwrap();
        assert!(ev.iter().all(|e| (e["re"].as_f64().unwrap() + 0.2).abs() < 1e-6 && (e["im"].as_f64().unwrap().abs() - w).abs() < 1e-6), "{ev:?}");
        // d lambda / dK = -1 / (2 lambda + C); elasticity = K / lambda * that
        let upper = ev.iter().position(|e| e["im"].as_f64().unwrap() > 0.0).unwrap();
        let lam = (-0.2, w);
        let (dre, dim) = {
            // -1 / (2 lam + C) with complex lam
            let (a, b) = (2.0 * lam.0 + 0.4, 2.0 * lam.1);
            let d = a * a + b * b;
            (-a / d, b / d)
        };
        let den = lam.0 * lam.0 + lam.1 * lam.1;
        let want = (4.0 * (dre * lam.0 + dim * lam.1) / den, 4.0 * (dim * lam.0 - dre * lam.1) / den);
        let got = &r["elasticity"][0]["elasticities"][upper];
        assert!((got["re"].as_f64().unwrap() - want.0).abs() < 1e-4 && (got["im"].as_f64().unwrap() - want.1).abs() < 1e-4, "{got:?} vs {want:?}");
    }
}

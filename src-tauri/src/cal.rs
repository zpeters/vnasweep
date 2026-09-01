//! One-port 3-term OSL error model. See
//! `docs/superpowers/specs/2026-09-01-vnasweep-v1-design.md` §5.
//!
//! Model:  Γm = e00 + (e10·e01 · Γa) / (1 - e11·Γa)
//!   e00     = directivity          (equals the Load measurement, since Γ_load = 0)
//!   e11     = source match
//!   e10·e01 = reflection tracking  (stored as one product)
//! Standards are ideal for v1: Γ_open = +1, Γ_short = -1, Γ_load = 0.

#![allow(dead_code)]

use crate::complex::Complex;
use crate::model::{ErrorTerms, RawTrace, Standards};

fn cx(p: &crate::model::RawPoint) -> Complex {
    Complex::new(p.re, p.im)
}

pub fn solve_osl(std: &Standards) -> Result<ErrorTerms, String> {
    let n = std.open.points.len();
    if n == 0 {
        return Err("calibration sweeps are empty".into());
    }
    if std.short.points.len() != n || std.load.points.len() != n {
        return Err(format!(
            "standard sweeps have different point counts: open {}, short {}, load {}",
            n,
            std.short.points.len(),
            std.load.points.len()
        ));
    }

    let one = Complex::new(1.0, 0.0);
    let mut e00 = Vec::with_capacity(n);
    let mut e11 = Vec::with_capacity(n);
    let mut e10e01 = Vec::with_capacity(n);

    for i in 0..n {
        let g_open = cx(&std.open.points[i]);
        let g_short = cx(&std.short.points[i]);
        let g_load = cx(&std.load.points[i]);

        let d = g_load; // e00
        let mo = g_open - d;
        let ms = g_short - d;

        let denom = mo - ms;
        if denom.abs() < 1e-12 {
            return Err(format!(
                "point {i}: open and short measurements are indistinguishable — bad standards or a loose connection?"
            ));
        }

        let b = (mo + ms) / denom; // e11
        let a = mo * (one - b); // e10·e01

        if !d.is_finite() || !b.is_finite() || !a.is_finite() {
            return Err(format!("point {i}: solved a non-finite error term"));
        }

        e00.push([d.re, d.im]);
        e11.push([b.re, b.im]);
        e10e01.push([a.re, a.im]);
    }

    Ok(ErrorTerms { e00, e11, e10e01 })
}

pub fn apply(raw: &RawTrace, terms: &ErrorTerms) -> Result<Vec<Complex>, String> {
    let n = raw.points.len();
    if terms.e00.len() != n || terms.e11.len() != n || terms.e10e01.len() != n {
        return Err(format!(
            "calibration is internally inconsistent or has {} points but the sweep has {n}",
            terms.e00.len()
        ));
    }

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let gm = cx(&raw.points[i]);
        let e00 = Complex::new(terms.e00[i][0], terms.e00[i][1]);
        let e11 = Complex::new(terms.e11[i][0], terms.e11[i][1]);
        let a = Complex::new(terms.e10e01[i][0], terms.e10e01[i][1]);

        // Γa = (Γm - e00) / (e10·e01 + e11·(Γm - e00))
        let d = gm - e00;
        out.push(d / (a + e11 * d));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complex::Complex;
    use crate::model::{RawPoint, RawTrace, Standards};

    // Forward error model: Γm = e00 + a·Γa / (1 - e11·Γa)
    fn measured(e00: Complex, e11: Complex, a: Complex, ga: Complex) -> Complex {
        let one = Complex::new(1.0, 0.0);
        e00 + (a * ga) / (one - e11 * ga)
    }

    fn one_point(re: f64, im: f64) -> RawTrace {
        RawTrace {
            points: vec![RawPoint {
                freq_hz: 1_000_000,
                re,
                im,
            }],
        }
    }

    const E00: Complex = Complex::new(0.05, -0.02);
    const E11: Complex = Complex::new(-0.10, 0.08);
    const A: Complex = Complex::new(0.90, 0.03);

    #[test]
    fn solve_recovers_fabricated_terms() {
        let mo = measured(E00, E11, A, Complex::new(1.0, 0.0));
        let ms = measured(E00, E11, A, Complex::new(-1.0, 0.0));
        let ml = measured(E00, E11, A, Complex::new(0.0, 0.0));
        let std = Standards {
            open: one_point(mo.re, mo.im),
            short: one_point(ms.re, ms.im),
            load: one_point(ml.re, ml.im),
        };
        let t = solve_osl(&std).unwrap();
        assert!((t.e00[0][0] - E00.re).abs() < 1e-9);
        assert!((t.e00[0][1] - E00.im).abs() < 1e-9);
        assert!((t.e11[0][0] - E11.re).abs() < 1e-9);
        assert!((t.e11[0][1] - E11.im).abs() < 1e-9);
        assert!((t.e10e01[0][0] - A.re).abs() < 1e-9);
        assert!((t.e10e01[0][1] - A.im).abs() < 1e-9);
    }

    #[test]
    fn apply_recovers_known_dut() {
        let ga = Complex::new(0.30, 0.20);
        let gm = measured(E00, E11, A, ga);
        let terms = ErrorTerms {
            e00: vec![[E00.re, E00.im]],
            e11: vec![[E11.re, E11.im]],
            e10e01: vec![[A.re, A.im]],
        };
        let corrected = apply(&one_point(gm.re, gm.im), &terms).unwrap();
        assert!((corrected[0].re - ga.re).abs() < 1e-9);
        assert!((corrected[0].im - ga.im).abs() < 1e-9);
    }

    #[test]
    fn solve_rejects_identical_open_and_short() {
        let m = measured(E00, E11, A, Complex::new(0.0, 0.0));
        let std = Standards {
            open: one_point(m.re, m.im),
            short: one_point(m.re, m.im),
            load: one_point(m.re, m.im),
        };
        assert!(solve_osl(&std).is_err());
    }

    #[test]
    fn solve_rejects_mismatched_lengths() {
        let std = Standards {
            open: RawTrace {
                points: vec![
                    RawPoint {
                        freq_hz: 1,
                        re: 0.9,
                        im: 0.0,
                    },
                    RawPoint {
                        freq_hz: 2,
                        re: 0.9,
                        im: 0.0,
                    },
                ],
            },
            short: one_point(-0.9, 0.0),
            load: one_point(0.0, 0.0),
        };
        assert!(solve_osl(&std).is_err());
    }

    #[test]
    fn apply_rejects_length_mismatch() {
        let terms = ErrorTerms {
            e00: vec![[0.0, 0.0]],
            e11: vec![[0.0, 0.0]],
            e10e01: vec![[1.0, 0.0]],
        };
        let raw = RawTrace {
            points: vec![
                RawPoint {
                    freq_hz: 1,
                    re: 0.1,
                    im: 0.0,
                },
                RawPoint {
                    freq_hz: 2,
                    re: 0.1,
                    im: 0.0,
                },
            ],
        };
        assert!(apply(&raw, &terms).is_err());
    }

    #[test]
    fn apply_rejects_inconsistent_error_terms() {
        let terms = ErrorTerms {
            e00: vec![[0.0, 0.0], [0.0, 0.0]],
            e11: vec![[0.0, 0.0]], // short
            e10e01: vec![[1.0, 0.0], [1.0, 0.0]],
        };
        let raw = RawTrace {
            points: vec![
                RawPoint {
                    freq_hz: 1,
                    re: 0.1,
                    im: 0.0,
                },
                RawPoint {
                    freq_hz: 2,
                    re: 0.1,
                    im: 0.0,
                },
            ],
        };
        assert!(apply(&raw, &terms).is_err());
    }
}

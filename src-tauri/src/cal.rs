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
use crate::model::{CalHealth, ErrorTerms, MagStats, RawTrace, Standards};

fn cx(p: &crate::model::RawPoint) -> Complex {
    Complex::new(p.re, p.im)
}

fn mag_stats(points: &[crate::model::RawPoint]) -> MagStats {
    if points.is_empty() {
        return MagStats {
            mean: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }
    let mags: Vec<f64> = points.iter().map(|p| cx(p).abs()).collect();
    let sum: f64 = mags.iter().sum();
    MagStats {
        mean: sum / mags.len() as f64,
        min: mags.iter().cloned().fold(f64::INFINITY, f64::min),
        max: mags.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    }
}

fn mean_abs(terms: &[[f64; 2]]) -> f64 {
    if terms.is_empty() {
        return 0.0;
    }
    let sum: f64 = terms.iter().map(|t| Complex::new(t[0], t[1]).abs()).sum();
    sum / terms.len() as f64
}

/// Sanity-check a stored calibration against what ideal Open/Short/Load
/// captures should look like. Purely arithmetic on the stored data.
pub fn health(std: &Standards, terms: &ErrorTerms) -> CalHealth {
    let open_mag = mag_stats(&std.open.points);
    let short_mag = mag_stats(&std.short.points);
    let load_mag = mag_stats(&std.load.points);

    // Mean unsigned phase difference between Γ_open and Γ_short, in degrees.
    let mut phase_sum = 0.0;
    let mut phase_count = 0.0;
    for i in 0..std.open.points.len().min(std.short.points.len()) {
        let o = cx(&std.open.points[i]);
        let s = cx(&std.short.points[i]);
        if o.abs() > 1e-6 && s.abs() > 1e-6 {
            // angle of s relative to o, folded into [0, 180]
            let d = (s * o.conj()).arg().abs().to_degrees();
            phase_sum += d;
            phase_count += 1.0;
        }
    }
    let open_short_phase_deg = if phase_count > 0.0 {
        phase_sum / phase_count
    } else {
        0.0
    };

    let directivity_mag = mean_abs(&terms.e00);
    let source_match_mag = mean_abs(&terms.e11);
    let tracking_mag = mean_abs(&terms.e10e01);

    let mut warnings = Vec::new();
    if load_mag.mean > 0.2 {
        warnings.push(format!(
            "Load |Γ| averages {:.2} — a real 50 Ω load reads near 0. The Load standard may not have been connected.",
            load_mag.mean
        ));
    }
    if open_mag.mean < 0.7 {
        warnings.push(format!(
            "Open |Γ| averages {:.2} — expected near 1. The port may have been shorted or loaded during the Open step.",
            open_mag.mean
        ));
    }
    if short_mag.mean < 0.7 {
        warnings.push(format!(
            "Short |Γ| averages {:.2} — expected near 1. The port may have been open or loaded during the Short step.",
            short_mag.mean
        ));
    }
    if open_short_phase_deg < 120.0 {
        warnings.push(format!(
            "Open and Short are only {open_short_phase_deg:.0}° apart — expected ~180°. Standards may be swapped or a connection was loose."
        ));
    }
    if tracking_mag < 0.1 {
        warnings.push(format!(
            "Reflection tracking |e10·e01| is {tracking_mag:.3} — near zero means the calibration is degenerate."
        ));
    }
    if directivity_mag > 0.3 {
        warnings.push(format!(
            "Directivity |e00| is {directivity_mag:.2} — large values usually mean a bad Load capture."
        ));
    }

    CalHealth {
        open_mag,
        short_mag,
        load_mag,
        open_short_phase_deg,
        directivity_mag,
        source_match_mag,
        tracking_mag,
        warnings,
    }
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
        let g = d / (a + e11 * d);
        if !g.is_finite() {
            return Err(format!(
                "point {i}: correction produced a non-finite reflection coefficient — the calibration may be bad"
            ));
        }
        out.push(g);
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

    fn multi_point(re: f64, im: f64, n: usize) -> RawTrace {
        RawTrace {
            points: (0..n)
                .map(|i| RawPoint {
                    freq_hz: 1_000_000 * (i as u64 + 1),
                    re,
                    im,
                })
                .collect(),
        }
    }

    #[test]
    fn health_flags_a_clean_cal_as_ok() {
        let std = Standards {
            open: multi_point(0.98, 0.05, 5),
            short: multi_point(-0.97, -0.03, 5),
            load: multi_point(0.02, 0.01, 5),
        };
        let terms = solve_osl(&std).unwrap();
        let h = health(&std, &terms);
        assert!(
            h.warnings.is_empty(),
            "unexpected warnings: {:?}",
            h.warnings
        );
        assert!((h.open_mag.mean - 0.9812).abs() < 1e-3);
        assert!(h.load_mag.mean < 0.05);
        assert!(h.open_short_phase_deg > 150.0);
    }

    #[test]
    fn health_flags_a_load_that_was_not_connected() {
        let std = Standards {
            open: multi_point(0.98, 0.0, 4),
            short: multi_point(-0.98, 0.0, 4),
            load: multi_point(0.6, 0.0, 4), // way too reflective for a 50 Ω load
        };
        let terms = solve_osl(&std).unwrap();
        let h = health(&std, &terms);
        assert!(h.warnings.iter().any(|w| w.contains("Load")));
    }

    #[test]
    fn health_flags_swapped_standards() {
        // open and short both look like an open ⇒ ~0° apart
        let std = Standards {
            open: multi_point(0.95, 0.0, 4),
            short: multi_point(0.95, 0.01, 4),
            load: multi_point(0.01, 0.0, 4),
        };
        // solve would reject (open≈short); build terms directly for the health call
        let terms = ErrorTerms {
            e00: vec![[0.0, 0.0]; 4],
            e11: vec![[0.0, 0.0]; 4],
            e10e01: vec![[1.0, 0.0]; 4],
        };
        let h = health(&std, &terms);
        assert!(h.warnings.iter().any(|w| w.contains("apart")));
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
    fn apply_rejects_non_finite_correction() {
        // e10e01 = 0 and e11 = 0 ⇒ a + e11*d == 0 for any d ⇒ div yields NaN/Inf.
        let terms = ErrorTerms {
            e00: vec![[0.05, -0.02]],
            e11: vec![[0.0, 0.0]],
            e10e01: vec![[0.0, 0.0]],
        };
        // re != e00.re so d != 0.
        let raw = one_point(0.5, 0.0);
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

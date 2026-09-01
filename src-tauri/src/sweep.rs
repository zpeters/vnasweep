//! SWR, impedance, and trace-level statistics. Pure. See spec §5.3, §5.5.

#![allow(dead_code)]

use crate::complex::Complex;
use crate::model::{Band, RawTrace, SwrPoint, TracePoint};

const Z0: f64 = 50.0;

pub fn swr(gamma: Complex) -> f64 {
    let g = gamma.abs();
    if g >= 1.0 {
        return 999.0;
    }
    (1.0 + g) / (1.0 - g)
}

pub fn impedance(gamma: Complex) -> (f64, f64) {
    // Z = Z0 · (1 + Γ) / (1 - Γ)
    let one = Complex::new(1.0, 0.0);
    let denom = one - gamma;

    // Check if denominator is near-zero (open circuit)
    if denom.abs() < f64::EPSILON {
        return (f64::INFINITY, f64::INFINITY);
    }

    let z = Complex::new(Z0, 0.0) * ((one + gamma) / denom);
    (z.re, z.im)
}

pub fn return_loss_db(gamma: Complex) -> f64 {
    let g = gamma.abs();
    if g <= 0.0 {
        return f64::INFINITY;
    }
    -20.0 * g.log10()
}

pub fn build_trace(raw: &RawTrace, corrected: &[Complex]) -> Vec<TracePoint> {
    debug_assert_eq!(raw.points.len(), corrected.len());
    raw.points
        .iter()
        .zip(corrected)
        .map(|(p, &g)| {
            let (r, x) = impedance(g);
            TracePoint {
                freq_hz: p.freq_hz,
                gamma_re: g.re,
                gamma_im: g.im,
                swr: swr(g),
                r,
                x,
                return_loss_db: return_loss_db(g),
            }
        })
        .collect()
}

pub fn min_swr(trace: &[TracePoint]) -> SwrPoint {
    trace
        .iter()
        .min_by(|a, b| a.swr.partial_cmp(&b.swr).unwrap_or(std::cmp::Ordering::Equal))
        .map(|p| SwrPoint { freq_hz: p.freq_hz, swr: p.swr })
        .unwrap_or(SwrPoint { freq_hz: 0, swr: 999.0 })
}

pub fn swr_bandwidth(trace: &[TracePoint], threshold: f64) -> Option<Band> {
    if trace.is_empty() {
        return None;
    }
    let mi = trace
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.swr.partial_cmp(&b.swr).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)?;
    if trace[mi].swr > threshold {
        return None;
    }

    // Linear interpolation of the frequency where swr crosses `threshold`
    // between samples `lo` (below threshold) and `hi` (above threshold).
    let cross = |lo: &TracePoint, hi: &TracePoint| -> u64 {
        let span_swr = hi.swr - lo.swr;
        if span_swr.abs() < f64::EPSILON {
            return lo.freq_hz;
        }
        let frac = (threshold - lo.swr) / span_swr;
        let f = lo.freq_hz as f64 + frac * (hi.freq_hz as f64 - lo.freq_hz as f64);
        f.round() as u64
    };

    // Walk left from the minimum.
    let mut lo_hz = trace[0].freq_hz;
    for j in (0..mi).rev() {
        if trace[j].swr > threshold {
            lo_hz = cross(&trace[j + 1], &trace[j]);
            break;
        }
    }

    // Walk right from the minimum.
    let mut hi_hz = trace[trace.len() - 1].freq_hz;
    for j in (mi + 1)..trace.len() {
        if trace[j].swr > threshold {
            hi_hz = cross(&trace[j - 1], &trace[j]);
            break;
        }
    }

    Some(Band { lo_hz, hi_hz, threshold })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complex::Complex;
    use crate::model::TracePoint;

    fn tp(freq_hz: u64, swr: f64) -> TracePoint {
        TracePoint {
            freq_hz,
            gamma_re: 0.0,
            gamma_im: 0.0,
            swr,
            r: 0.0,
            x: 0.0,
            return_loss_db: 0.0,
        }
    }

    #[test]
    fn swr_values() {
        assert!((swr(Complex::new(0.0, 0.0)) - 1.0).abs() < 1e-12);
        // |Γ| = 0.5 -> SWR = 1.5/0.5 = 3.0
        assert!((swr(Complex::new(0.5, 0.0)) - 3.0).abs() < 1e-12);
        assert_eq!(swr(Complex::new(1.0, 0.0)), 999.0);
        assert_eq!(swr(Complex::new(2.0, 0.0)), 999.0);
    }

    #[test]
    fn impedance_of_matched_and_open() {
        let (r, x) = impedance(Complex::new(0.0, 0.0));
        assert!((r - 50.0).abs() < 1e-9 && x.abs() < 1e-9);
        // Γ = +1 -> open -> R huge, but must not panic
        let (r, _x) = impedance(Complex::new(1.0, 0.0));
        assert!(r.is_infinite() || r > 1e6);
    }

    #[test]
    fn return_loss() {
        // |Γ| = 0.1 -> RL = -20 log10(0.1) = 20 dB
        assert!((return_loss_db(Complex::new(0.1, 0.0)) - 20.0).abs() < 1e-9);
        assert!(return_loss_db(Complex::new(0.0, 0.0)).is_infinite());
    }

    #[test]
    fn min_swr_picks_the_dip() {
        let t = [tp(1_000, 3.0), tp(2_000, 1.2), tp(3_000, 2.5)];
        let m = min_swr(&t);
        assert_eq!(m.freq_hz, 2_000);
        assert!((m.swr - 1.2).abs() < 1e-12);
    }

    #[test]
    fn min_swr_empty() {
        let m = min_swr(&[]);
        assert_eq!(m.freq_hz, 0);
        assert_eq!(m.swr, 999.0);
    }

    #[test]
    fn bandwidth_interpolates_edges() {
        // swr crosses 2.0 between 1000-2000 (3->1) and between 2000-3000 (1->3).
        let t = [tp(1_000, 3.0), tp(2_000, 1.0), tp(3_000, 3.0)];
        let b = swr_bandwidth(&t, 2.0).unwrap();
        // left: 3 + (1-3)*f = 2  -> f = 0.5 -> 1500 Hz. right: symmetric -> 2500 Hz.
        assert_eq!(b.lo_hz, 1_500);
        assert_eq!(b.hi_hz, 2_500);
        assert_eq!(b.threshold, 2.0);
    }

    #[test]
    fn bandwidth_none_when_never_below() {
        let t = [tp(1_000, 3.0), tp(2_000, 2.5), tp(3_000, 4.0)];
        assert!(swr_bandwidth(&t, 2.0).is_none());
    }

    #[test]
    fn bandwidth_clamps_to_trace_ends() {
        // minimum at the first point and it's already under threshold; no left crossing.
        let t = [tp(1_000, 1.2), tp(2_000, 1.5), tp(3_000, 3.0)];
        let b = swr_bandwidth(&t, 2.0).unwrap();
        assert_eq!(b.lo_hz, 1_000); // clamped to the first sample
        assert_eq!(b.hi_hz, 2_333); // interpolated 1.5 -> 3.0 crossing 2.0 at 2_333
    }
}

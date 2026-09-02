//! Data structs shared between the Rust backend and the Svelte frontend.
//! Mirrored by hand in `src/lib/types.ts`.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalMeta {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub created: String,
    pub start_hz: u64,
    pub stop_hz: u64,
    pub points: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPoint {
    pub freq_hz: u64,
    pub re: f64,
    pub im: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTrace {
    pub points: Vec<RawPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Standards {
    pub open: RawTrace,
    pub short: RawTrace,
    pub load: RawTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorTerms {
    pub e00: Vec<[f64; 2]>,
    pub e11: Vec<[f64; 2]>,
    pub e10e01: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calibration {
    pub meta: CalMeta,
    pub standards: Standards,
    pub error_terms: ErrorTerms,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracePoint {
    pub freq_hz: u64,
    pub gamma_re: f64,
    pub gamma_im: f64,
    pub swr: f64,
    pub r: f64,
    pub x: f64,
    pub return_loss_db: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwrPoint {
    pub freq_hz: u64,
    pub swr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub lo_hz: u64,
    pub hi_hz: u64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingMeta {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub created: String,
    pub cal_id: String,
    pub cal_name: String,
    pub start_hz: u64,
    pub stop_hz: u64,
    pub points: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub meta: ReadingMeta,
    pub trace: Vec<TracePoint>,
    pub min_swr: SwrPoint,
    pub bandwidth: Option<Band>,
}

/// One entry in the serial-port picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub description: String,
    /// True when the USB VID:PID matches the NanoVNA-H.
    pub is_nanovna: bool,
}

/// The sweep span the device is currently programmed for, read back from
/// its `frequencies` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepConfig {
    pub start_hz: u64,
    pub stop_hz: u64,
    pub points: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_json_round_trips() {
        let cal = Calibration {
            meta: CalMeta {
                id: "hf-1-30mhz".into(),
                name: "HF 1-30 MHz".into(),
                notes: "bench".into(),
                created: "2026-09-01T20:12:00Z".into(),
                start_hz: 1_000_000,
                stop_hz: 30_000_000,
                points: 101,
            },
            standards: Standards {
                open: RawTrace {
                    points: vec![RawPoint {
                        freq_hz: 1_000_000,
                        re: 0.98,
                        im: 0.03,
                    }],
                },
                short: RawTrace {
                    points: vec![RawPoint {
                        freq_hz: 1_000_000,
                        re: -0.97,
                        im: -0.02,
                    }],
                },
                load: RawTrace {
                    points: vec![RawPoint {
                        freq_hz: 1_000_000,
                        re: 0.01,
                        im: 0.0,
                    }],
                },
            },
            error_terms: ErrorTerms {
                e00: vec![[0.01, 0.0]],
                e11: vec![[-0.1, 0.08]],
                e10e01: vec![[0.9, 0.03]],
            },
        };
        let json = serde_json::to_string(&cal).unwrap();
        let back: Calibration = serde_json::from_str(&json).unwrap();
        assert_eq!(back.meta.id, cal.meta.id);
        assert_eq!(back.standards.open.points[0].re, 0.98);
        assert_eq!(back.error_terms.e11[0], [-0.1, 0.08]);
    }
}

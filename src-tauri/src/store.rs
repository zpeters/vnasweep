//! Flat-file persistence under `$VNASWEEP_HOME` (default `$HOME/vnasweep`).
//! `cals/<slug>.json` — full `Calibration`.
//! `readings/<slug>.s1p` — Touchstone; `readings/<slug>.json` — metadata sidecar.
//! Reading stats (`min_swr`, `bandwidth`) are derived on load, never stored.

#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::model::{CalMeta, Calibration, Reading, ReadingMeta, TracePoint};
use crate::sweep::{min_swr, swr_bandwidth};

const SWR_BANDWIDTH_THRESHOLD: f64 = 2.0;

pub fn home() -> PathBuf {
    if let Ok(p) = std::env::var("VNASWEEP_HOME") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("vnasweep")
}

fn cals_dir() -> PathBuf {
    home().join("cals")
}
fn readings_dir() -> PathBuf {
    home().join("readings")
}

fn ensure_dir(p: &Path) -> Result<(), String> {
    fs::create_dir_all(p).map_err(|e| format!("creating {}: {e}", p.display()))
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    // Derive the temp name from the full filename so siblings that share a stem
    // (foo.s1p / foo.json) don't collide on one foo.tmp. The leading '.' keeps it
    // out of json_files() (which filters on the ".json" extension; ".foo.json.tmp"
    // has extension "tmp").
    let tmp = match path.file_name().and_then(|s| s.to_str()) {
        Some(name) => path.with_file_name(format!(".{name}.tmp")),
        None => return Err(format!("bad path: {}", path.display())),
    };
    {
        let mut f =
            fs::File::create(&tmp).map_err(|e| format!("writing {}: {e}", tmp.display()))?;
        f.write_all(contents.as_bytes())
            .map_err(|e| format!("writing {}: {e}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("finalizing {}: {e}", path.display()))
}

pub fn slugify(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut s: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if s.is_empty() {
        s = "untitled".to_string();
    }
    if !taken(&s) {
        return s;
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{s}-{n}");
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

// ---- calibrations --------------------------------------------------------

pub fn list_cals() -> Result<Vec<CalMeta>, String> {
    let mut metas = Vec::new();
    for p in json_files(&cals_dir()) {
        match fs::read_to_string(&p)
            .ok()
            .and_then(|s| serde_json::from_str::<Calibration>(&s).ok())
        {
            Some(c) => metas.push(c.meta),
            None => eprintln!("store: skipping unreadable calibration {}", p.display()),
        }
    }
    Ok(metas)
}

pub fn read_cal(id: &str) -> Result<Calibration, String> {
    let p = cals_dir().join(format!("{id}.json"));
    let s = fs::read_to_string(&p).map_err(|e| format!("reading {}: {e}", p.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("parsing {}: {e}", p.display()))
}

pub fn write_cal(cal: &Calibration) -> Result<(), String> {
    let p = cals_dir().join(format!("{}.json", cal.meta.id));
    let json =
        serde_json::to_string_pretty(cal).map_err(|e| format!("serializing calibration: {e}"))?;
    write_atomic(&p, &json)
}

pub fn rename_cal(id: &str, name: &str) -> Result<CalMeta, String> {
    let mut cal = read_cal(id)?;
    cal.meta.name = name.trim().to_string();
    write_cal(&cal)?;
    Ok(cal.meta)
}

pub fn delete_cal(id: &str) -> Result<(), String> {
    let p = cals_dir().join(format!("{id}.json"));
    fs::remove_file(&p).map_err(|e| format!("deleting {}: {e}", p.display()))
}

pub fn cal_usage(id: &str) -> Result<usize, String> {
    Ok(list_readings()?
        .into_iter()
        .filter(|m| m.cal_id == id)
        .count())
}

// ---- readings ----------------------------------------------------------

pub fn list_readings() -> Result<Vec<ReadingMeta>, String> {
    let mut metas = Vec::new();
    for p in json_files(&readings_dir()) {
        match fs::read_to_string(&p)
            .ok()
            .and_then(|s| serde_json::from_str::<ReadingMeta>(&s).ok())
        {
            Some(m) => metas.push(m),
            None => eprintln!("store: skipping unreadable reading sidecar {}", p.display()),
        }
    }
    Ok(metas)
}

pub fn read_reading(id: &str) -> Result<Reading, String> {
    let side_path = readings_dir().join(format!("{id}.json"));
    let side = fs::read_to_string(&side_path)
        .map_err(|e| format!("reading {}: {e}", side_path.display()))?;
    let meta: ReadingMeta =
        serde_json::from_str(&side).map_err(|e| format!("parsing {}: {e}", side_path.display()))?;

    let s1p_path = readings_dir().join(format!("{id}.s1p"));
    let s1p = fs::read_to_string(&s1p_path)
        .map_err(|e| format!("reading {}: {e}", s1p_path.display()))?;
    let trace = parse_touchstone(&s1p)?;

    let min = min_swr(&trace);
    let band = swr_bandwidth(&trace, SWR_BANDWIDTH_THRESHOLD);
    Ok(Reading {
        meta,
        trace,
        min_swr: min,
        bandwidth: band,
    })
}

pub fn write_reading(reading: &Reading) -> Result<ReadingMeta, String> {
    if reading.meta.id.is_empty() {
        return Err("write_reading: reading has no id/slug".into());
    }
    let s1p = render_touchstone(reading);
    write_atomic(
        &readings_dir().join(format!("{}.s1p", reading.meta.id)),
        &s1p,
    )?;
    let side = serde_json::to_string_pretty(&reading.meta)
        .map_err(|e| format!("serializing sidecar: {e}"))?;
    write_atomic(
        &readings_dir().join(format!("{}.json", reading.meta.id)),
        &side,
    )?;
    Ok(reading.meta.clone())
}

pub fn rename_reading(id: &str, name: &str) -> Result<ReadingMeta, String> {
    let mut r = read_reading(id)?;
    r.meta.name = name.trim().to_string();
    // rewrite only the sidecar; the .s1p body is unchanged
    let side =
        serde_json::to_string_pretty(&r.meta).map_err(|e| format!("serializing sidecar: {e}"))?;
    write_atomic(&readings_dir().join(format!("{id}.json")), &side)?;
    Ok(r.meta)
}

pub fn delete_reading(id: &str) -> Result<(), String> {
    let a = readings_dir().join(format!("{id}.s1p"));
    let b = readings_dir().join(format!("{id}.json"));
    let _ = fs::remove_file(&a);
    fs::remove_file(&b).map_err(|e| format!("deleting {}: {e}", b.display()))
}

pub fn export_reading(id: &str, dest: &str) -> Result<(), String> {
    let src = readings_dir().join(format!("{id}.s1p"));
    fs::copy(&src, dest)
        .map(|_| ())
        .map_err(|e| format!("copying {} to {dest}: {e}", src.display()))
}

/// Write the reading's derived trace as a spreadsheet-friendly CSV.
/// Non-finite return loss (a perfect match) is left blank.
pub fn export_reading_csv(id: &str, dest: &str) -> Result<(), String> {
    let r = read_reading(id)?;
    let mut out = String::from("freq_mhz,swr,r_ohm,x_ohm,return_loss_db,gamma_re,gamma_im\n");
    for p in &r.trace {
        let rl = if p.return_loss_db.is_finite() {
            format!("{:.3}", p.return_loss_db)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{:.6},{:.4},{:.3},{:.3},{rl},{:.6},{:.6}\n",
            p.freq_hz as f64 / 1e6,
            p.swr,
            p.r,
            p.x,
            p.gamma_re,
            p.gamma_im,
        ));
    }
    fs::write(dest, out).map_err(|e| format!("writing {dest}: {e}"))
}

// ---- Touchstone -------------------------------------------------------

fn render_touchstone(r: &Reading) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "! vnasweep  {}  {}  cal: {}\n",
        r.meta.name, r.meta.created, r.meta.cal_name
    ));
    out.push_str("# MHz S RI R 50\n");
    for p in &r.trace {
        out.push_str(&format!(
            "{:.6}  {:.9}  {:.9}\n",
            p.freq_hz as f64 / 1e6,
            p.gamma_re,
            p.gamma_im
        ));
    }
    out
}

/// Parse a 1-port RI Touchstone into `TracePoint`s. `swr`/`r`/`x`/`return_loss_db`
/// are recomputed from Γ so a hand-edited file stays self-consistent.
fn parse_touchstone(text: &str) -> Result<Vec<TracePoint>, String> {
    use crate::complex::Complex;
    use crate::sweep::{impedance, return_loss_db, swr};

    let mut freq_scale = 1e6_f64; // default MHz
    let mut pts = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('!') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            let up = rest.to_uppercase();
            freq_scale = if up.contains("GHZ") {
                1e9
            } else if up.contains("KHZ") {
                1e3
            } else if up.contains(" HZ") || up.starts_with("HZ") {
                1.0
            } else {
                1e6
            };
            continue;
        }
        let mut it = line.split_whitespace();
        let f: f64 = it
            .next()
            .ok_or("touchstone: missing frequency")?
            .parse()
            .map_err(|_| "touchstone: bad frequency")?;
        let re: f64 = it
            .next()
            .ok_or("touchstone: missing real part")?
            .parse()
            .map_err(|_| "touchstone: bad real part")?;
        let im: f64 = it
            .next()
            .ok_or("touchstone: missing imag part")?
            .parse()
            .map_err(|_| "touchstone: bad imag part")?;
        let g = Complex::new(re, im);
        let (r, x) = impedance(g);
        pts.push(TracePoint {
            freq_hz: (f * freq_scale).round() as u64,
            gamma_re: re,
            gamma_im: im,
            swr: swr(g),
            r,
            x,
            return_loss_db: return_loss_db(g),
        });
    }
    if pts.is_empty() {
        return Err("touchstone: no data rows".into());
    }
    Ok(pts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    // Each test gets its own temp home via VNASWEEP_HOME. Tests run in the same
    // process, so serialize env access with a mutex.
    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct TempHome {
        _guard: std::sync::MutexGuard<'static, ()>,
        dir: std::path::PathBuf,
        prev: Option<String>,
    }
    impl TempHome {
        #[allow(unsafe_code)]
        fn new() -> Self {
            let guard = ENV_LOCK.lock().unwrap();
            let dir = std::env::temp_dir().join(format!("vnasweep-test-{}", uid()));
            std::fs::create_dir_all(&dir).unwrap();
            let prev = std::env::var("VNASWEEP_HOME").ok();
            unsafe { std::env::set_var("VNASWEEP_HOME", &dir) };
            TempHome {
                _guard: guard,
                dir,
                prev,
            }
        }
    }
    impl Drop for TempHome {
        #[allow(unsafe_code)]
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var("VNASWEEP_HOME", v) },
                None => unsafe { std::env::remove_var("VNASWEEP_HOME") },
            }
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
    fn uid() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    fn sample_cal(id: &str, name: &str) -> Calibration {
        Calibration {
            meta: CalMeta {
                id: id.into(),
                name: name.into(),
                notes: "n".into(),
                created: "2026-09-01T00:00:00Z".into(),
                start_hz: 1_000_000,
                stop_hz: 3_000_000,
                points: 3,
            },
            standards: Standards {
                open: RawTrace {
                    points: (0..3)
                        .map(|i| RawPoint {
                            freq_hz: 1_000_000 * (i + 1),
                            re: 0.9,
                            im: 0.0,
                        })
                        .collect(),
                },
                short: RawTrace {
                    points: (0..3)
                        .map(|i| RawPoint {
                            freq_hz: 1_000_000 * (i + 1),
                            re: -0.9,
                            im: 0.0,
                        })
                        .collect(),
                },
                load: RawTrace {
                    points: (0..3)
                        .map(|i| RawPoint {
                            freq_hz: 1_000_000 * (i + 1),
                            re: 0.0,
                            im: 0.0,
                        })
                        .collect(),
                },
            },
            error_terms: ErrorTerms {
                e00: vec![[0.0, 0.0]; 3],
                e11: vec![[0.0, 0.0]; 3],
                e10e01: vec![[1.0, 0.0]; 3],
            },
        }
    }

    fn sample_reading(id: &str) -> Reading {
        let trace = vec![
            TracePoint {
                freq_hz: 1_000_000,
                gamma_re: 0.5,
                gamma_im: 0.0,
                swr: 3.0,
                r: 150.0,
                x: 0.0,
                return_loss_db: 6.0,
            },
            TracePoint {
                freq_hz: 2_000_000,
                gamma_re: 0.0,
                gamma_im: 0.0,
                swr: 1.0,
                r: 50.0,
                x: 0.0,
                return_loss_db: f64::INFINITY,
            },
            TracePoint {
                freq_hz: 3_000_000,
                gamma_re: 0.5,
                gamma_im: 0.0,
                swr: 3.0,
                r: 150.0,
                x: 0.0,
                return_loss_db: 6.0,
            },
        ];
        Reading {
            meta: ReadingMeta {
                id: id.into(),
                name: "40m dipole v2".into(),
                notes: "legs -4cm".into(),
                created: "2026-09-01T00:00:00Z".into(),
                cal_id: "hf".into(),
                cal_name: "HF".into(),
                start_hz: 1_000_000,
                stop_hz: 3_000_000,
                points: 3,
            },
            trace,
            min_swr: SwrPoint {
                freq_hz: 2_000_000,
                swr: 1.0,
            },
            bandwidth: None,
        }
    }

    #[test]
    fn slugify_basics_and_collisions() {
        assert_eq!(slugify("HF 1-30 MHz", |_| false), "hf-1-30-mhz");
        assert_eq!(slugify("  spaces & junk!! ", |_| false), "spaces-junk");
        assert_eq!(slugify("", |_| false), "untitled");
        let taken = |s: &str| s == "hf" || s == "hf-2";
        assert_eq!(slugify("HF", taken), "hf-3");
    }

    #[test]
    fn cal_round_trip_and_list_and_delete() {
        let _h = TempHome::new();
        let cal = sample_cal("hf-1-30mhz", "HF 1-30 MHz");
        write_cal(&cal).unwrap();
        let listed = list_cals().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "hf-1-30mhz");
        let back = read_cal("hf-1-30mhz").unwrap();
        assert_eq!(back.standards.open.points.len(), 3);
        assert_eq!(back.error_terms.e10e01[0], [1.0, 0.0]);
        delete_cal("hf-1-30mhz").unwrap();
        assert!(list_cals().unwrap().is_empty());
    }

    #[test]
    fn reading_writes_touchstone_and_sidecar_and_reloads_with_stats() {
        let _h = TempHome::new();
        let r = sample_reading("40m-dipole-v2");
        write_reading(&r).unwrap();

        let s1p = std::fs::read_to_string(home().join("readings/40m-dipole-v2.s1p")).unwrap();
        assert!(s1p.contains("# MHz S RI R 50"));
        assert!(s1p.lines().any(|l| l.starts_with("2.000000")));

        let back = read_reading("40m-dipole-v2").unwrap();
        assert_eq!(back.meta.name, "40m dipole v2");
        assert_eq!(back.meta.notes, "legs -4cm");
        assert_eq!(back.trace.len(), 3);
        // stats are derived on load, not read from disk:
        assert_eq!(back.min_swr.freq_hz, 2_000_000);
        assert!((back.min_swr.swr - 1.0).abs() < 1e-9);
    }

    #[test]
    fn export_reading_copies_s1p() {
        let _h = TempHome::new();
        write_reading(&sample_reading("r1")).unwrap();
        let src = home().join("readings/r1.s1p");
        let dest = home().join("exported.s1p");
        export_reading("r1", dest.to_str().unwrap()).unwrap();
        assert!(dest.exists());
        assert_eq!(
            std::fs::read_to_string(&dest).unwrap(),
            std::fs::read_to_string(&src).unwrap()
        );
        assert!(export_reading("nope", dest.to_str().unwrap()).is_err());
    }

    #[test]
    fn export_reading_csv_has_header_rows_and_blank_inf_rl() {
        let _h = TempHome::new();
        write_reading(&sample_reading("r1")).unwrap();
        let dest = home().join("r1.csv");
        export_reading_csv("r1", dest.to_str().unwrap()).unwrap();
        let csv = std::fs::read_to_string(&dest).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(
            lines[0],
            "freq_mhz,swr,r_ohm,x_ohm,return_loss_db,gamma_re,gamma_im"
        );
        assert_eq!(lines.len(), 1 + 3); // header + 3 trace points
        assert!(lines[1].starts_with("1.000000,3.0000,"));
        // middle point has infinite RL (perfect match) -> blank field
        assert!(lines[2].contains(",50.000,0.000,,")); // r,x,<blank rl>,
        assert!(export_reading_csv("nope", dest.to_str().unwrap()).is_err());
    }

    #[test]
    fn cal_usage_counts_referencing_readings() {
        let _h = TempHome::new();
        write_cal(&sample_cal("hf", "HF")).unwrap();
        let mut r = sample_reading("r1");
        r.meta.cal_id = "hf".into();
        write_reading(&r).unwrap();
        assert_eq!(cal_usage("hf").unwrap(), 1);
    }
}

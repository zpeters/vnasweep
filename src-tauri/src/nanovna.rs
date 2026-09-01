//! Minimal NanoVNA serial link: find the device, talk to its `ch> ` shell,
//! and read back board identity. This is the only module that touches
//! `serialport`; everything above it works with the returned data.

use std::io::Write;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::model::{RawPoint, RawTrace};

/// STMicroelectronics Virtual COM Port — what the NanoVNA-H enumerates as.
const NANOVNA_VID: u16 = 0x0483;
const NANOVNA_PID: u16 = 0x5740;

/// The NanoVNA shell prints this when it is ready for the next command.
const PROMPT: &[u8] = b"ch> ";

/// What `board_info` hands back to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct BoardInfo {
    /// The serial port the device was found on.
    pub port: String,
    /// Output of `version` (a single line, e.g. `1.2.27`).
    pub version: String,
    /// Output of `info` (multi-line board / firmware dump).
    pub info: String,
}

/// Connect to the attached NanoVNA and read `version` + `info`.
///
/// Blocking: opens the port, does a short serial conversation, closes it.
/// Call it off the UI thread.
pub fn board_info() -> Result<BoardInfo, String> {
    let port_name = autodetect()?;

    let mut port = serialport::new(&port_name, 115_200)
        .timeout(Duration::from_millis(300))
        .open()
        .map_err(|e| format!("opening {port_name}: {e}"))?;

    // Wake the shell and drain anything it was mid-sending.
    port.write_all(b"\r").map_err(io_err)?;
    port.flush().map_err(io_err)?;
    let _ = read_until_prompt(port.as_mut(), Duration::from_millis(500));

    let version = run_command(port.as_mut(), "version")?;
    let info = run_command(port.as_mut(), "info")?;

    Ok(BoardInfo {
        port: port_name,
        version,
        info,
    })
}

/// Send one shell command, return its output with the echoed command line and
/// the trailing prompt stripped.
fn run_command(port: &mut dyn serialport::SerialPort, cmd: &str) -> Result<String, String> {
    port.write_all(cmd.as_bytes()).map_err(io_err)?;
    port.write_all(b"\r").map_err(io_err)?;
    port.flush().map_err(io_err)?;

    let raw = read_until_prompt(port, Duration::from_millis(1500));
    if !raw.ends_with(PROMPT) {
        return Err(format!(
            "no `ch> ` prompt after `{cmd}` — reply may be truncated"
        ));
    }
    Ok(clean_response(&String::from_utf8_lossy(&raw), cmd))
}

/// Strip the echoed command line and the trailing `ch> ` prompt, leaving just
/// the payload.
fn clean_response(raw: &str, cmd: &str) -> String {
    raw.trim_end()
        .strip_suffix("ch>")
        .unwrap_or(raw)
        .lines()
        .filter(|line| line.trim() != cmd)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Find the NanoVNA by USB VID:PID. On macOS, prefer the `/dev/cu.*` (call-up)
/// node over `/dev/tty.*`, which blocks on carrier detect.
fn autodetect() -> Result<String, String> {
    let ports = serialport::available_ports().map_err(|e| format!("listing serial ports: {e}"))?;
    for p in &ports {
        if let serialport::SerialPortType::UsbPort(usb) = &p.port_type
            && usb.vid == NANOVNA_VID
            && usb.pid == NANOVNA_PID
        {
            return Ok(p.port_name.replace("/dev/tty.", "/dev/cu."));
        }
    }
    let seen: Vec<&str> = ports.iter().map(|p| p.port_name.as_str()).collect();
    Err(format!(
        "no NanoVNA (USB {NANOVNA_VID:04x}:{NANOVNA_PID:04x}) found. Ports: {seen:?}"
    ))
}

/// Accumulate bytes until the buffer ends with the `ch> ` prompt, or until
/// `budget` elapses. Read timeouts are expected and mean "nothing yet".
fn read_until_prompt(port: &mut dyn serialport::SerialPort, budget: Duration) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    let deadline = Instant::now() + budget;

    while Instant::now() < deadline {
        match port.read(&mut chunk) {
            Ok(0) => {}
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.ends_with(PROMPT) {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
    buf
}

fn io_err(e: std::io::Error) -> String {
    format!("serial I/O: {e}")
}

/// Pair the `frequencies` output (one integer Hz per line) with the `data 0`
/// output (one `re im` pair per line). Blank lines are ignored.
fn parse_sweep(frequencies: &str, data: &str) -> Result<RawTrace, String> {
    let freqs: Vec<u64> = frequencies
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            if t.is_empty() { None } else { t.parse::<u64>().ok() }
        })
        .collect();

    let samples: Vec<(f64, f64)> = data
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let re = it.next()?.parse::<f64>().ok()?;
            let im = it.next()?.parse::<f64>().ok()?;
            Some((re, im))
        })
        .collect();

    if freqs.is_empty() {
        return Err("sweep returned no frequency points".into());
    }
    if freqs.len() != samples.len() {
        return Err(format!(
            "sweep frequency count ({}) and data count ({}) disagree",
            freqs.len(),
            samples.len()
        ));
    }

    Ok(RawTrace {
        points: freqs
            .into_iter()
            .zip(samples)
            .map(|(freq_hz, (re, im))| RawPoint { freq_hz, re, im })
            .collect(),
    })
}

/// One raw (uncorrected) S11 sweep at the given span. Blocking.
#[allow(dead_code)]
pub fn sweep_raw(start_hz: u64, stop_hz: u64, points: u32) -> Result<RawTrace, String> {
    if points == 0 {
        return Err("sweep needs at least 1 point".into());
    }
    if stop_hz <= start_hz {
        return Err("sweep stop frequency must be above the start".into());
    }

    let port_name = autodetect()?;
    let mut port = serialport::new(&port_name, 115_200)
        .timeout(Duration::from_millis(300))
        .open()
        .map_err(|e| format!("opening {port_name}: {e}"))?;

    // Wake the shell, drain anything pending.
    port.write_all(b"\r").map_err(|e| format!("serial I/O: {e}"))?;
    port.flush().map_err(|e| format!("serial I/O: {e}"))?;
    let _ = read_until_prompt(port.as_mut(), Duration::from_millis(500));

    // Disable on-device correction so we read the true raw reflection.
    run_command(port.as_mut(), "cal off")?;

    // Program the sweep span.
    run_command(
        port.as_mut(),
        &format!("sweep {start_hz} {stop_hz} {points}"),
    )?;

    // Let at least one full sweep complete before reading. The H sweeps ~2/s;
    // budget generously. (If this proves flaky, poll a status command instead —
    // see the device-verification note.)
    std::thread::sleep(Duration::from_millis(1200));

    let frequencies = run_command(port.as_mut(), "frequencies")?;
    let data = run_command(port.as_mut(), "data 0")?;

    // Be polite: re-enable device correction for standalone use.
    let _ = run_command(port.as_mut(), "cal on");

    parse_sweep(&frequencies, &data)
}

#[cfg(test)]
mod tests {
    use super::clean_response;

    #[test]
    fn strips_echo_and_prompt() {
        let raw = "version\r\n1.2.27\r\nch> ";
        assert_eq!(clean_response(raw, "version"), "1.2.27");
    }

    #[test]
    fn keeps_multiline_body() {
        let raw = "info\r\nBoard: NanoVNA-H\r\nVersion: 1.2.27\r\nch> ";
        assert_eq!(
            clean_response(raw, "info"),
            "Board: NanoVNA-H\nVersion: 1.2.27"
        );
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::parse_sweep;

    #[test]
    fn pairs_frequencies_with_samples() {
        let freqs = "1000000\n2000000\n3000000\n";
        let data = "0.98 0.03\n-0.10 0.20\n0.00 -0.01\n";
        let t = parse_sweep(freqs, data).unwrap();
        assert_eq!(t.points.len(), 3);
        assert_eq!(t.points[0].freq_hz, 1_000_000);
        assert!((t.points[1].re + 0.10).abs() < 1e-12);
        assert!((t.points[2].im + 0.01).abs() < 1e-12);
    }

    #[test]
    fn rejects_count_mismatch() {
        assert!(parse_sweep("1000000\n2000000\n", "0.1 0.2\n").is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_sweep("", "").is_err());
    }

    #[test]
    fn tolerates_blank_and_trailing_lines() {
        let freqs = "\n1000000\n2000000\n\n";
        let data = "0.1 0.2\n0.3 0.4\n";
        let t = parse_sweep(freqs, data).unwrap();
        assert_eq!(t.points.len(), 2);
    }
}

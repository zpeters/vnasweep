//! Minimal NanoVNA serial link: find the device, talk to its `ch> ` shell,
//! and read back a raw S11 sweep. This is the only module that touches
//! `serialport`; everything above it works with the returned data.

use std::io::Write;
use std::time::{Duration, Instant};

use crate::model::{PortInfo, RawPoint, RawTrace, SweepConfig};

/// STMicroelectronics Virtual COM Port — what the NanoVNA-H enumerates as.
const NANOVNA_VID: u16 = 0x0483;
const NANOVNA_PID: u16 = 0x5740;

/// The NanoVNA shell prints this when it is ready for the next command.
const PROMPT: &[u8] = b"ch> ";

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

/// Resolve the port to talk to: an explicit user override if set, else
/// autodetect by USB id. On macOS, force the `/dev/cu.*` node.
fn resolve_port(override_name: Option<String>) -> Result<String, String> {
    match override_name {
        Some(name) if !name.trim().is_empty() => Ok(name.replace("/dev/tty.", "/dev/cu.")),
        _ => autodetect(),
    }
}

/// Every serial port the OS can see, tagged with whether it looks like a
/// NanoVNA. Feeds the port picker.
pub fn list_ports() -> Result<Vec<PortInfo>, String> {
    let ports = serialport::available_ports().map_err(|e| format!("listing serial ports: {e}"))?;
    Ok(ports
        .into_iter()
        .map(|p| {
            let (description, is_nanovna) = match &p.port_type {
                serialport::SerialPortType::UsbPort(usb) => {
                    let label = usb
                        .product
                        .clone()
                        .or_else(|| usb.manufacturer.clone())
                        .unwrap_or_else(|| format!("USB {:04x}:{:04x}", usb.vid, usb.pid));
                    (label, usb.vid == NANOVNA_VID && usb.pid == NANOVNA_PID)
                }
                serialport::SerialPortType::BluetoothPort => ("Bluetooth".to_string(), false),
                serialport::SerialPortType::PciPort => ("PCI".to_string(), false),
                serialport::SerialPortType::Unknown => ("unknown".to_string(), false),
            };
            PortInfo {
                name: p.port_name.replace("/dev/tty.", "/dev/cu."),
                description,
                is_nanovna,
            }
        })
        .collect())
}

/// Open the port and drain whatever the shell had buffered.
fn open_and_wake(port_name: &str) -> Result<Box<dyn serialport::SerialPort>, String> {
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(300))
        .open()
        .map_err(|e| format!("opening {port_name}: {e}"))?;
    port.write_all(b"\r").map_err(io_err)?;
    port.flush().map_err(io_err)?;
    let _ = read_until_prompt(port.as_mut(), Duration::from_millis(500));
    Ok(port)
}

/// `version` + `info`, as one text block. Read-only — safe to call any time.
pub fn board_info(port: Option<String>) -> Result<String, String> {
    let name = resolve_port(port)?;
    let mut port = open_and_wake(&name)?;
    let version = run_command(port.as_mut(), "version")?;
    let info = run_command(port.as_mut(), "info")?;
    Ok(format!("{version}\n{info}").trim().to_string())
}

/// Read back the span the device is currently swept over.
pub fn sweep_config(port: Option<String>) -> Result<SweepConfig, String> {
    let name = resolve_port(port)?;
    let mut port = open_and_wake(&name)?;
    let freqs = run_command(port.as_mut(), "frequencies")?;
    parse_frequencies(&freqs)
}

/// Parse the `frequencies` output (one integer Hz per line) into a span.
fn parse_frequencies(text: &str) -> Result<SweepConfig, String> {
    let freqs: Vec<u64> = text
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<u64>().ok()
            }
        })
        .collect();
    match (freqs.first(), freqs.last(), u32::try_from(freqs.len())) {
        (Some(&start_hz), Some(&stop_hz), Ok(points)) if points > 0 => Ok(SweepConfig {
            start_hz,
            stop_hz,
            points,
        }),
        _ => Err("device returned no frequency points".into()),
    }
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
            if t.is_empty() {
                None
            } else {
                t.parse::<u64>().ok()
            }
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
/// `port` overrides autodetect when set (from the port picker).
pub fn sweep_raw(
    port: Option<String>,
    start_hz: u64,
    stop_hz: u64,
    points: u32,
) -> Result<RawTrace, String> {
    if points == 0 {
        return Err("sweep needs at least 1 point".into());
    }
    if stop_hz <= start_hz {
        return Err("sweep stop frequency must be above the start".into());
    }

    let port_name = resolve_port(port)?;
    let mut port = open_and_wake(&port_name)?;

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

#[cfg(test)]
mod frequencies_tests {
    use super::parse_frequencies;

    #[test]
    fn reads_span_and_count() {
        let c = parse_frequencies("1000000\n1500000\n2000000\n").unwrap();
        assert_eq!((c.start_hz, c.stop_hz, c.points), (1_000_000, 2_000_000, 3));
    }

    #[test]
    fn ignores_blank_lines() {
        let c = parse_frequencies("\n7000000\n\n7300000\n").unwrap();
        assert_eq!((c.start_hz, c.stop_hz, c.points), (7_000_000, 7_300_000, 2));
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_frequencies("").is_err());
        assert!(parse_frequencies("\n\n").is_err());
    }
}

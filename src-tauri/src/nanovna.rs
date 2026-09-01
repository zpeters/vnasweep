//! Minimal NanoVNA serial link: find the device, talk to its `ch> ` shell,
//! and read back board identity. This is the only module that touches
//! `serialport`; everything above it works with the returned data.

use std::io::Write;
use std::time::{Duration, Instant};

use serde::Serialize;

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

# TODO

Feature ideas and known gaps not yet scoped into a plan. Each needs its
own brainstorming pass before implementation (see `docs/superpowers/specs/`
for design docs and past plans).

## Fine-tuning (deferred from the v1 hardware-validation session)

- **Cleaner Open calibration.** Raw Open captures read |Γ| ≈ 0.66 on the
  test unit — low even for a raw NanoVNA open. Try the Open step with
  *nothing* connected (bare port) vs. a cap, and see if it comes up.
  A skewed Open/Short side barely affects loads near 50 Ω but throws off
  high-reflection points.
- **Poll for sweep completion.** `nanovna.rs` still uses a fixed
  `SETTLE` (1200 ms) after programming a span before reading `data 0`.
  This is the last unverified timing assumption. Replace with a poll of
  a status command, or an adaptive wait.
- **Live |Γ| per wizard step.** Show the just-captured magnitude next to
  each Open/Short/Load step so a bad or swapped standard is obvious in
  the moment, not after solving.
- **Real cal-kit offset models.** `cal.rs` assumes ideal standards
  (Γ_open = +1, Γ_short = −1, Γ_load = 0). Support open/short offset
  delay + load parasitics for better accuracy with a known kit.

## Ideas

- S21 / 2-port (`data 1`) — currently 1-port SWR only.
- Reconnect handling when the device disappears mid-session.
- Tag / filter saved readings in the Library.
- Restore the device's prior `cal on` / span state on disconnect.
- Smith chart view.

## Recently shipped

- **Scaffold** — Tauri 2 + Svelte 5, jj, flake/direnv, `just` recipes.
- **v1** — in-app OSL calibration wizard, one-shot + live SWR sweep,
  flat-file cals + Touchstone readings under `~/vnasweep/`, overlay /
  compare / difference view, CSV export, serial port picker, board-info
  and device sweep-config readback.
- **Diagnostics** — calibration health check (flags swapped / unconnected
  standards) and a 3-way diagnostic sweep (our OSL correction vs the
  device's own `cal on` vs uncorrected raw).
- **Hardware validation (2026-09-02)** — GUI runs on the NanoVNA-H;
  OSL pipeline confirmed against a 50 Ω load (SWR 1.07) and a short
  (SWR 999). The health check caught a real swapped-standards cal.

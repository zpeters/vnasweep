# vnasweep

Capture, name, and overlay [NanoVNA](https://nanovna.com/) sweeps. A
lightweight desktop app for the common case: calibrate, grab an SWR
trace, compare it against last time.

Tauri 2 (Rust backend) + Svelte 5 (frontend). Target device: NanoVNA-H
(USB `0483:5740`), macOS and Linux. Single user, local use.

## What it does

- **Calibrate** — a 3-step Open / Short / Load wizard does a one-port OSL
  calibration in the app, stored as a flat file. A health check flags a
  swapped or unconnected standard.
- **Sweep** — one-shot or live (auto-repeating) SWR sweep over a
  calibrated span. Min-SWR marker, R ± jX and return loss at the dip,
  amateur-band shading, SWR-≤ bandwidth readout.
- **Save / compare** — name a reading; it's written as Touchstone `.s1p`
  plus a JSON sidecar under `~/vnasweep/`. Overlay several on one plot,
  or show the difference of two. Export `.s1p` or `.csv`.
- **Diagnose** — one pass plotting our OSL correction against the
  device's own `cal on` correction and the uncorrected raw, to see where
  a bad reading comes from.

Status: **v1, validated on hardware.** The OSL pipeline was confirmed
against a 50 Ω load and a short. Remaining fine-tuning is in `TODO.md`.

## Dev

Nix flake + direnv provide `just` and Node/npm; Rust comes from `rustup`.

```
npm install       # once
just tauri-dev    # opens the app with hot reload
just check        # cargo fmt --check + clippy -D warnings + cargo test + npm run check + npm test
just install      # build the release bundle, copy to /Applications (macOS)
```

## Layout

- `src-tauri/src/` — Rust backend, pure core + thin shell:
  - `complex.rs` — minimal complex arithmetic.
  - `model.rs` — serde structs shared with the frontend (mirrored by hand
    in `src/lib/types.ts`).
  - `nanovna.rs` — the only module that touches `serialport`: find the
    device, drive its `ch> ` shell, read raw S11 sweeps and board info.
  - `cal.rs` — one-port OSL solve / apply, plus the calibration health
    check.
  - `sweep.rs` — SWR / impedance / return loss / bandwidth from a
    corrected trace.
  - `store.rs` — flat-file cals and Touchstone readings under
    `~/vnasweep/`.
  - `main.rs` — Tauri builder; every `#[tauri::command]` is a thin
    wrapper running blocking work on a worker thread.
- `src/` — Svelte 5 + TypeScript frontend. `App.svelte` shell, then
  `MeasureTab` / `LibraryTab` and their pieces (`CalWizard`,
  `DevicePanel`, `CalHealthPanel`, `UplotChart`).
- `docs/superpowers/{specs,plans}/` — design docs and implementation plans.
- `AGENTS.md` — conventions and known gotchas.
- `TODO.md` — unscoped ideas and deferred fine-tuning.

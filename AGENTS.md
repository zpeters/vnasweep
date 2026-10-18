# AGENTS.md

Guidance for AI coding agents working in this repo.

## What this is

`vnasweep` is a desktop app to calibrate a **NanoVNA-H**, sweep an
antenna's SWR, and save / label / overlay the results as flat files. It
exists because the stock PC software and NanoVNA-Saver are heavier than
the job needs for "grab a trace, compare it to last time".

Current state: **v1, validated on hardware** (2026-09-02). In-app OSL
calibration, one-shot + live SWR sweep, flat-file storage, overlay /
compare / difference, CSV export, serial port picker, and a diagnostics
pass (health check + 3-way sweep). See `README.md` for the feature list
and GitHub Issues (`zpeters/vnasweep`) for deferred fine-tuning and ideas.

Tauri 2 (Rust backend) + Svelte 5 (frontend). Target hardware: NanoVNA-H
(USB VID:PID `0483:5740`), on macOS and Linux. Single user, local use.

Out of scope unless asked: S21 / 2-port, driving the device's
touchscreen, other VNA families, network/remote operation, TDR math,
mobile builds.

## Stack and layout

Pure core, thin shell. Protocol parsing, the OSL math, and the SWR math
are free of I/O and clocks; the serial link, disk, and Tauri commands are
thin wrappers.

- `src-tauri/src/complex.rs` — minimal complex arithmetic. No deps.
- `src-tauri/src/model.rs` — serde structs shared with the frontend.
  **Mirrored by hand in `src/lib/types.ts` — change both.**
- `src-tauri/src/nanovna.rs` — the **only** module that touches
  `serialport`. Finds the device, drives its `ch> ` prompt-framed shell,
  returns plain data. Pure helpers (`clean_response`, `parse_sweep`,
  `parse_frequencies`, `dedupe_ports`) are unit-tested; the I/O path
  needs the device.
- `src-tauri/src/cal.rs` — one-port 3-term OSL `solve_osl` / `apply`, and
  `health` (sanity-checks a stored cal). Ideal standards for now
  (Γ_open = +1, Γ_short = −1, Γ_load = 0). Round-trip unit tests.
- `src-tauri/src/sweep.rs` — SWR, impedance, return loss, min-SWR,
  bandwidth from a corrected trace. Pure, unit-tested.
- `src-tauri/src/store.rs` — flat-file cals (`~/vnasweep/cals/*.json`) and
  readings (`*.s1p` + `*.json` sidecar). Atomic writes. Stats are derived
  on load, never stored.
- `src-tauri/src/main.rs` — the Tauri shell. Every `#[tauri::command]` is
  a thin wrapper that runs blocking serial/disk work via
  `tauri::async_runtime::spawn_blocking`. `PortOverride` managed state
  holds the user's serial-port choice. No protocol logic here.
- `src/` — Svelte 5 + TypeScript. `App.svelte` shell → `MeasureTab` /
  `LibraryTab` → `CalWizard`, `DevicePanel`, `CalHealthPanel`,
  `UplotChart`. `lib/api.ts` wraps every command; `lib/plot.ts` builds
  uPlot series + plugins. Thin render layer — no protocol logic.

`docs/superpowers/{specs,plans}/` — the v1 design doc and implementation
plan.

## Principles

Cross-project coding philosophy lives in `~/.claude/PHILOSOPHY.md` and
`~/.claude/PRINCIPLES.md` — they apply here. Project-specific rules:

- **Pure core, thin shell.** See the layout above. Keep the math and
  parsing free of I/O and clocks.
- **`#![deny(unsafe_code)]`** at the `src-tauri` crate root (`main.rs`).
- **The NanoVNA shell protocol lives only in `nanovna.rs`.** Line
  oriented, echoes the command, ends every response with `ch> `. Keep
  that quirk and the prompt sentinel there.
- **Commands are the wire contract.** A `#[tauri::command]` name + its
  argument/return types are a contract with `lib/api.ts`. The Rust
  structs in `model.rs` and their mirror in `lib/types.ts` are kept in
  sync by hand.

## Dev environment

Nix flake (`flake.nix`) + direnv (`.envrc`) provide `just`, Node/npm and
`resvg`.
Rust/Cargo comes from your own `rustup`, not the flake. (`mkShellNoCC`
is deliberate — see the comment in `flake.nix`.)

```
just              # list recipes (JS deps install themselves via `deps`)
just tauri-dev    # opens the app with hot reload
just check        # cargo fmt --check + clippy -D warnings + cargo test + npm run check + npm test
just install      # release bundle -> /Applications/vnasweep.app (macOS)
just dmg          # ad-hoc signed .dmg -> target/release/bundle/dmg/
just logo         # assets/logo/*.svg -> *.png (README banner, social preview)
```

CI (`.github/workflows/ci.yml`) runs `just check` on macOS and Linux.
`release.yml` runs on a `v*` tag: it checks the tag against the version
in `tauri.conf.json`, `Cargo.toml` and `package.json` (keep all three
equal), builds a `.dmg` and Linux `.deb` / `.AppImage` with
`tauri-action`, and attaches them to a draft release. Dependabot opens
weekly grouped updates for crates, npm and Actions.

The webview has a CSP (`app.security` in `tauri.conf.json`). It allows
inline styles (Svelte `style=`, uPlot) but no inline or remote scripts
and no network except Tauri IPC. Store ids from IPC go through
`store::id_path`, which rejects anything that is not a slug.

## Version control

`jj` (Jujutsu), colocated with git — use `jj`, not `git commit`.

**Run `jj new` before starting a new unit of work** if you're not already
on a fresh empty commit.

```
jj new
# ... changes ...
jj describe -m 'Message'
jj bookmark set main -r @
jj git push --bookmark main                   # remote 'origin' -> git:/git/vnasweep.git
jj git push --bookmark main --remote github   # public mirror -> github.com/zpeters/vnasweep
```

Personal project: **merge locally, never push feature branches or open
PRs.** Push `main` directly when a unit of work is done and green.

## Testing expectations

- Logic changes (protocol parsing, OSL / SWR math, storage): add/update
  `#[cfg(test)] mod tests` in the same file. `cargo test` before calling
  a change done. TDD is the default for parsing/math features and
  bugfixes.
- Serial/transport code has no unit suite by design — it needs the
  device. Verify with `just tauri-dev` and a NanoVNA attached; note what
  was checked.
- Frontend: `npm run check` (svelte-check) must be clean; pure helpers in
  `lib/` get vitest tests (`*.test.ts`). Both are in `just check`.

## Before making a change

- macOS: use the `/dev/cu.*` node, not `/dev/tty.*` (blocks on carrier
  detect). `resolve_port` / `dedupe_ports` normalize this.
- Baud is nominally 115200 but the NanoVNA CDC ignores it — never blame a
  baud mismatch for a comms failure.
- The `src-tauri` crate is `edition = "2024"` (let-chains); the workspace
  root has no crate of its own.
- Device seen during validation: NanoVNA-H, fw 1.2.27, at
  `/dev/cu.usbmodem4001`. Port name and serial differ per unit.
- **Working raw-sweep sequence** (confirmed on fw 1.2.27): `cal off` →
  `sweep {start} {stop} {points}` → wait `SETTLE` (1200 ms) →
  `frequencies` → `data 0` (S11 as `re im` lines) → `cal on`. The fixed
  `SETTLE` is the one remaining unverified timing assumption — if sweeps
  come back with garbage on some points, poll for completion instead
  (there is a GitHub issue for this).
- **Stored calibration standards are raw** (`cal off`) captures, not
  ideal values — a raw NanoVNA open reads |Γ| ≈ 0.6–0.9, not 1.0. The
  `cal::health` thresholds account for this; don't tighten them back to
  ideal-standard expectations.
- The `sweep` command grid-checks the device's returned frequencies
  against the calibration's grid before applying correction (`check_grid`
  in `main.rs`) — a mismatch means re-run the cal for that span.

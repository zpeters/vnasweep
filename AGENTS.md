# AGENTS.md

Guidance for AI coding agents working in this repo.

## What this is

`vnasweep` is a desktop app to capture sweeps from a **NanoVNA** over its USB
CDC serial link, save them as named records, and overlay old vs new so
changes to an antenna/filter/cable are easy to see. It exists because the
stock PC software and NanoVNA-Saver are heavier than the job needs for "grab
a trace, compare it to last time".

Current state: scaffold only. One button, *Read board info*, which runs the
NanoVNA's `version` + `info` shell commands and displays them. Sweep capture,
storage, and overlay are not built yet — that's the next design pass.

Tauri 2 (Rust backend) + Svelte 5 (frontend). Target hardware: **NanoVNA-H**
(USB VID:PID `0483:5740`), on macOS and Linux. Single user, local use.

Explicitly out of scope unless asked: a calibration workflow, driving the
device's touchscreen/UI, other VNA families, network/remote operation, TDR
math, mobile builds.

## Stack and layout

Three layers, kept separate:

- `src-tauri/src/nanovna.rs` — the **only** module that touches
  `serialport`. Finds the device, drives its `ch> ` prompt-framed shell,
  returns plain data (`BoardInfo`). Pure helpers here (`clean_response`) are
  unit-tested; the I/O path needs the device.
- `src-tauri/src/main.rs` — the Tauri shell. Registers `#[tauri::command]`s,
  each a thin wrapper that runs the blocking serial work via
  `tauri::async_runtime::spawn_blocking` so the UI never freezes. No
  protocol logic here.
- `src/` — Svelte 5 + TypeScript frontend. `App.svelte` is a thin render
  layer: call a command, show the result or the error string. No business
  logic.

`docs/superpowers/specs/` and `docs/superpowers/plans/` — design docs and
implementation plans. Nothing there yet; the design pass is the next task.

## Principles

Cross-project coding philosophy lives in `~/.claude/PHILOSOPHY.md` and
`~/.claude/PRINCIPLES.md` — they apply here. Project-specific rules:

- **Pure core, thin shell.** Protocol parsing and (eventually) the sweep
  data model stay free of I/O and clocks. The serial link, disk, and Tauri
  commands are thin wrappers.
- **`#![forbid(unsafe_code)]`** at the `src-tauri` crate root.
- **The NanoVNA shell protocol lives only in `nanovna.rs`.** It is
  line-oriented, echoes the command back, and ends every response with
  `ch> `. Keep that quirk and the prompt sentinel there, not scattered.
- **Commands are the wire contract.** A `#[tauri::command]` name and its
  return type are a contract with `App.svelte`. The Rust `BoardInfo` struct
  and its TS mirror in `App.svelte` are kept in sync by hand — change both.

## Dev environment

Nix flake (`flake.nix`) + direnv (`.envrc`) provide `just` and Node/npm.
Rust/Cargo comes from your own `rustup`, not the flake.

```
npm install       # once
just              # list recipes
just tauri-dev    # opens the app with hot reload
just check        # cargo fmt --check + clippy -D warnings + cargo test + npm run build
```

## Version control

`jj` (Jujutsu), colocated with git — use `jj`, not `git commit`.

**Run `jj new` before starting a new unit of work** if you're not already on
a fresh empty commit.

```
jj new
# ... changes ...
jj describe -m 'Message'
jj bookmark set main -r @
```

Personal project: **merge locally, never push feature branches or open PRs.**
`jj git push --bookmark main` when a unit of work is done and green. (No
remote configured yet — needs the home network.)

## Testing expectations

- Logic changes (protocol parsing, future sweep model): add/update
  `#[cfg(test)] mod tests` in the same file. Run `cargo test` before calling
  a change done.
- Serial/transport code has no unit suite by design — it needs the device.
  Verify by running `just tauri-dev` with a NanoVNA attached and clicking
  the button; note what was checked.
- Frontend: `npm run build` must type-check and build clean (it's in
  `just check`).
- TDD is the default for new parsing/model features and bugfixes.

## Before making a change

- macOS: use the `/dev/cu.*` node, not `/dev/tty.*` (blocks on carrier
  detect). `autodetect` normalizes this.
- Baud is nominally 115200 but the NanoVNA CDC ignores it — never blame a
  baud mismatch for a comms failure.
- The `src-tauri` crate is `edition = "2024"` (needed for the let-chain in
  `autodetect`); the workspace root has no crate of its own.
- Device seen at scaffold: NanoVNA-H, fw 1.2.27, serial `400`, at
  `/dev/cu.usbmodem4001`. Port name and serial differ per unit.
- Not yet runtime-verified: the Tauri window actually opening (built and
  type-checked only), and anything past `version` / `info`. `scan`,
  `data`, `frequencies` output formats are assumed, not confirmed.
- `src-tauri/icons/` is a plain generated placeholder (solid teal). Replace
  with a real icon whenever.

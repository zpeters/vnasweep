# vnasweep

Capture, name, and overlay [NanoVNA](https://nanovna.com/) sweeps. A
lightweight desktop app for the common case: grab a trace, compare it
against last time.

Status: **early scaffold.** Right now the app has one button — *Read board
info* — which connects to the NanoVNA over USB CDC and shows its `version`
and `info` output. Sweep capture and overlay are the next work.

Tauri 2 (Rust backend) + Svelte 5 (frontend). Target device: NanoVNA-H
(USB `0483:5740`), macOS and Linux.

## Dev

Nix flake + direnv provide `just` and Node/npm; Rust comes from `rustup`.

```
npm install       # once
just tauri-dev    # opens the app with hot reload
just check        # cargo fmt --check + clippy -D warnings + cargo test + npm run build
```

## Layout

- `src-tauri/` — Rust backend.
  - `src/nanovna.rs` — the only module that touches `serialport`: find the
    device, talk to its `ch> ` shell, read board identity.
  - `src/main.rs` — Tauri builder + the `board_info` command.
- `src/` — Svelte 5 + TypeScript frontend (`App.svelte`, `main.ts`).
- `docs/superpowers/{specs,plans}/` — design docs and implementation plans.
- `AGENTS.md` — conventions and known gotchas.
- `TODO.md` — unscoped ideas.

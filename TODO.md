# TODO

Feature ideas and known gaps not yet scoped into a plan. Each needs its
own brainstorming pass before implementation (see `docs/superpowers/specs/`
for design docs and past plans).

## Next up

- **Design pass.** Decide the sweep data model, on-disk format, and how
  overlay/compare is presented in the UI.
- **Confirm the window runs.** Only built + type-checked so far —
  `just tauri-dev` on a machine with the NanoVNA attached, click the
  button, confirm board info shows.

## Ideas

- Capture a full sweep: `scan <start> <stop> <points>` then `data 0` /
  `data 1`, parse the S-param lines.
- Name + save sweeps (git-friendly, or Touchstone `.s1p`/`.s2p`), list, tag.
- Overlay old vs new on one plot in the app.
- Read back the device's current sweep config (`frequencies`, `sweep`).
- Reconnect / device-picker UI (right now `autodetect` takes the first
  matching USB device).
- Real app icon (current one is a generated placeholder).

## Recently shipped

- **Scaffold** — Tauri 2 + Svelte 5, jj, flake/direnv, `just` recipes.
- **Board info** — `board_info` command + button: connects over USB CDC,
  runs `version` + `info`, shows the result. Serial protocol lives in
  `src-tauri/src/nanovna.rs`; `clean_response` is unit-tested.

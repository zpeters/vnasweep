# vnasweep — morning verification checklist

Everything below is **code-complete and gate-green** but **not run against
the NanoVNA-H**. Work top to bottom with the device connected. Tick as you
go; note failures inline.

Commits under test (newest last), all on `main`:

| change | what |
|---|---|
| `feat(icons)` | real app icon from `src-tauri/icons/icon.svg` |
| `feat(ui): live sweep …` | live mode, marker Z readout, reference overlay, empty/error states |
| `feat(device): serial port picker …` | port picker, `board_info`, device sweep-config readback |
| `feat: … threshold + CSV export` | SWR-threshold selector, `.csv` export |
| `feat(ui): keyboard shortcuts + Library difference view` | space/L keys, B−A delta plot |

---

## 0. Build + gate

- [ ] `direnv exec . just check` — `fmt`, `clippy -D warnings`, 33 Rust tests, `svelte-check` all green.
- [ ] `direnv exec . npm install` (uPlot etc. already in `package-lock.json`).
- [ ] `direnv exec . just tauri-dev` — window opens on the **Measure** tab. First confirmed GUI launch.

## 1. App icon

- [ ] Dock / task-switcher icon is the dark rounded square with the teal→blue resonance-dip curve and coral marker — **not** the flat teal placeholder.
- [ ] `src-tauri/icons/` holds only the 5 referenced files + `icon.png` + `icon.svg` (no stray `Square*`/`android`/`ios`).

## 2. Device panel (Measure tab, "Device" disclosure)

- [ ] Expand **Device**. The **Port** dropdown lists real serial ports; the NanoVNA row is prefixed `★`.
- [ ] Leave it on **auto-detect**. Click **Identify** → shows `version` + `info` text from the device.
- [ ] Click **Read device config** → shows `start–stop MHz · N pts` matching whatever the device is currently set to.
- [ ] Pick the NanoVNA port explicitly → **Identify** still works. Summary line shows `Device · /dev/cu.usbmodemXXXX`.
- [ ] Pick a **wrong** port → Identify fails with a readable error, no crash. Set back to auto-detect.
- [ ] Unplug the device → **Refresh** drops it from the list; **Identify** shows the "no NanoVNA found" error.

## 3. Calibration (unchanged path, still needs a pass)

- [ ] **＋ New calibration**, name `smoke`, span 2–4 MHz, 51 points.
- [ ] Open → Capture, Short → Capture, Load → Capture. Wizard closes, `smoke` is selected.
- [ ] `~/vnasweep/cals/smoke.json` exists; `standards` and `error_terms` are each length 51.

## 4. One-shot sweep + save

- [ ] 50 Ω terminator on the port. Press **Sweep** (or hit **space**).
- [ ] Plot draws; coral min-SWR marker; readout line shows `min SWR x : 1 @ f MHz`.
- [ ] Readout also shows `Z ≈ R ± jX Ω · RL … dB` at the dip. For a terminator: R ≈ 50, X ≈ 0, SWR ≈ 1.0–1.1.
- [ ] Enter a name, **Save reading** → `~/vnasweep/readings/<slug>.s1p` + `<slug>.json` exist.

## 5. SWR threshold selector

- [ ] Change **SWR ≤** to `1.5`, sweep again → the `SWR ≤ 1.5 from a–b MHz` band in the readout narrows vs `2.0`.
- [ ] Set to `3.0`, sweep → band widens. (Saved readings always reload at 2.0 — expected.)

## 6. Live sweep

- [ ] Set **every** to `2 s`. Click **▶ Live** (or hit **L**). Button turns red `■ Stop`; `● live · updated Ns ago` badge ticks.
- [ ] Plot refreshes each cycle. Real cadence ≈ 2 s + sweep time. **Sweep** button is disabled while live.
- [ ] Wiggle the antenna / touch the load — the trace tracks it within one cycle.
- [ ] **Unplug the device mid-live** → loop **stops** (button back to `▶ Live`), error shown, no runaway retries.
- [ ] Re-plug, start live again, then switch the **Calibration** dropdown → live **stops** and the reference clears.
- [ ] Start live, switch to the **Library** tab and back → no zombie timer (check: badge gone, only one sweep runs when you restart).

## 7. Reference overlay

- [ ] With a reading on screen, click **Keep as reference** → dashed grey curve labelled `… (ref)` stays on the plot.
- [ ] Sweep again → new blue trace over the frozen grey one.
- [ ] **Clear reference** → grey curve gone.

## 8. Empty / error states

- [ ] Fresh `VNASWEEP_HOME` (or move `~/vnasweep/cals` aside): Measure tab shows the "No calibrations yet" hint; **Sweep**/**Live** disabled.
- [ ] Trigger a device error → the "check the NanoVNA-H is plugged in… no other program holds the port" hint appears under the raw error.

## 9. Library — CSV + difference view

- [ ] Save a second reading (`smoke reading 2`).
- [ ] Library tab → tick both → two labelled curves on one plot.
- [ ] A **show difference (B − A)** checkbox appears (only with exactly 2 ticked). Toggle it → single coral `Δ SWR` trace, auto y-scale, crossing ~0 where they agree.
- [ ] Untick one → difference checkbox disappears, plot returns to overlay.
- [ ] On a reading row: **.s1p** → save dialog → file written. **.csv** → `<name>.csv` with header `freq_mhz,swr,r_ohm,x_ohm,return_loss_db,gamma_re,gamma_im` and one row per point; RL blank where SWR = 1.000.
- [ ] **rename** / **delete** still work.

## 10. Keyboard

- [ ] Measure tab, focus not in a field: **space** sweeps, **L** toggles live.
- [ ] Focus a text input and press space → types a space, does **not** sweep.

---

## If something in the sweep path is wrong

The raw sweep sequence in `src-tauri/src/nanovna.rs` (`sweep_raw`) is still the
unverified part: `cal off` → `sweep {s} {e} {n}` → **fixed 1200 ms sleep** →
`frequencies` → `data 0` → `cal on`. If sweeps come back truncated or the grid
check in the `sweep` command fires, the sleep is the first suspect — bump it or
switch to polling a status command.

## After a clean pass — docs to update (plan Task 11 step 7)

- [ ] `AGENTS.md` — replace the "not yet runtime-verified" note with the exact working command sequence + firmware version. Drop the "window not confirmed to open" caveat.
- [ ] `TODO.md` — move "Design pass", "Confirm the window runs", "read back device sweep config", "device-picker UI" into "Recently shipped".
- [ ] `README.md` — status line + module list.
- [ ] Update the memory file `…/memory` status line to "v1 GUI verified on device".

## Not built (deliberately deferred — need design + device)

- Smith-chart view.
- Polling-based sweep-completion (replacing the fixed 1200 ms wait).
- Reading tags / filtering (storage schema change).
- Any 2-port / S21 work — out of scope for a 1-port SWR tool.

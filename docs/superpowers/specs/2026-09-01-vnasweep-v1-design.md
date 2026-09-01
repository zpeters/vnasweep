# vnasweep v1 — design

Status: approved 2026-09-01. Next: implementation plan (`writing-plans`).

## 1. Purpose

`vnasweep` is a desktop companion for a NanoVNA-H, aimed at one job: **build
an antenna, tune its SWR, and keep a labelled record of the result.**

The three problems it solves, in the user's words:

1. *"Hard to read the display."* → a big desktop SWR plot with a large
   min-SWR readout and band shading.
2. *"No good way to label or long-term save the calibrations."* → calibrations
   are named files on the PC, computed in-app, not the device's five slots.
3. *"No good way to label or save the readings."* → readings are named
   Touchstone files with a metadata sidecar; saved readings can be overlaid
   to compare antenna builds over time.

## 2. Scope

### v1 does

- **In-app OSL (1-port) calibration.** The app reads *raw, uncorrected* S11
  from the device, walks the user through Open / Short / Load standards,
  computes the 3-term error model, and stores it as a named file.
- **One-shot sweep.** Pick a calibration, press Sweep, get one corrected
  trace. Nothing is saved until the user names it.
- **SWR readout.** SWR vs frequency plot; auto marker at minimum SWR;
  "SWR ≤ 2" bandwidth span; hover readout of SWR and R ± jX; amateur-band
  shading on the frequency axis.
- **Save / label readings.** Name + notes → a Touchstone `.s1p` plus a JSON
  sidecar in a plain folder.
- **Compare.** Select two or more saved readings and overlay their SWR
  curves on one plot, each labelled, on a shared axis (each curve spans
  its own measured range).
- **Library management.** List, rename, delete, and export (copy-to) both
  readings and calibrations.

### v1 does not — deferred to v2+

- Continuous / live sweep (watch SWR move while trimming). *The sweep
  primitive is shared, so this is additive later.*
- 2-port / S21 (filters, duplexers, amplifiers). Keeps calibration a 3-term
  SOL solve instead of a 12-term SOLT solve.
- Segmented sweeps for more than the device's native point count.
- Real cal-kit coefficient models (open fringing capacitance C0–C3, offset
  delay). v1 uses ideal standards; see §5.4.
- Smith chart, phase / group-delay / TDR traces.
- Sweep averaging.
- Frequency / band presets, re-calibration reminders.

### Rationale for the shape

- **1-port only:** the stated use is antenna SWR. 2-port would roughly
  quadruple the calibration math for a case the user does not have.
- **In-app OSL, not device cal:** the pain is *naming and long-term
  storage*. The device has five slots and finite flash; a PC folder has
  neither limit. This is the model NanoVNA-Saver uses.
- **One-shot, not live, in v1:** the user chose the smaller v1. Live is the
  bigger fix for "hard to read the display" but it needs a sweep loop,
  cancellation, and a plot pipeline that keeps up — a clean v2.
- **Flat files, not SQLite:** the output *is* a file format people share
  (`.s1p`); calibrations get copied between machines; everything is
  git-friendly and hand-inspectable; there are no query needs a folder
  cannot meet.
- **uPlot for charting:** ~45 KB, canvas, purpose-built for instrument line
  plots, and it also handles the v2 live-streaming case — no rewrite.

## 3. User flows

### 3.1 Calibrate

1. Measure tab → cal dropdown → **＋ New calibration**.
2. Enter name, notes, start Hz, stop Hz, points (≤ device max, 101 on the H).
3. Wizard, three steps: *"Connect the OPEN standard, then Next."* → the app
   runs one raw sweep. Repeat for SHORT and LOAD.
4. App solves the 3-term model and writes `cals/<slug>.json` (raw standards
   **and** error terms).
5. The new calibration is selected in the dropdown.

### 3.2 Measure and save

1. Measure tab → select a calibration. The UI shows its span and age
   ("HF 1–30 MHz · 101 pts · 3 days old").
2. **Sweep.** The app sweeps raw at *the calibration's exact frequency
   grid*, applies the error terms, and draws the corrected SWR trace with
   the min-SWR marker and bandwidth span.
3. **Save reading…** → name + notes → `readings/<slug>.s1p` +
   `readings/<slug>.json`.

### 3.3 Compare builds

1. Library tab → tick two or more readings.
2. The right pane overlays their SWR curves, labelled. Each is drawn across
   its own frequency extent; where grids differ, the axis is the union and
   a curve simply stops where its data does.
3. Tick one only → its stats (min SWR, bandwidth, cal used, date).

## 4. Architecture

Backend is Rust in `src-tauri/`, frontend is Svelte 5 in `src/`. All I/O and
device state lives in the backend; the frontend is a render layer that calls
`#[tauri::command]`s and shows results or error strings.

### 4.1 Backend modules (`src-tauri/src/`)

| module | kind | responsibility |
|---|---|---|
| `nanovna.rs` | I/O | Serial port + the `ch> ` shell. **All** wire quirks. Exists (board info). Adds `sweep_raw(start_hz, stop_hz, points) -> RawTrace`. |
| `cal.rs` | pure | `solve_osl(open, short, load) -> ErrorTerms`; `apply(raw, &ErrorTerms) -> Vec<Complex>`. No I/O. Unit-tested with synthetic data. |
| `sweep.rs` | pure | `swr(gamma) -> f64`, `impedance(gamma) -> (f64, f64)`, `return_loss_db(gamma) -> f64`, `min_swr(&[TracePoint]) -> SwrPoint`, `swr_bandwidth(&[TracePoint], threshold) -> Option<Band>`. No I/O. Unit-tested. |
| `store.rs` | fs only | The `~/vnasweep/` folder. Serialize/deserialize `Calibration` and `Reading`; slug generation; list/rename/delete. |
| `model.rs` | pure | Shared `serde` structs (§4.4). |
| `main.rs` | thin | Tauri builder + commands. Each command wraps the blocking work in `tauri::async_runtime::spawn_blocking` and returns `Result<T, String>`. No protocol or math here. |

`Complex` is a small local `{ re: f64, im: f64 }` with the handful of ops
needed (add, sub, mul, div, abs). No `num-complex` dependency unless the math
grows.

### 4.2 Statelessness

The calibration wizard is **stateless on the backend**. The frontend calls
`sweep_raw` three times, holds the three raw traces itself, then calls
`save_calibration(…, open, short, load)`. Three 101-point complex arrays are
a few KB each; not worth a backend session map and its lifecycle.

### 4.3 Commands

```
list_cals()                                   -> Vec<CalMeta>
get_cal(id)                                    -> Calibration
sweep_raw(start_hz, stop_hz, points)           -> RawTrace          # cal wizard
save_calibration(name, notes, start_hz, stop_hz, points,
                 open: RawTrace, short: RawTrace, load: RawTrace)
                                               -> CalMeta
rename_cal(id, name)                            -> CalMeta
delete_cal(id)                                  -> ()

sweep(cal_id)                                  -> Reading           # unsaved
save_reading(reading: Reading, name, notes)    -> ReadingMeta
list_readings()                                -> Vec<ReadingMeta>
get_reading(id)                                -> Reading
rename_reading(id, name)                        -> ReadingMeta
delete_reading(id)                              -> ()
export_reading(id, dest_path)                   -> ()               # copy .s1p out

board_info()                                   -> BoardInfo         # exists
```

`sweep(cal_id)` deliberately takes no span — it reads the calibration and
sweeps at *that* grid. See §5.3.

`export_reading` uses `@tauri-apps/plugin-dialog` for the destination
picker (house pattern; tally uses the same plugin).

### 4.4 Shared types (`model.rs` ⇄ `src/lib/types.ts`, kept in sync by hand)

```rust
struct CalMeta     { id, name, notes, created: String,          // RFC3339
                     start_hz: u64, stop_hz: u64, points: u32 }
struct RawTrace    { points: Vec<RawPoint> }                    // as measured
struct RawPoint    { freq_hz: u64, re: f64, im: f64 }
struct Standards   { open: RawTrace, short: RawTrace, load: RawTrace }
struct ErrorTerms  { e00: Vec<[f64;2]>, e11: Vec<[f64;2]>,
                     e10e01: Vec<[f64;2]> }                      // per point
struct Calibration { meta: CalMeta, standards: Standards, error_terms: ErrorTerms }

struct TracePoint  { freq_hz: u64, gamma_re: f64, gamma_im: f64,
                     swr: f64, r: f64, x: f64, return_loss_db: f64 }
struct SwrPoint    { freq_hz: u64, swr: f64 }
struct Band        { lo_hz: u64, hi_hz: u64, threshold: f64 }
struct ReadingMeta { id, name, notes, created: String,
                     cal_id: String, cal_name: String,
                     start_hz: u64, stop_hz: u64, points: u32 }
struct Reading     { meta: ReadingMeta, trace: Vec<TracePoint>,
                     min_swr: SwrPoint, bandwidth: Option<Band> }
```

`sweep()` returns a `Reading` with `meta.id` and `meta.name` empty;
`save_reading` fills them and writes the files.

## 5. Calibration

### 5.1 The 3-term one-port error model

Measured reflection `Γm` relates to actual reflection `Γa` at each frequency
point by:

```
Γm = e00 + (e10·e01 · Γa) / (1 − e11 · Γa)
```

with three complex unknowns per point:

- `e00`     — directivity
- `e11`     — source match
- `e10·e01` — reflection tracking (stored as one product; the individual
  terms are not separable from a 1-port measurement)

### 5.2 Solve (ideal standards)

Standards, ideal: `Γ_open = +1`, `Γ_short = −1`, `Γ_load = 0`.

With `Γ_load = 0`, the load measurement gives directivity directly:

```
e00 = Γm_load
```

Let `Mo = Γm_open − e00`, `Ms = Γm_short − e00`. Substituting the open and
short into the model and eliminating `e10e01`:

```
e11    = (Mo + Ms) / (Mo − Ms)
e10e01 = Mo · (1 − e11)
```

Done per frequency point; the three error-term vectors are the calibration.

### 5.3 Apply

Invert the model:

```
D  = Γm − e00
Γa = D / (e10e01 + e11 · D)
```

Then per point:

```
|Γ|  = abs(Γa)                       clamp to < 1 for SWR (see §5.5)
SWR  = (1 + |Γ|) / (1 − |Γ|)
Z    = 50 · (1 + Γa) / (1 − Γa)      →  R = Re Z,  X = Im Z
RL   = −20 · log10(|Γ|)              dB
```

Because `apply` is elementwise over the point index, **the calibration and
the antenna sweep must be on the same frequency grid.** This is enforced
structurally, not checked: `sweep(cal_id)` sweeps at the calibration's
`{start_hz, stop_hz, points}`. A different span means a new calibration.
This removes the single most common silent-garbage mistake (applying a
mismatched cal) and removes all "does my cal still fit?" logic from the UI.

### 5.4 Ideal-standard error (accepted for v1)

Real SMA standards are not ideal: an open has ~tens of fF of fringing
capacitance, a short has a small offset delay. Ignoring them rotates the
reference plane slightly — negligible for SWR *magnitude* at HF, larger at
VHF/UHF. v1 accepts this. Because the raw standard sweeps are stored in the
calibration file, v2 can add the coefficient model and **re-solve without
re-measuring.**

### 5.5 Degenerate cases

- `|Γa| ≥ 1` (noise, or an active/negative-resistance DUT): report
  `swr = 999.0` and keep the raw `gamma_*`, `r`, `x` for the plot.
- `Mo == Ms` (bad or identical open/short measurements): `save_calibration`
  fails with a clear message; nothing is written.
- Any `NaN`/`inf` in a solved term: same — fail the save.

## 6. NanoVNA protocol

Extends `nanovna.rs`. The `ch> `-framed shell already works for
`version` / `info`. **The exact raw-sweep command sequence is assumed here
and must be verified against firmware 1.2.27 in the implementation plan.**

Intended sequence for one raw sweep:

```
cal off                         # disable on-device correction
sweep {start_hz} {stop_hz} {points}
<one settle sweep, then>
frequencies                     # authoritative per-point frequency list
data 0                          # S11 as "re im" lines, one per point
```

Notes / risks to resolve in the plan:

- Whether a full sweep must complete before `data 0` returns fresh values,
  and how to detect completion (poll, fixed delay, or a status command).
- Some firmwares expose `scan {start} {stop} {points} {outmask}` as a
  combined set-scan-dump; if `sweep` + `data 0` is unreliable, use `scan`.
- The device's max point count (101 on the H; do not offer more in v1).
- Restore `cal on` / the user's prior device state on disconnect so the
  standalone device is left as found. (Nice-to-have; note if skipped.)

Pair frequencies to samples by index from the `frequencies` output rather
than recomputing a linspace, in case the device quantizes.

## 7. Storage

### 7.1 Location

Default `$HOME/vnasweep/`. Overridable by the `VNASWEEP_HOME` environment
variable (used by tests and anyone who wants the data elsewhere). Created on
first write.

```
~/vnasweep/
  cals/
    hf-1-30mhz.json
  readings/
    40m-dipole-v2.s1p
    40m-dipole-v2.json
```

### 7.2 Calibration file — `cals/<slug>.json`

```json
{
  "meta": {
    "id": "hf-1-30mhz",
    "name": "HF 1-30 MHz",
    "notes": "SMA cal kit, bench, ~22 C",
    "created": "2026-09-01T20:12:00Z",
    "start_hz": 1000000,
    "stop_hz": 30000000,
    "points": 101
  },
  "standards": {
    "open":  { "points": [ { "freq_hz": 1000000, "re": 0.98, "im": 0.03 }, … ] },
    "short": { "points": [ … ] },
    "load":  { "points": [ … ] }
  },
  "error_terms": {
    "e00":    [ [re, im], … ],
    "e11":    [ [re, im], … ],
    "e10e01": [ [re, im], … ]
  }
}
```

### 7.3 Reading files — `readings/<slug>.{s1p,json}`

`<slug>.s1p` — Touchstone 1.1, so any other tool reads it:

```
! vnasweep  40m-dipole-v2  2026-09-01T20:40:00Z  cal: HF 1-30 MHz
# MHz S RI R 50
1.000000  0.812  -0.043
1.290000  0.799  -0.051
…
```

`<slug>.json` — sidecar the `.s1p` cannot hold:

```json
{
  "id": "40m-dipole-v2",
  "name": "40m dipole v2",
  "notes": "legs shortened 4 cm each, fed at 9 m",
  "created": "2026-09-01T20:40:00Z",
  "cal_id": "hf-1-30mhz",
  "cal_name": "HF 1-30 MHz"
}
```

Stats (min SWR, bandwidth) are **derived on load**, not stored — they follow
from the trace and would only drift out of sync.

### 7.4 Slugs

`slug = kebab-case(name)`, lowercased, non-alphanumerics collapsed to `-`.
On collision append `-2`, `-3`, … The slug is the `id`. Rename rewrites the
`meta.name`/sidecar `name` only; the slug/id and filenames are stable so
references from readings to their calibration do not break.

## 8. Frontend

Svelte 5, uPlot. Two tabs.

### 8.1 uPlot wrapper

`src/lib/UplotChart.svelte` — a thin wrapper: takes labelled series plus
options, owns the uPlot instance, destroys it on unmount, resizes with the
container. ~40–60 lines.

uPlot uses one shared x array for all series. Readings with different
frequency grids are merged onto the **union** of their frequencies, with
each series carrying `null` where it has no sample there (uPlot draws a gap).
No resampling/interpolation — a reading is only drawn at frequencies it was
actually measured at.

`src/lib/bands.ts` — amateur band edges (Hz) for the shaded regions.
`src/lib/plot-plugins.ts` — uPlot draw-hook plugins for band shading, the
min-SWR marker + label, and the "SWR ≤ 2" span.

Frequency axis: `log` when `stop/start > 3`, else linear.

### 8.2 Measure tab

- Calibration `<select>` populated from `list_cals()`. `＋ New calibration`
  opens the wizard (a modal or an inline panel): name/notes/start/stop/points
  form, then three "connect X → capture" steps calling `sweep_raw`, then
  `save_calibration`.
- Selected-cal line: name, span, point count, age.
- **Sweep** → `sweep(cal_id)` → uPlot SWR trace + min-SWR marker + bandwidth
  span; hover shows SWR and R ± jX at the nearest point.
- **Save reading…** → name + notes → `save_reading`.
- Errors (no device, timeout, bad cal) render as a dismissible banner.

### 8.3 Library tab

- Left: readings list — name, date, cal name; a checkbox per row; row
  actions rename / delete / export.
- Right: uPlot. Checked readings draw as labelled series on the union
  frequency axis (§8.1). Exactly one checked → also show its stats block.
- Below, collapsible: calibrations list with rename / delete and a "used by
  N readings" count (delete warns if N > 0; the readings keep working, they
  just lose the back-reference name).

## 9. Error handling

- Every command returns `Result<T, String>`; the `Err` string is
  user-facing. No `unwrap`/`expect`/`panic!` on any command path.
- Device errors: not found, port busy, open failed, prompt timeout, short
  read — each a distinct message naming the likely cause.
- File errors: parse failure names the file; a corrupt cal/reading is
  skipped in list views with a logged warning, not a hard failure.
- `save_calibration` validates the solved terms (§5.5) before writing;
  partial writes are avoided by writing to a temp file and renaming.

## 10. Testing

- `cal.rs`: synthetic round-trip — fabricate `ErrorTerms`, generate the
  three "measured" standards from ideal `Γ`, `solve_osl`, assert the terms
  come back within `1e-9`; then generate a "measured" DUT from a known
  `Γa`, `apply`, assert recovery. Include a `Mo == Ms` failure case.
- `sweep.rs`: table tests for `swr`, `impedance`, `return_loss_db` against
  hand-computed values; `min_swr` and `swr_bandwidth` over a crafted trace
  including the no-crossing and edge-touching cases.
- `store.rs`: round-trip a `Calibration` and a `Reading` through a temp
  `VNASWEEP_HOME`; slug collision; Touchstone text shape.
- `nanovna.rs`: no unit suite — verified by `just tauri-dev` against the
  real NanoVNA, then inspecting `~/vnasweep/`.
- Frontend: `npm run check` (svelte-check) clean; `npm test` green
  (`--passWithNoTests` until there are `src/lib/*.test.ts`).
- `just check` is the gate.

## 11. Open questions / risks

1. **Raw-sweep command sequence** (§6) — the main unknown. First task of the
   implementation plan is to nail it on firmware 1.2.27 with the device in
   hand.
2. **Sweep completion detection** — fixed delay vs poll. Affects reliability
   and how long a sweep feels.
3. **uPlot log-axis + band-shading plugin** — modest effort; fallback is a
   linear axis and simpler shading for the first cut.
4. **`export_reading` dialog** — needs `@tauri-apps/plugin-dialog` wired
   with a capability entry; small but easy to forget.
5. Leaving the standalone device in a sane state on disconnect (§6) — decide
   whether v1 bothers.

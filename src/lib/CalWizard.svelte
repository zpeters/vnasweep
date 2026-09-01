<script lang="ts">
  import { sweepRaw, saveCalibration } from "./api";
  import type { CalMeta, RawTrace } from "./types";

  let { ondone, oncancel }: { ondone: (m: CalMeta) => void; oncancel: () => void } = $props();

  let name = $state("");
  let notes = $state("");
  let startMhz = $state(1);
  let stopMhz = $state(30);
  let points = $state(101);

  const STEPS = ["open", "short", "load"] as const;
  let stepIdx = $state(-1); // -1 = params form; 0..2 = capture; 3 = solving
  let captured = $state<Partial<Record<(typeof STEPS)[number], RawTrace>>>({});
  let busy = $state(false);
  let error = $state<string | null>(null);

  const startHz = () => Math.round(startMhz * 1e6);
  const stopHz = () => Math.round(stopMhz * 1e6);

  function beginCapture() {
    error = null;
    if (!name.trim()) { error = "Give the calibration a name."; return; }
    if (stopHz() <= startHz()) { error = "Stop must be above start."; return; }
    if (points < 2 || points > 101) { error = "Points must be 2–101 (device limit)."; return; }
    stepIdx = 0;
  }

  async function captureCurrent() {
    busy = true; error = null;
    try {
      const which = STEPS[stepIdx];
      captured[which] = await sweepRaw(startHz(), stopHz(), points);
      if (stepIdx < 2) {
        stepIdx += 1;
      } else {
        stepIdx = 3;
        const meta = await saveCalibration(
          name, notes, startHz(), stopHz(), points,
          captured.open!, captured.short!, captured.load!,
        );
        ondone(meta);
      }
    } catch (e) {
      error = String(e);
      if (stepIdx === 3) stepIdx = 2; // let them retry the last capture
    } finally {
      busy = false;
    }
  }
</script>

<div class="wizard">
  {#if stepIdx === -1}
    <h3>New calibration</h3>
    <label>Name <input bind:value={name} placeholder="HF 1-30 MHz" /></label>
    <label>Notes <input bind:value={notes} placeholder="SMA kit, bench" /></label>
    <div class="row">
      <label>Start (MHz) <input type="number" bind:value={startMhz} min="0" step="0.1" /></label>
      <label>Stop (MHz) <input type="number" bind:value={stopMhz} min="0" step="0.1" /></label>
      <label>Points <input type="number" bind:value={points} min="2" max="101" /></label>
    </div>
    <div class="row">
      <button onclick={beginCapture}>Start</button>
      <button onclick={oncancel}>Cancel</button>
    </div>
  {:else if stepIdx >= 0 && stepIdx <= 2}
    <h3>Step {stepIdx + 1} of 3</h3>
    <p>Connect the <b>{STEPS[stepIdx].toUpperCase()}</b> standard, then capture.</p>
    <div class="row">
      <button onclick={captureCurrent} disabled={busy}>{busy ? "Sweeping…" : "Capture"}</button>
      <button onclick={oncancel} disabled={busy}>Cancel</button>
    </div>
  {:else}
    <p>Solving…</p>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .wizard { border: 1px solid #8884; border-radius: 6px; padding: 1rem; margin: 0.5rem 0; }
  label { display: block; margin: 0.3rem 0; }
  .row { display: flex; gap: 0.75rem; align-items: end; flex-wrap: wrap; }
  input[type="number"] { width: 7rem; }
  .error { color: #b00020; }
</style>

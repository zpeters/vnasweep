<script lang="ts">
  import { onMount } from "svelte";
  import type uPlot from "uplot";
  import { listCals, sweep, saveReading } from "./api";
  import type { CalMeta, Reading } from "./types";
  import UplotChart from "./UplotChart.svelte";
  import CalWizard from "./CalWizard.svelte";
  import { swrSeries, bandShadePlugin, minSwrMarkerPlugin } from "./plot";

  let cals = $state<CalMeta[]>([]);
  let selectedId = $state<string>("");
  let showWizard = $state(false);
  let reading = $state<Reading | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  let saveName = $state("");
  let saveNotes = $state("");
  let saved = $state<string | null>(null);

  const selectedCal = $derived(cals.find((c) => c.id === selectedId) ?? null);

  async function refreshCals() {
    cals = await listCals();
    if (!selectedId && cals.length) selectedId = cals[0].id;
  }
  onMount(refreshCals);

  function ageDays(iso: string): number {
    return Math.floor((Date.now() - Date.parse(iso)) / 86_400_000);
  }

  async function doSweep() {
    if (!selectedId) return;
    busy = true; error = null; saved = null;
    try {
      reading = await sweep(selectedId);
    } catch (e) {
      reading = null;
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function doSave() {
    if (!reading || !saveName.trim()) return;
    busy = true; error = null;
    try {
      const meta = await saveReading(reading, saveName, saveNotes);
      saved = meta.name;
      saveName = ""; saveNotes = "";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const plot = $derived.by(() => {
    if (!reading) return null;
    const { x, series } = swrSeries([{ label: reading.meta.name || "sweep", trace: reading.trace }]);
    const data: uPlot.AlignedData = [x, ...series.map((s) => s.y)];
    const logAxis = reading.meta.stop_hz / Math.max(1, reading.meta.start_hz) > 3;
    const opts: uPlot.Options = {
      width: 600,
      height: 360,
      scales: { x: { distr: (logAxis ? 3 : 1) as 1 | 3 }, y: { range: [1, 5] } },
      axes: [
        { label: "MHz", values: (_u, ticks) => ticks.map((t) => (t / 1e6).toFixed(3)) },
        { label: "SWR" },
      ],
      series: [
        {},
        { label: series[0]?.label ?? "SWR", stroke: "#1769ff", width: 2 },
      ],
      plugins: [bandShadePlugin(), minSwrMarkerPlugin(() => reading?.min_swr ?? null)],
    };
    return { data, opts };
  });
</script>

<section>
  <div class="row">
    <label>
      Calibration
      <select bind:value={selectedId}>
        {#each cals as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
      </select>
    </label>
    <button onclick={() => (showWizard = true)}>＋ New calibration</button>
  </div>

  {#if selectedCal}
    <p class="muted">
      {(selectedCal.start_hz / 1e6).toFixed(3)}–{(selectedCal.stop_hz / 1e6).toFixed(3)} MHz ·
      {selectedCal.points} pts · {ageDays(selectedCal.created)} d old
    </p>
  {/if}

  {#if showWizard}
    <CalWizard
      ondone={async (m) => { showWizard = false; await refreshCals(); selectedId = m.id; }}
      oncancel={() => (showWizard = false)}
    />
  {/if}

  <div class="row">
    <button onclick={doSweep} disabled={busy || !selectedId}>{busy ? "Sweeping…" : "Sweep"}</button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if plot}
    <UplotChart data={plot.data} opts={plot.opts} />
    {#if reading}
      <p>
        min SWR <b>{reading.min_swr.swr.toFixed(2)} : 1</b> @ {(reading.min_swr.freq_hz / 1e6).toFixed(3)} MHz
        {#if reading.bandwidth}
          · SWR ≤ {reading.bandwidth.threshold} from
          {(reading.bandwidth.lo_hz / 1e6).toFixed(3)}–{(reading.bandwidth.hi_hz / 1e6).toFixed(3)} MHz
        {/if}
      </p>
      <div class="row">
        <input bind:value={saveName} placeholder="reading name, e.g. 40m dipole v2" />
        <input bind:value={saveNotes} placeholder="notes" />
        <button onclick={doSave} disabled={busy || !saveName.trim()}>Save reading</button>
      </div>
      {#if saved}<p class="muted">saved "{saved}"</p>{/if}
    {/if}
  {/if}
</section>

<style>
  section { padding: 1rem; }
  .row { display: flex; gap: 0.75rem; align-items: end; flex-wrap: wrap; margin: 0.5rem 0; }
  .muted { color: #888; font-size: 0.9rem; }
  .error { color: #b00020; white-space: pre-wrap; }
</style>

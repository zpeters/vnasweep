<script lang="ts">
  import { onMount } from "svelte";
  import type uPlot from "uplot";
  import { listCals, sweep, saveReading } from "./api";
  import type { CalMeta, Reading, TracePoint } from "./types";
  import UplotChart from "./UplotChart.svelte";
  import CalWizard from "./CalWizard.svelte";
  import DevicePanel from "./DevicePanel.svelte";
  import { swrSeries, bandShadePlugin, minSwrMarkerPlugin } from "./plot";

  let cals = $state<CalMeta[]>([]);
  let selectedId = $state<string>("");
  let showWizard = $state(false);
  let reading = $state<Reading | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // Live sweep: re-run the sweep on a fixed gap after each one completes.
  // Never overlapping — the serial port is opened fresh per sweep and a
  // second concurrent open fails.
  let live = $state(false);
  let intervalS = $state(2);
  let threshold = $state(2);              // SWR ceiling for the bandwidth readout
  let lastSweepAt = $state<number | null>(null);
  let now = $state(Date.now());
  let liveTimer: ReturnType<typeof setTimeout> | undefined;
  let agoTimer: ReturnType<typeof setInterval> | undefined;

  // Transient on-plot comparison — not saved to disk.
  let reference = $state<{ label: string; trace: TracePoint[] } | null>(null);

  let saveName = $state("");
  let saveNotes = $state("");
  let saved = $state<string | null>(null);

  const selectedCal = $derived(cals.find((c) => c.id === selectedId) ?? null);
  const agoText = $derived(
    lastSweepAt == null ? "" : `${Math.max(0, Math.round((now - lastSweepAt) / 1000))}s ago`,
  );

  // Impedance + return loss at the min-SWR frequency.
  const markerPoint = $derived.by<TracePoint | null>(() => {
    if (!reading) return null;
    const target = reading.min_swr.freq_hz;
    let best: TracePoint | null = null;
    let bestDelta = Infinity;
    for (const p of reading.trace) {
      const d = Math.abs(p.freq_hz - target);
      if (d < bestDelta) { bestDelta = d; best = p; }
    }
    return best;
  });

  async function refreshCals() {
    try {
      cals = await listCals();
      if (!selectedId && cals.length) selectedId = cals[0].id;
    } catch (e) {
      error = String(e);
    }
  }
  onMount(() => {
    refreshCals();
    return () => stopLive();
  });

  function ageDays(iso: string): number {
    return Math.floor((Date.now() - Date.parse(iso)) / 86_400_000);
  }

  async function runSweep(): Promise<boolean> {
    if (!selectedId) return false;
    busy = true; error = null; saved = null;
    try {
      reading = await sweep(selectedId, threshold);
      lastSweepAt = Date.now();
      now = lastSweepAt;
      return true;
    } catch (e) {
      reading = null;
      error = String(e);
      return false;
    } finally {
      busy = false;
    }
  }

  async function doSweep() {
    if (live) return;
    await runSweep();
  }

  function startLive() {
    if (!selectedId || live) return;
    live = true;
    agoTimer = setInterval(() => (now = Date.now()), 1000);
    tick();
  }

  function stopLive() {
    live = false;
    clearTimeout(liveTimer); liveTimer = undefined;
    clearInterval(agoTimer); agoTimer = undefined;
  }

  async function tick() {
    if (!live) return;
    const ok = await runSweep();
    if (!ok) { stopLive(); return; }        // don't hammer a failing device
    if (!live) return;                       // stopped while the sweep was in flight
    liveTimer = setTimeout(tick, Math.max(1, intervalS) * 1000);
  }

  function toggleLive() {
    if (live) stopLive();
    else startLive();
  }

  // Changing the calibration mid-run invalidates the point grid — stop.
  function onCalChange() {
    stopLive();
    reference = null;
  }

  function keepAsReference() {
    if (!reading) return;
    reference = {
      label: `${reading.meta.name || "ref"} (ref)`,
      trace: reading.trace,
    };
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
    const labeled = [
      ...(reference ? [{ label: reference.label, trace: reference.trace }] : []),
      { label: reading.meta.name || "sweep", trace: reading.trace },
    ];
    const { x, series } = swrSeries(labeled);
    const data: uPlot.AlignedData = [x, ...series.map((s) => s.y)];
    const logAxis = reading.meta.stop_hz / Math.max(1, reading.meta.start_hz) > 3;
    const hasRef = reference != null;
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
        ...series.map((s, i) => ({
          label: s.label,
          stroke: hasRef && i === 0 ? "#94a3b8" : "#1769ff",
          width: 2,
          dash: hasRef && i === 0 ? [6, 4] : undefined,
        })),
      ],
      plugins: [bandShadePlugin(), minSwrMarkerPlugin(() => reading?.min_swr ?? null)],
    };
    return { data, opts };
  });

  const deviceMissing = $derived(
    error != null && /no .*nanovna|device|serial port|not found/i.test(error),
  );
</script>

<section>
  <DevicePanel />

  <div class="row">
    <label>
      Calibration
      <select bind:value={selectedId} onchange={onCalChange}>
        {#each cals as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
      </select>
    </label>
    <button onclick={() => (showWizard = true)}>＋ New calibration</button>
  </div>

  {#if cals.length === 0}
    <p class="muted">No calibrations yet. Run <b>＋ New calibration</b> to capture Open / Short / Load, then sweep.</p>
  {/if}

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
    <button onclick={doSweep} disabled={busy || live || !selectedId}>
      {busy && !live ? "Sweeping…" : "Sweep"}
    </button>
    <button onclick={toggleLive} disabled={!selectedId} class:live>
      {live ? "■ Stop" : "▶ Live"}
    </button>
    <label>
      every
      <select bind:value={intervalS} disabled={live}>
        <option value={2}>2 s</option>
        <option value={5}>5 s</option>
        <option value={10}>10 s</option>
      </select>
    </label>
    <label>
      SWR ≤
      <select bind:value={threshold}>
        <option value={1.5}>1.5</option>
        <option value={2}>2.0</option>
        <option value={3}>3.0</option>
      </select>
    </label>
    {#if live}
      <span class="badge"><span class="dot"></span> live · updated {agoText}</span>
    {/if}
  </div>

  {#if error}
    <p class="error">{error}</p>
    {#if deviceMissing}
      <p class="muted">Check the NanoVNA-H is plugged in over USB and no other program holds the port.</p>
    {/if}
  {/if}

  {#if plot}
    <UplotChart data={plot.data} opts={plot.opts} />
    {#if reading}
      <p>
        min SWR <b>{reading.min_swr.swr.toFixed(2)} : 1</b> @ {(reading.min_swr.freq_hz / 1e6).toFixed(3)} MHz
        {#if markerPoint}
          · Z ≈ {markerPoint.r.toFixed(1)} {markerPoint.x >= 0 ? "+" : "−"} j{Math.abs(markerPoint.x).toFixed(1)} Ω
          · RL {markerPoint.return_loss_db.toFixed(1)} dB
        {/if}
        {#if reading.bandwidth}
          · SWR ≤ {reading.bandwidth.threshold} from
          {(reading.bandwidth.lo_hz / 1e6).toFixed(3)}–{(reading.bandwidth.hi_hz / 1e6).toFixed(3)} MHz
        {/if}
      </p>
      <div class="row">
        <button onclick={keepAsReference}>Keep as reference</button>
        {#if reference}
          <span class="muted">ref: {reference.label}</span>
          <button onclick={() => (reference = null)}>Clear reference</button>
        {/if}
      </div>
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
  button.live { background: #b00020; color: #fff; border-color: #b00020; }
  .badge { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.9rem; color: #666; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%; background: #b00020;
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.25; } }
</style>

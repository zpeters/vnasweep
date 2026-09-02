<script lang="ts">
  import { onMount } from "svelte";
  import type uPlot from "uplot";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    listReadings, getReading, renameReading, deleteReading, exportReading, exportReadingCsv,
    listCals, renameCal, deleteCal, calUsage,
  } from "./api";
  import type { CalMeta, Reading, ReadingMeta } from "./types";
  import UplotChart from "./UplotChart.svelte";
  import { swrSeries, bandShadePlugin } from "./plot";

  let readings = $state<ReadingMeta[]>([]);
  let cals = $state<CalMeta[]>([]);
  let checked = $state<Set<string>>(new Set());
  let loaded = $state<Map<string, Reading>>(new Map());
  let error = $state<string | null>(null);
  let showCals = $state(false);
  let showDelta = $state(false);         // only meaningful with exactly 2 ticked

  async function refresh() {
    try {
      readings = await listReadings();
      cals = await listCals();
    } catch (e) {
      error = String(e);
    }
  }
  onMount(refresh);

  async function toggle(id: string) {
    error = null;
    const next = new Set(checked);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
      if (!loaded.has(id)) {
        try {
          loaded = new Map(loaded).set(id, await getReading(id));
        } catch (e) {
          error = String(e);
          next.delete(id);
        }
      }
    }
    checked = next;
  }

  async function doRename(id: string, current: string) {
    error = null;
    const name = prompt("Rename reading", current);
    if (name && name.trim()) {
      try {
        await renameReading(id, name.trim());
        const m = new Map(loaded); m.delete(id); loaded = m;
        const c = new Set(checked); c.delete(id); checked = c;
        await refresh();
      } catch (e) {
        error = String(e);
      }
    }
  }
  async function doDelete(id: string) {
    error = null;
    if (confirm("Delete this reading? The .s1p and sidecar are removed.")) {
      try {
        await deleteReading(id);
        const c = new Set(checked); c.delete(id); checked = c;
        const m = new Map(loaded); m.delete(id); loaded = m;
        await refresh();
      } catch (e) {
        error = String(e);
      }
    }
  }
  async function doExport(id: string) {
    error = null;
    try {
      const dest = await save({ defaultPath: `${id}.s1p`, filters: [{ name: "Touchstone", extensions: ["s1p"] }] });
      if (dest) { await exportReading(id, dest); }
    } catch (e) {
      error = String(e);
    }
  }
  async function doExportCsv(id: string) {
    error = null;
    try {
      const dest = await save({ defaultPath: `${id}.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
      if (dest) { await exportReadingCsv(id, dest); }
    } catch (e) {
      error = String(e);
    }
  }
  async function doRenameCal(id: string, current: string) {
    error = null;
    const name = prompt("Rename calibration", current);
    if (name && name.trim()) {
      try {
        await renameCal(id, name.trim());
        await refresh();
      } catch (e) {
        error = String(e);
      }
    }
  }
  async function doDeleteCal(id: string) {
    error = null;
    try {
      const n = await calUsage(id);
      const msg = n > 0
        ? `${n} reading(s) reference this calibration. They'll keep working but lose the cal name. Delete anyway?`
        : "Delete this calibration?";
      if (confirm(msg)) { await deleteCal(id); await refresh(); }
    } catch (e) {
      error = String(e);
    }
  }

  const single = $derived(checked.size === 1 ? loaded.get([...checked][0]) ?? null : null);
  const canDelta = $derived(checked.size === 2);

  const plot = $derived.by(() => {
    const picked = [...checked]
      .map((id) => loaded.get(id))
      .filter((r): r is Reading => !!r);
    if (!picked.length) return null;
    const span0 = picked[0].meta.stop_hz / Math.max(1, picked[0].meta.start_hz);
    const distr = (span0 > 3 ? 3 : 1) as 1 | 3;

    // Difference view: SWR(B) − SWR(A) on the shared frequency grid.
    if (showDelta && picked.length === 2) {
      const [a, b] = picked;
      const { x, series } = swrSeries([
        { label: a.meta.name, trace: a.trace },
        { label: b.meta.name, trace: b.trace },
      ]);
      const dy = x.map((_, i) => {
        const ya = series[0].y[i];
        const yb = series[1].y[i];
        return ya == null || yb == null ? null : yb - ya;
      });
      const data: uPlot.AlignedData = [x, dy];
      const opts: uPlot.Options = {
        width: 600,
        height: 360,
        scales: { x: { distr }, y: {} },
        axes: [
          { label: "MHz", values: (_u, ticks) => ticks.map((t) => (t / 1e6).toFixed(3)) },
          { label: "Δ SWR" },
        ],
        series: [{}, { label: `Δ SWR (${b.meta.name} − ${a.meta.name})`, stroke: "#e0245e", width: 2 }],
        plugins: [bandShadePlugin()],
      };
      return { data, opts };
    }

    const traces = picked.map((r) => ({ label: r.meta.name, trace: r.trace }));
    const { x, series } = swrSeries(traces);
    const data: uPlot.AlignedData = [x, ...series.map((s) => s.y)];
    const palette = ["#1769ff", "#e0245e", "#17bf63", "#f45d22", "#794bc4"];
    const opts: uPlot.Options = {
      width: 600,
      height: 360,
      scales: {
        x: { distr },
        y: { range: (_u, _min, max) => [1, Math.max(5, Math.min(100, Math.ceil(max * 1.05)))] },
      },
      axes: [
        { label: "MHz", values: (_u, ticks) => ticks.map((t) => (t / 1e6).toFixed(3)) },
        { label: "SWR" },
      ],
      series: [{}, ...series.map((s, i) => ({ label: s.label, stroke: palette[i % palette.length], width: 2 }))],
      plugins: [bandShadePlugin()],
    };
    return { data, opts };
  });
</script>

<section>
  <div class="grid">
    <ul class="list">
      {#each readings as r (r.id)}
        <li>
          <label>
            <input type="checkbox" checked={checked.has(r.id)} onchange={() => toggle(r.id)} />
            <span class="name">{r.name}</span>
            <span class="muted">{new Date(r.created).toLocaleDateString()} · {r.cal_name}</span>
          </label>
          <span class="actions">
            <button onclick={() => doRename(r.id, r.name)}>rename</button>
            <button onclick={() => doExport(r.id)}>.s1p</button>
            <button onclick={() => doExportCsv(r.id)}>.csv</button>
            <button onclick={() => doDelete(r.id)}>delete</button>
          </span>
        </li>
      {/each}
      {#if !readings.length}<li class="muted">No saved readings yet.</li>{/if}
    </ul>

    <div class="pane">
      {#if error}<p class="error">{error}</p>{/if}
      {#if canDelta}
        <label class="muted">
          <input type="checkbox" bind:checked={showDelta} /> show difference (B − A)
        </label>
      {/if}
      {#if plot}
        <UplotChart data={plot.data} opts={plot.opts} />
      {:else}
        <p class="muted">Tick one or more readings to plot.</p>
      {/if}
      {#if single}
        <p>
          <b>{single.meta.name}</b> — min SWR {single.min_swr.swr.toFixed(2)} : 1 @
          {(single.min_swr.freq_hz / 1e6).toFixed(3)} MHz
          {#if single.bandwidth}
            · SWR ≤ {single.bandwidth.threshold} from
            {(single.bandwidth.lo_hz / 1e6).toFixed(3)}–{(single.bandwidth.hi_hz / 1e6).toFixed(3)} MHz
          {/if}
          <br /><span class="muted">{single.meta.notes}</span>
        </p>
      {/if}
    </div>
  </div>

  <details bind:open={showCals}>
    <summary>Calibrations ({cals.length})</summary>
    <ul class="list">
      {#each cals as c (c.id)}
        <li>
          <span class="name">{c.name}</span>
          <span class="muted">{(c.start_hz / 1e6).toFixed(3)}–{(c.stop_hz / 1e6).toFixed(3)} MHz · {c.points} pts</span>
          <span class="actions">
            <button onclick={() => doRenameCal(c.id, c.name)}>rename</button>
            <button onclick={() => doDeleteCal(c.id)}>delete</button>
          </span>
        </li>
      {/each}
    </ul>
  </details>
</section>

<style>
  section { padding: 1rem; }
  .grid { display: grid; grid-template-columns: 20rem 1fr; gap: 1rem; align-items: start; }
  .list { list-style: none; padding: 0; margin: 0; }
  .list li { display: flex; justify-content: space-between; gap: 0.5rem; padding: 0.25rem 0; border-bottom: 1px solid #8882; }
  .name { font-weight: 600; }
  .muted { color: #888; font-size: 0.85rem; }
  .actions button { font-size: 0.8rem; margin-left: 0.25rem; }
  .error { color: #b00020; white-space: pre-wrap; }
</style>

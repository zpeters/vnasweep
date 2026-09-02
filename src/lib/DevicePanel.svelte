<script lang="ts">
  import { onMount } from "svelte";
  import { listSerialPorts, setSerialPort, getSerialPort, boardInfo, deviceSweepConfig } from "./api";
  import type { PortInfo } from "./types";

  let ports = $state<PortInfo[]>([]);
  let chosen = $state<string>("");        // "" = autodetect
  let info = $state<string | null>(null);
  let cfg = $state<string | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function refreshPorts() {
    error = null;
    try {
      ports = await listSerialPorts();
      chosen = (await getSerialPort()) ?? "";
    } catch (e) {
      error = String(e);
    }
  }
  onMount(refreshPorts);

  async function choose() {
    error = null; info = null; cfg = null;
    try {
      await setSerialPort(chosen || null);
    } catch (e) {
      error = String(e);
    }
  }

  async function identify() {
    busy = true; error = null; info = null;
    try {
      info = await boardInfo();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function readConfig() {
    busy = true; error = null; cfg = null;
    try {
      const c = await deviceSweepConfig();
      cfg = `${(c.start_hz / 1e6).toFixed(3)}–${(c.stop_hz / 1e6).toFixed(3)} MHz · ${c.points} pts`;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<details class="device">
  <summary>Device{chosen ? ` · ${chosen}` : " · auto-detect"}</summary>

  <div class="row">
    <label>
      Port
      <select bind:value={chosen} onchange={choose}>
        <option value="">auto-detect</option>
        {#each ports as p (p.name)}
          <option value={p.name}>{p.is_nanovna ? "★ " : ""}{p.name} — {p.description}</option>
        {/each}
      </select>
    </label>
    <button onclick={refreshPorts}>Refresh</button>
    <button onclick={identify} disabled={busy}>Identify</button>
    <button onclick={readConfig} disabled={busy}>Read device config</button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}
  {#if info}<pre class="info">{info}</pre>{/if}
  {#if cfg}<p class="muted">device programmed for {cfg}</p>{/if}
</details>

<style>
  .device { margin: 0.5rem 0 0.75rem; padding: 0.25rem 0; border-bottom: 1px solid #8882; }
  summary { cursor: pointer; color: #666; font-size: 0.9rem; }
  .row { display: flex; gap: 0.75rem; align-items: end; flex-wrap: wrap; margin: 0.5rem 0; }
  .muted { color: #888; font-size: 0.9rem; }
  .error { color: #b00020; white-space: pre-wrap; }
  .info { background: #8881; padding: 0.5rem; border-radius: 4px; font-size: 0.85rem; overflow-x: auto; }
</style>

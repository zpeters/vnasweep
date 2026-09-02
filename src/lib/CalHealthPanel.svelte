<script lang="ts">
  import { calHealth } from "./api";
  import type { CalHealth } from "./types";

  let { calId }: { calId: string } = $props();

  let health = $state<CalHealth | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const id = calId;
    health = null;
    error = null;
    if (!id) return;
    calHealth(id)
      .then((h) => { if (id === calId) health = h; })
      .catch((e) => { if (id === calId) error = String(e); });
  });
</script>

{#if error}
  <p class="err">calibration health: {error}</p>
{:else if health}
  {#if health.warnings.length}
    <ul class="warn">
      {#each health.warnings as w}<li>⚠ {w}</li>{/each}
    </ul>
  {:else}
    <p class="ok">✓ calibration looks healthy</p>
  {/if}
  <table>
    <tbody>
      <tr>
        <td>Open |Γ|</td><td>{health.open_mag.mean.toFixed(3)}</td>
        <td>Short |Γ|</td><td>{health.short_mag.mean.toFixed(3)}</td>
        <td>Load |Γ|</td><td>{health.load_mag.mean.toFixed(3)}</td>
      </tr>
      <tr>
        <td>O↔S phase</td><td>{health.open_short_phase_deg.toFixed(0)}°</td>
        <td>directivity</td><td>{health.directivity_mag.toFixed(3)}</td>
        <td>tracking</td><td>{health.tracking_mag.toFixed(3)}</td>
      </tr>
    </tbody>
  </table>
{/if}

<style>
  .warn { margin: 0.25rem 0; padding-left: 1.1rem; color: #b00020; font-size: 0.85rem; }
  .ok { margin: 0.25rem 0; color: #17772f; font-size: 0.85rem; }
  .err { margin: 0.25rem 0; color: #b00020; font-size: 0.85rem; }
  table { border-collapse: collapse; font-size: 0.8rem; color: #666; margin: 0.25rem 0 0.5rem; }
  td { padding: 0.1rem 0.5rem 0.1rem 0; }
  td:nth-child(odd) { color: #999; }
</style>

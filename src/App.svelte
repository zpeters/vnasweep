<script lang="ts">
  import MeasureTab from "./lib/MeasureTab.svelte";
  import LibraryTab from "./lib/LibraryTab.svelte";

  let tab = $state<"measure" | "library">("measure");
</script>

<header>
  <h1>vnasweep</h1>
  <nav>
    <button class:active={tab === "measure"} onclick={() => (tab = "measure")}>Measure</button>
    <button class:active={tab === "library"} onclick={() => (tab = "library")}>Library</button>
  </nav>
</header>

{#if tab === "measure"}
  <MeasureTab />
{:else}
  <LibraryTab />
{/if}

<style>
  :global(body) { margin: 0; font-family: system-ui, sans-serif; }
  header { display: flex; align-items: baseline; gap: 1.5rem; padding: 0.75rem 1rem; border-bottom: 1px solid #8884; }
  h1 { font-size: 1.1rem; margin: 0; }
  nav button { font: inherit; border: none; background: none; padding: 0.4rem 0.6rem; cursor: pointer; color: #888; }
  nav button.active { color: inherit; border-bottom: 2px solid currentColor; }
</style>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type BoardInfo = { port: string; version: string; info: string };

  let board = $state<BoardInfo | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function read() {
    loading = true;
    error = null;
    try {
      board = await invoke<BoardInfo>("board_info");
    } catch (e) {
      board = null;
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<main>
  <h1>vnasweep</h1>

  <button onclick={read} disabled={loading}>
    {loading ? "Reading…" : "Read board info"}
  </button>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if board}
    <dl>
      <dt>Port</dt>
      <dd>{board.port}</dd>
      <dt>Version</dt>
      <dd>{board.version}</dd>
    </dl>
    <pre>{board.info}</pre>
  {/if}
</main>

<style>
  main {
    font-family: system-ui, sans-serif;
    max-width: 40rem;
    margin: 0 auto;
    padding: 1.5rem;
  }
  h1 {
    font-size: 1.25rem;
    margin: 0 0 1rem;
  }
  button {
    font: inherit;
    padding: 0.4rem 0.9rem;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.2rem 1rem;
    margin: 1rem 0 0.5rem;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin: 0;
  }
  pre {
    background: #f4f4f4;
    padding: 0.75rem;
    border-radius: 4px;
    overflow-x: auto;
    white-space: pre-wrap;
  }
  .error {
    color: #b00020;
    white-space: pre-wrap;
  }
  @media (prefers-color-scheme: dark) {
    pre {
      background: #1e1e1e;
    }
    .error {
      color: #ff6b6b;
    }
  }
</style>

<script lang="ts">
  import { onMount } from "svelte";
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";

  let { data, opts }: { data: uPlot.AlignedData; opts: uPlot.Options } = $props();

  let el: HTMLDivElement;
  let chart: uPlot | undefined;

  onMount(() => {
    chart = new uPlot({ ...opts, width: el.clientWidth || 600, height: opts.height ?? 340 }, data, el);
    const ro = new ResizeObserver(() => {
      if (chart) chart.setSize({ width: el.clientWidth, height: opts.height ?? 340 });
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
      chart?.destroy();
      chart = undefined;
    };
  });

  $effect(() => {
    // re-create on opts change (series count / plugins), just setData on data change
    if (chart && data) chart.setData(data);
  });
</script>

<div bind:this={el} class="chart"></div>

<style>
  .chart { width: 100%; }
</style>

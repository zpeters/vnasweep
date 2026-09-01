<script lang="ts">
  import { onMount } from "svelte";
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";

  let { data, opts }: { data: uPlot.AlignedData; opts: uPlot.Options } = $props();

  let el: HTMLDivElement;
  let chart: uPlot | undefined;
  let ro: ResizeObserver | undefined;

  function build() {
    chart?.destroy();
    chart = new uPlot(
      { ...opts, width: el.clientWidth || 600, height: opts.height ?? 340 },
      data,
      el,
    );
  }

  onMount(() => {
    build();
    ro = new ResizeObserver(() => {
      chart?.setSize({ width: el.clientWidth, height: opts.height ?? 340 });
    });
    ro.observe(el);
    return () => {
      ro?.disconnect();
      chart?.destroy();
      chart = undefined;
    };
  });

  // Rebuild when opts identity changes (series count / plugins / scales);
  // otherwise just push new data.
  let lastOpts: uPlot.Options | undefined;
  $effect(() => {
    if (!chart) return;
    if (opts !== lastOpts) {
      lastOpts = opts;
      build();
    } else {
      chart.setData(data);
    }
  });
</script>

<div bind:this={el} class="chart"></div>

<style>
  .chart { width: 100%; }
</style>

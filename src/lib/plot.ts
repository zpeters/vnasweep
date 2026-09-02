import type uPlot from "uplot";
import type { SwrPoint, TracePoint } from "./types";
import { HAM_BANDS } from "./bands";

export interface LabeledTrace { label: string; trace: TracePoint[]; }

/** SWR y-axis ceiling: the next "nice" step at or above the data max, so the
 *  trace always fits and the axis stays put across live updates. Floor is
 *  always 1 (SWR can't go lower). */
export function niceSwrMax(dataMax: number): number {
  for (const step of [1.5, 2, 3, 5, 10, 20, 50, 100]) {
    if (dataMax <= step) return step;
  }
  return Math.ceil(dataMax / 100) * 100;
}

/** Merge traces onto the union of their frequencies; a series carries `null`
 *  where it has no sample (uPlot draws a gap). No interpolation. */
export function swrSeries(traces: LabeledTrace[]): {
  x: number[];
  series: { label: string; y: (number | null)[] }[];
} {
  const freqSet = new Set<number>();
  for (const t of traces) for (const p of t.trace) freqSet.add(p.freq_hz);
  const x = [...freqSet].sort((a, b) => a - b);
  const index = new Map(x.map((f, i) => [f, i]));

  const series = traces.map((t) => {
    const y: (number | null)[] = new Array(x.length).fill(null);
    for (const p of t.trace) y[index.get(p.freq_hz)!] = p.swr;
    return { label: t.label, y };
  });
  return { x, series };
}

const BAND_FILL = "rgba(80,140,255,0.10)";

/** Shade amateur bands that fall within the current x-range. */
export function bandShadePlugin(): uPlot.Plugin {
  return {
    hooks: {
      draw: (u: uPlot) => {
        const { ctx } = u;
        const yTop = u.bbox.top;
        const yBot = u.bbox.top + u.bbox.height;
        ctx.save();
        ctx.fillStyle = BAND_FILL;
        for (const b of HAM_BANDS) {
          const x0 = u.valToPos(b.lo, "x", true);
          const x1 = u.valToPos(b.hi, "x", true);
          if (x1 < u.bbox.left || x0 > u.bbox.left + u.bbox.width) continue;
          ctx.fillRect(x0, yTop, Math.max(1, x1 - x0), yBot - yTop);
        }
        ctx.restore();
      },
    },
  };
}

/** Draw a marker + label at the minimum-SWR point. */
export function minSwrMarkerPlugin(getMin: () => SwrPoint | null): uPlot.Plugin {
  return {
    hooks: {
      draw: (u: uPlot) => {
        const m = getMin();
        if (!m || m.swr >= 999) return;
        const { ctx } = u;
        const px = u.valToPos(m.freq_hz, "x", true);
        const py = u.valToPos(m.swr, "y", true);
        ctx.save();
        ctx.fillStyle = "#e0245e";
        ctx.beginPath();
        ctx.arc(px, py, 4, 0, Math.PI * 2);
        ctx.fill();
        ctx.font = "12px system-ui, sans-serif";
        ctx.fillText(
          `${m.swr.toFixed(2)} : 1  @ ${(m.freq_hz / 1e6).toFixed(3)} MHz`,
          px + 8,
          py - 8,
        );
        ctx.restore();
      },
    },
  };
}

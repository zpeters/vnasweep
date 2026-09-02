export interface CalMeta {
  id: string;
  name: string;
  notes: string;
  created: string;
  start_hz: number;
  stop_hz: number;
  points: number;
}

export interface RawPoint { freq_hz: number; re: number; im: number; }
export interface RawTrace { points: RawPoint[]; }
export interface Standards { open: RawTrace; short: RawTrace; load: RawTrace; }
export interface ErrorTerms { e00: [number, number][]; e11: [number, number][]; e10e01: [number, number][]; }
export interface Calibration { meta: CalMeta; standards: Standards; error_terms: ErrorTerms; }

export interface TracePoint {
  freq_hz: number;
  gamma_re: number;
  gamma_im: number;
  swr: number;
  r: number;
  x: number;
  return_loss_db: number;
}

export interface SwrPoint { freq_hz: number; swr: number; }
export interface Band { lo_hz: number; hi_hz: number; threshold: number; }

export interface ReadingMeta {
  id: string;
  name: string;
  notes: string;
  created: string;
  cal_id: string;
  cal_name: string;
  start_hz: number;
  stop_hz: number;
  points: number;
}

export interface Reading {
  meta: ReadingMeta;
  trace: TracePoint[];
  min_swr: SwrPoint;
  bandwidth: Band | null;
}

export interface PortInfo { name: string; description: string; is_nanovna: boolean; }
export interface SweepConfig { start_hz: number; stop_hz: number; points: number; }

export interface MagStats { mean: number; min: number; max: number; }
export interface CalHealth {
  open_mag: MagStats;
  short_mag: MagStats;
  load_mag: MagStats;
  open_short_phase_deg: number;
  directivity_mag: number;
  source_match_mag: number;
  tracking_mag: number;
  warnings: string[];
}

export interface DiagReading {
  cal_id: string;
  cal_name: string;
  start_hz: number;
  stop_hz: number;
  points: number;
  ours: TracePoint[];
  raw: TracePoint[];
  device: TracePoint[];
}

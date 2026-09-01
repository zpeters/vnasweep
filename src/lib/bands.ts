// Amateur band edges in Hz (IARU R2 / US general coverage, coarse — for axis shading only).
export interface HamBand { name: string; lo: number; hi: number; }

export const HAM_BANDS: HamBand[] = [
  { name: "160m", lo: 1_800_000, hi: 2_000_000 },
  { name: "80m", lo: 3_500_000, hi: 4_000_000 },
  { name: "60m", lo: 5_330_500, hi: 5_406_400 },
  { name: "40m", lo: 7_000_000, hi: 7_300_000 },
  { name: "30m", lo: 10_100_000, hi: 10_150_000 },
  { name: "20m", lo: 14_000_000, hi: 14_350_000 },
  { name: "17m", lo: 18_068_000, hi: 18_168_000 },
  { name: "15m", lo: 21_000_000, hi: 21_450_000 },
  { name: "12m", lo: 24_890_000, hi: 24_990_000 },
  { name: "10m", lo: 28_000_000, hi: 29_700_000 },
  { name: "6m", lo: 50_000_000, hi: 54_000_000 },
  { name: "2m", lo: 144_000_000, hi: 148_000_000 },
  { name: "1.25m", lo: 222_000_000, hi: 225_000_000 },
  { name: "70cm", lo: 420_000_000, hi: 450_000_000 },
];

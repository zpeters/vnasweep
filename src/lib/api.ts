import { invoke } from "@tauri-apps/api/core";
import type {
  CalMeta, Calibration, RawTrace, Reading, ReadingMeta,
} from "./types";

export const listCals = () => invoke<CalMeta[]>("list_cals");
export const getCal = (id: string) => invoke<Calibration>("get_cal", { id });
export const sweepRaw = (start_hz: number, stop_hz: number, points: number) =>
  invoke<RawTrace>("sweep_raw", { start_hz, stop_hz, points });
export const saveCalibration = (
  name: string, notes: string,
  start_hz: number, stop_hz: number, points: number,
  open: RawTrace, short: RawTrace, load: RawTrace,
) => invoke<CalMeta>("save_calibration", { name, notes, start_hz, stop_hz, points, open, short, load });
export const renameCal = (id: string, name: string) => invoke<CalMeta>("rename_cal", { id, name });
export const deleteCal = (id: string) => invoke<void>("delete_cal", { id });
export const calUsage = (id: string) => invoke<number>("cal_usage", { id });

export const sweep = (cal_id: string) => invoke<Reading>("sweep", { cal_id });
export const saveReading = (reading: Reading, name: string, notes: string) =>
  invoke<ReadingMeta>("save_reading", { reading, name, notes });
export const listReadings = () => invoke<ReadingMeta[]>("list_readings");
export const getReading = (id: string) => invoke<Reading>("get_reading", { id });
export const renameReading = (id: string, name: string) => invoke<ReadingMeta>("rename_reading", { id, name });
export const deleteReading = (id: string) => invoke<void>("delete_reading", { id });
export const exportReading = (id: string, dest_path: string) =>
  invoke<void>("export_reading", { id, dest_path });

import { invoke } from "@tauri-apps/api/core";
import type {
  CalHealth, CalMeta, Calibration, DiagReading, PortInfo, RawTrace, Reading,
  ReadingMeta, SweepConfig,
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

export const sweep = (cal_id: string, threshold?: number) =>
  invoke<Reading>("sweep", { cal_id, threshold });
export const calHealth = (id: string) => invoke<CalHealth>("cal_health", { id });
export const sweepDiag = (cal_id: string) => invoke<DiagReading>("sweep_diag", { cal_id });
export const saveReading = (reading: Reading, name: string, notes: string) =>
  invoke<ReadingMeta>("save_reading", { reading, name, notes });
export const listReadings = () => invoke<ReadingMeta[]>("list_readings");
export const getReading = (id: string) => invoke<Reading>("get_reading", { id });
export const renameReading = (id: string, name: string) => invoke<ReadingMeta>("rename_reading", { id, name });
export const deleteReading = (id: string) => invoke<void>("delete_reading", { id });
export const exportReading = (id: string, dest_path: string) =>
  invoke<void>("export_reading", { id, dest_path });
export const exportReadingCsv = (id: string, dest_path: string) =>
  invoke<void>("export_reading_csv", { id, dest_path });

export const listSerialPorts = () => invoke<PortInfo[]>("list_serial_ports");
export const setSerialPort = (port: string | null) => invoke<void>("set_serial_port", { port });
export const getSerialPort = () => invoke<string | null>("get_serial_port");
export const boardInfo = () => invoke<string>("board_info");
export const deviceSweepConfig = () => invoke<SweepConfig>("device_sweep_config");

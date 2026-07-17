// Thin typed wrappers around the Tauri backend commands.

import { invoke } from "@tauri-apps/api/core";
import type { FolderReport, SavedReport, SpectroSummary } from "./types";

export const analyzePaths = (targets: string[]) =>
  invoke<FolderReport>("analyze_paths", { targets });

export const generateSpectrograms = (targets: string[]) =>
  invoke<SpectroSummary>("generate_spectrograms", { targets });

// Writes both the CSV and JSON reports (same stem/folder as `dest`).
export const saveReport = (dest: string, report: FolderReport) =>
  invoke<SavedReport>("save_report", { dest, report });

// Re-imports a previously-saved JSON report (dropped onto the window).
export const loadReport = (path: string) =>
  invoke<FolderReport>("load_report", { path });

export const ffmpegAvailable = () => invoke<boolean>("ffmpeg_available");

export const cancelTask = () => invoke("cancel_task");

export const revealInFolder = (path: string) =>
  invoke("reveal_in_folder", { path });

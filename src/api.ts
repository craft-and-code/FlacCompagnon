// Thin typed wrappers around the Tauri backend commands.

import { invoke } from "@tauri-apps/api/core";
import type { FolderReport, SpectroSummary } from "./types";

export const analyzePaths = (targets: string[]) =>
  invoke<FolderReport>("analyze_paths", { targets });

export const generateSpectrograms = (targets: string[]) =>
  invoke<SpectroSummary>("generate_spectrograms", { targets });

export const saveCsv = (dest: string, report: FolderReport) =>
  invoke<string>("save_csv", { dest, report });

export const ffmpegAvailable = () => invoke<boolean>("ffmpeg_available");

export const cancelTask = () => invoke("cancel_task");

export const revealInFolder = (path: string) =>
  invoke("reveal_in_folder", { path });

// Types mirroring the Rust `serde` payloads exchanged with the Tauri backend.

export type TranscodeState = "none" | "suspected" | "detected";

export interface Detections {
  upscaling: boolean;
  upsampling: boolean;
  transcoding: TranscodeState;
  detail: string;
  summary: string;
}

export interface ClippingInfo {
  clipped_samples: number;
  clip_events: number;
  peak: number;
  peak_dbfs: number;
  clipped: boolean;
}

export type FlacMd5Status =
  | { state: "NoSignature" }
  | { state: "Present" }
  | { state: "Match" }
  | { state: "Mismatch" }
  | { state: "Error"; detail: string };

export interface FileAnalysis {
  path: string;
  file_name: string;
  format: string;
  ext_mismatch: boolean;
  sample_rate: number;
  channels: number;
  declared_bits: number | null;
  duration_secs: number;
  detections: Detections;
  cutoff_hz: number | null;
  cutoff_ratio: number | null;
  real_bit_depth: number | null;
  fake_stereo: boolean | null;
  clipping: ClippingInfo;
  flac_md5: FlacMd5Status | null;
  error: string | null;
}

export interface FolderReport {
  root: string;
  files: FileAnalysis[];
  has_flac: boolean;
}

export interface Progress {
  current: number;
  total: number;
  file: string;
}

export interface SpectroSummary {
  total: number;
  rendered: number;
  failed: number;
  spectres_dirs: string[];
  errors: string[];
}

export type Theme = "auto" | "light" | "dark";

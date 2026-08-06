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
  true_peak: number;
  true_peak_dbtp: number;
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
  // On-disk size in bytes, read from the filesystem by the Rust side so it
  // matches what the OS reports (never derived from bitrate × duration).
  size_bytes: number;
  detections: Detections;
  cutoff_hz: number | null;
  cutoff_ratio: number | null;
  real_bit_depth: number | null;
  requant_rate: number | null;
  fake_stereo: boolean | null;
  badge: string | null;
  clipping: ClippingInfo;
  dr_db: number | null;
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

// --- Tags (editor panel + thumbnail column) ----------------------------------

export interface CoverArt {
  mime: string;
  width: number;
  height: number;
  size_bytes: number;
  // Picture role from the tag ("CoverFront", "CoverBack", "Other", ...) —
  // localized for display by the frontend.
  picture_type: string;
  data_base64: string;
}

export interface TagSet {
  title: string | null;
  artist: string | null;
  album: string | null;
  album_artist: string | null;
  composer: string | null;
  year: string | null;
  track: string | null;
  track_total: string | null;
  disc: string | null;
  disc_total: string | null;
  genre: string | null;
  comment: string | null;
  compilation: boolean;
  extra: [string, string][];
  cover: CoverArt | null;
  // MusicBrainz Release ID already in the file's tags (e.g. from Picard), if
  // any — lets "Search online" skip straight to that exact release.
  musicbrainz_release_id: string | null;
}

export interface TagReadResult {
  path: string;
  tags: TagSet | null;
  error: string | null;
}

// Rust's `FieldEdit`/`CoverEdit` are three-way (leave alone / clear / set),
// not a plain optional value — needed so a field the user never touched
// doesn't clobber files in the selection that had a *different* value than
// the one shown ("multiple values"). Default (externally tagged) serde
// representation: unit variants serialize as bare strings, the `Set` tuple/
// struct variant as `{ Set: ... }`.
export type FieldEdit = "Unset" | "Clear" | { Set: string };
export type CoverEdit =
  | "Unset"
  | "Clear"
  | { Set: { mime: string; data_base64: string; picture_type: string } };

export interface TagEdits {
  title: FieldEdit;
  artist: FieldEdit;
  album: FieldEdit;
  album_artist: FieldEdit;
  composer: FieldEdit;
  year: FieldEdit;
  track: FieldEdit;
  track_total: FieldEdit;
  disc: FieldEdit;
  disc_total: FieldEdit;
  genre: FieldEdit;
  comment: FieldEdit;
  // `null` leaves the compilation flag untouched.
  compilation: boolean | null;
  cover: CoverEdit;
}

export interface TagWriteSummary {
  total: number;
  written: number;
  failed: number;
  errors: string[];
}

export interface PlaybackFinished {
  request_id: number;
}

// An approximate loudness reading (0..1, an RMS of the samples about to play,
// not a calibrated measurement) for the equalizer bars — see
// `src-tauri/src/playback.rs`'s emission side.
export interface PlaybackLevel {
  request_id: number;
  level: number;
}

// --- Online tag lookup (MusicBrainz + Discogs) -------------------------------

export type LookupSource = "MusicBrainz" | "Discogs";

export interface LookupCandidate {
  source: LookupSource;
  id: string;
  title: string;
  artist: string;
  year: string | null;
  track_count: number | null;
}

export interface LookupTrack {
  position: string;
  title: string;
}

export interface LookupRelease {
  title: string;
  artist: string;
  year: string | null;
  tracks: LookupTrack[];
  cover: CoverArt | null;
}

// --- Playlist export (Extended M3U) ------------------------------------------

export interface PlaylistEntry {
  path: string;
  duration_secs: number;
  title: string | null;
  artist: string | null;
}

export type PlaylistFormat = "Simple" | "Extended";

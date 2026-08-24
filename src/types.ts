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
  // The codec inside `format`, when that distinction means something (an MP4
  // can hold ALAC or AAC) — `null` for single-codec containers, where it
  // would just repeat `format`. See core's `FileAnalysis::codec` doc comment.
  codec: string | null;
  ext_mismatch: boolean;
  sample_rate: number;
  channels: number;
  declared_bits: number | null;
  duration_secs: number;
  // On-disk size in bytes, read from the filesystem by the Rust side so it
  // matches what the OS reports (never derived from bitrate × duration).
  size_bytes: number;
  // Average bitrate in kbps (`size_bytes * 8 / duration_secs`) — `null` when
  // the duration isn't known.
  bitrate_kbps: number | null;
  // Filesystem modification time, Unix seconds — `null` if unreadable.
  modified_unix: number | null;
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
  // The tool that produced this file, when it left a signature behind (FLAC's
  // Vorbis comment vendor string, an MP3's ID3v2 TSSE frame, ...). Read-only —
  // no counterpart in TagEdits.
  encoder: string | null;
}

export interface TagReadResult {
  path: string;
  tags: TagSet | null;
  error: string | null;
}

// Backs the results table's inline rename — the new path and file name of a
// file just renamed on disk (`rename_file`). `file_name` comes from the
// backend rather than being re-derived from `path` on the frontend, so a
// Windows-style "\" separator doesn't need its own regex here too.
export interface RenameResult {
  path: string;
  file_name: string;
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
  // Sparse add/edit/remove instructions for extended tags, keyed by the same
  // raw format-specific tag name `TagSet.extra` pairs use — see Rust's
  // `TagEdits::extra` doc comment.
  extra: [string, FieldEdit][];
}

// One entry in the extended-tags pop-in's "+" picker — mirrors Rust's
// `core::tags::AddableTag`.
export interface AddableTag {
  key: string;
  label: string;
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

// The current playhead, throttled the same way as `PlaybackLevel` — drives
// the footer's seek bar.
export interface PlaybackPosition {
  request_id: number;
  position_secs: number;
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

// --- Conversion (ConvertPanel) ------------------------------------------------

// Mirrors Rust's `core::convert::ConvertFormat` (`#[serde(rename_all =
// "lowercase")]` on a unit-only enum serializes as a plain string).
export type ConvertFormat = "flac" | "opus" | "mp3" | "wav";

export interface ConvertSettings {
  format: ConvertFormat;
  // kbps, only meaningful for "opus"/"mp3" — ignored (and may be omitted) for
  // "flac"/"wav". `null`/omitted falls back to the backend's own default for
  // whichever lossy format is picked.
  bitrate_kbps: number | null;
}

// One audio file queued for conversion, and the folder its output layout is
// measured against — the parent of whatever the user dropped, so a dropped
// folder is itself recreated at the destination. Carried per file rather than
// recomputed later: once folders are expanded into files, nothing downstream
// can tell what was dropped. Mirrors `SourceEntry` in commands/convert.rs.
export interface ConvertSource {
  path: string;
  base: string;
}

export interface ConvertSummary {
  total: number;
  converted: number;
  failed: number;
  // Non-audio files (covers, playlists, spectrograms, ...) copied verbatim —
  // 0 unless the "copy other files" option was on.
  copied: number;
  output_root: string;
  // One "name: reason" message per failed file — a batch keeps going past a
  // single bad file rather than aborting the rest.
  errors: string[];
  // Files that converted but whose tags could not be carried over. Separate
  // from `errors` because these files exist and play: reporting them as
  // failures would send the user looking for output that is already there.
  tag_warnings: string[];
}

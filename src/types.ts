// Types mirroring the Rust `serde` payloads exchanged with the Tauri backend.

export interface Detections {
  upscaling: boolean;
  upsampling: boolean;
  // A plain boolean like its two siblings above. It used to be a three-state
  // enum whose middle value, "suspected", was raised by a gentle spectral
  // roll-off — which is what naturally dark acoustic and analog-tape masters
  // look like, so it accused a whole genre. Transcoding is now decided only
  // by the codec-lattice detectors, which either find the lattice or don't.
  transcoding: boolean;
  detail: string;
  // "Clean" (no finding), "Flagged", or "Not analyzed" for an empty record.
  // Unavailable checks are explained in detail rather than a third verdict.
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

export type StereoBalance =
  | { state: "Measured"; right_minus_left_db: number }
  | { state: "LeftSilent" }
  | { state: "RightSilent" };

export interface HighFrequencyStereo {
  // Side/Mid energy ratios in dB. More negative means less high-band stereo width.
  side_to_mid_db: number;
  reference_side_to_mid_db: number;
  narrowed_block_fraction: number;
  // A conservative high-frequency narrowing cue, not a codec verdict.
  narrowed: boolean;
}

export interface DcOffset {
  // Normalized amplitude means in decoded channel order; multiply by 100 for %.
  channel_means: number[];
  max_abs: number;
}

export interface PhaseSummary {
  correlation: number;
  minimum_correlation: number;
  minimum_start_secs: number;
  opposed_fraction: number;
  eligible_windows: number;
}

export interface LoudnessPeak {
  lufs: number;
  start_secs: number;
}

export interface LoudnessPeaks {
  momentary: LoudnessPeak | null;
  short_term: LoudnessPeak | null;
}

export interface BandPhase {
  low_hz: number;
  high_hz: number;
  summary: PhaseSummary | null;
}

export interface LocalPhase {
  window_secs: number;
  hop_secs: number;
  analyzed_windows: number;
  broadband: PhaseSummary | null;
  bands: BandPhase[];
}

export interface DiscontinuityEvent {
  channel: number;
  start_secs: number;
  duration_secs: number;
}

export interface EventSummary {
  // Events on different channels count separately; at most 32 locations.
  count: number;
  events: DiscontinuityEvent[];
}

export interface DiscontinuityAnalysis {
  clicks: EventSummary;
  dropouts: EventSummary;
}

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
  // Exact occupied bits can include low-level noise around an inferred grid.
  bit_depth_evidence?: { stored_bits: number; method: "Stored" | "NarrowGrid" } | null;
  // The transcoding evidence, 0..1 — see Rust's `FileAnalysis::lattice_score`.
  // `null` means the search did not run, which is not the same as a low score.
  lattice_score: number | null;
  fake_stereo: boolean | null;
  // Older JSON reports omit these measurements.
  phase_correlation?: number | null;
  phase_inverted?: boolean | null;
  stereo_balance?: StereoBalance | null;
  high_frequency_stereo?: HighFrequencyStereo | null;
  dc_offset?: DcOffset | null;
  local_phase?: LocalPhase | null;
  badge: string | null;
  clipping: ClippingInfo;
  dr_db: number | null;
  // Older JSON reports omit this measurement.
  integrated_lufs?: number | null;
  loudness_peaks?: LoudnessPeaks | null;
  loudness_range_lu?: number | null;
  discontinuities?: DiscontinuityAnalysis | null;
  flac_md5: FlacMd5Status | null;
  // Fingerprints of the *file's bytes* — tags and cover art included — as
  // lowercase hex. Not to be confused with `flac_md5`, which is about the
  // decoded audio: see Rust's `core::hash` module docs. `null` when the file
  // could not be read.
  file_md5: string | null;
  file_crc32: string | null;
  error: string | null;
}

// One file's move, found by `relocate_paths` — mirrors Rust's
// `core::relocate::Relocation`.
export interface Relocation {
  from: string;
  to: string;
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
  spectrogram_dirs: string[];
  errors: string[];
}

export type SpectrogramSize = "half" | "full";

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
  pictures: CoverArt[];
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

// Rust's `FieldEdit` is three-way (leave alone / clear / set),
// not a plain optional value — needed so a field the user never touched
// doesn't clobber files in the selection that had a *different* value than
// the one shown ("multiple values"). Default (externally tagged) serde
// representation: unit variants serialize as bare strings, the `Set` tuple/
// struct variant as `{ Set: ... }`.
export type FieldEdit = "Unset" | "Clear" | { Set: string };
export type CoverEdit =
  | { Clear: { picture_type: string } }
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
  pictures: CoverEdit[];
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

// How hard the FLAC encoder should search for a smaller representation.
// Deliberately not libFLAC's -0..-8: this app encodes with `flacenc`, whose
// knobs are its own, so those labels would promise an equivalence the output
// does not have. Every level is lossless — only the time spent and the
// resulting size change. Mirrors Rust's `core::convert::FlacEffort`.
export type FlacEffort = "fast" | "balanced" | "maximum";

export interface ConvertSettings {
  format: ConvertFormat;
  // kbps, only meaningful for "opus"/"mp3" — ignored (and may be omitted) for
  // "flac"/"wav". `null`/omitted falls back to the backend's own default for
  // whichever lossy format is picked.
  bitrate_kbps: number | null;
  // How hard to search when the target is FLAC. Ignored for other formats.
  flac_effort: FlacEffort;
  // Stamp the converted file with the source's last-modified date. Applies to
  // every format — it describes the file written, not the codec that wrote
  // it — which is why it sits beside "copy other files" and not under FLAC.
  preserve_modtime: boolean;
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

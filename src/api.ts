// Thin typed wrappers around the Tauri backend commands.

import { invoke } from "@tauri-apps/api/core";
import type {
  AddableTag,
  ConvertSettings,
  ConvertSource,
  ConvertSummary,
  CoverArt,
  FolderReport,
  LookupCandidate,
  LookupRelease,
  PlaylistEntry,
  PlaylistFormat,
  Relocation,
  RenameResult,
  SpectroSummary,
  TagEdits,
  TagReadResult,
  TagWriteSummary,
} from "./types";

export const analyzePaths = (targets: string[]) =>
  invoke<FolderReport>("analyze_paths", { targets });

export const generateSpectrograms = (targets: string[]) =>
  invoke<SpectroSummary>("generate_spectrograms", { targets });

// Report export is two separate commands (not one that always writes both)
// so the menu bar's standalone "Export CSV"/"Export JSON" can write just one
// file — the toolbar's combined "Save…" calls both in sequence itself
// instead of the backend duplicating that combination. Each forces its own
// extension from `dest`'s stem, so either may be called with any `dest`.
export const saveReportCsv = (dest: string, report: FolderReport) =>
  invoke<string>("save_report_csv", { dest, report });

export const saveReportJson = (dest: string, report: FolderReport) =>
  invoke<string>("save_report_json", { dest, report });

// Re-imports a previously-saved JSON report (dropped onto the window).
export const loadReport = (path: string) =>
  invoke<FolderReport>("load_report", { path });

export const ffmpegAvailable = () => invoke<boolean>("ffmpeg_available");

export const cancelTask = () => invoke("cancel_task");

export const revealInFolder = (path: string) =>
  invoke("reveal_in_folder", { path });

// Opens a folder's own contents in the OS file browser — unlike
// `revealInFolder`, which selects a file within its *parent*. Backs the
// results header's folder icon next to the analyzed root path.
export const openFolder = (path: string) => invoke("open_folder", { path });

// Which of `paths` are no longer files on disk. Returns only the missing
// ones, so an empty array means everything is where the table says it is.
// Backs the results table's refresh button and the automatic check after a
// saved report is reloaded — see useMissingFiles.ts for why the answer is
// never stored in `FileAnalysis`.
export const missingPaths = (paths: string[]) =>
  invoke<string[]>("missing_paths", { paths });

// Looks for each missing file by name under `root`, returning the moves it
// found. Nothing is written — the caller repoints the listing, and the user
// still has to save the report for the new paths to survive the session.
export const relocatePaths = (missing: string[], root: string) =>
  invoke<Relocation[]>("relocate_paths", { missing, root });

// Renames a file on disk to `newStem` plus its existing extension — the
// backend re-appends the extension itself rather than trusting a full name
// from the frontend, so the results table's inline rename can never smuggle
// a different one through even if this wrapper were bypassed. Returns the
// new path and file name.
export const renameFile = (path: string, newStem: string) =>
  invoke<RenameResult>("rename_file", { path, newStem });

// Tags: read-only batch used to pre-fill the thumbnail column and (later)
// the tag panel. Errors are per-file (e.g. DSD isn't taggable), not thrown.
export const readTagsBatch = (paths: string[]) =>
  invoke<TagReadResult[]>("read_tags_batch", { paths });

// Tags: batch write — the tag panel's Save button. Applies the same edits to
// every file in `paths`; per-file failures are reported in the summary
// rather than throwing, so one locked/read-only file doesn't abort the rest.
export const writeTagsBatch = (paths: string[], edits: TagEdits) =>
  invoke<TagWriteSummary>("write_tags_batch", { paths, edits });

// A plain image file (not one of the analyzed audio files) dropped onto the
// tag panel's cover box — backs "drop an image to replace every selected
// file's cover".
export const readCoverImage = (path: string) =>
  invoke<CoverArt>("read_cover_image", { path });

// Writes a cover (already in hand as base64 — no re-read from any tag) out as
// a plain file in `dir`, backing the tag panel's "extract cover(s)" button.
// `index` (1-based) picks the name: 1 gets the classic "cover.<ext>", any
// later index gets "cover-<n>.<ext>" — so extracting every distinct cover in
// a multi-cover selection doesn't have the second call overwrite the first.
// Returns the path actually written.
export const extractCoverArt = (dir: string, mime: string, dataBase64: string, index: number) =>
  invoke<string>("extract_cover_art", { dir, mime, dataBase64, index });

// The extended-tags pop-in's "+" picker: common tags this one representative
// file's format can actually take, curated on the Rust side (lofty has no
// full-enumeration API to derive this from).
export const listAddableTags = (path: string) =>
  invoke<AddableTag[]>("list_addable_tags", { path });

// Playback: single-track preview. `playTrack` returns a request id used to
// match the `playback://finished` event to the track that was actually
// still playing when it fired (a fast stop+replay could otherwise let a
// stale "finished" from the previous track trigger auto-advance).
export const playTrack = (path: string) => invoke<number>("play_track", { path });

export const stopPlayback = () => invoke("stop_playback");

// Playback: pause the current track in place (the footer's play/pause
// button) and resume it — unlike `stopPlayback`, which tears the stream down
// and forgets the position, this keeps both, so resuming is instant.
export const pausePlayback = () => invoke("pause_playback");
export const resumePlayback = () => invoke("resume_playback");

// Playback: move the current track's playhead — the footer's seek bar,
// dragged or clicked.
export const seekPlayback = (seconds: number) => invoke("seek_playback", { seconds });

// Playback: volume (0..1) and mute. Plain atomics on the Rust side (see
// `playback::set_volume`'s doc comment), so neither can fail.
export const setVolume = (volume: number) => invoke("set_volume", { volume });
export const setMuted = (muted: boolean) => invoke("set_muted", { muted });

// Online lookup: the tag panel's "Search online" button. Search returns a
// short candidate list; picking one fetches its full detail (track list +
// cover) to stage into the tag panel. Discogs calls need the user's own
// personal API token (kept in localStorage by the frontend, never persisted
// on the Rust side).
export const lookupMusicbrainz = (query: string) =>
  invoke<LookupCandidate[]>("lookup_musicbrainz", { query });

export const lookupMusicbrainzDetail = (id: string) =>
  invoke<LookupRelease>("lookup_musicbrainz_detail", { id });

export const lookupDiscogs = (query: string, token: string) =>
  invoke<LookupCandidate[]>("lookup_discogs", { query, token });

export const lookupDiscogsDetail = (id: string, token: string) =>
  invoke<LookupRelease>("lookup_discogs_detail", { id, token });

// Playlist export: the frontend builds each entry (order, duration, cached
// tags) — this just turns them into Simple or Extended M3U text and writes
// it. Returns the path actually written: the backend forces the extension to
// match the chosen format, so it may differ from `dest`.
export const savePlaylist = (dest: string, entries: PlaylistEntry[], format: PlaylistFormat) =>
  invoke<string>("save_playlist", { dest, entries, format });

// Conversion: the panel's own imported tracks/folders (`targets`, independent
// of the main results table), re-encoded under `outputRoot` mirroring their
// source layout. `copyOthers` also copies every non-audio file sharing a
// source folder (covers, `.m3u` playlists, generated spectrograms, ...)
// verbatim, unconverted — "tout ou rien", no per-file choice. Progress
// arrives on the `convert://progress` event, same `Progress` shape analysis
// and spectrogram rendering already use.
// Expands dropped folders into the tracks they hold, so the conversion panel
// lists (and counts) real files rather than whatever was dropped. Same walk
// and same filtering `convert_files` performs, so the two cannot disagree.
// Each file comes back with the folder its output layout is measured
// against, recorded from what was dropped — see `ConvertSource`.
export const listConvertSources = (targets: string[]) =>
  invoke<ConvertSource[]>("list_convert_sources", { targets });

export const convertFiles = (
  targets: ConvertSource[],
  outputRoot: string,
  settings: ConvertSettings,
  copyOthers: boolean,
) =>
  invoke<ConvertSummary>("convert_files", {
    targets,
    outputRoot,
    settings,
    copyOthers,
  });

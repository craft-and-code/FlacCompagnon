// Thin typed wrappers around the Tauri backend commands.

import { invoke } from "@tauri-apps/api/core";
import type {
  CoverArt,
  FolderReport,
  LookupCandidate,
  LookupRelease,
  PlaylistEntry,
  PlaylistFormat,
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
// a plain "cover.<ext>" file in `dir`, backing the tag panel's "extract this
// cover" button. Returns the path actually written.
export const extractCoverArt = (dir: string, mime: string, dataBase64: string) =>
  invoke<string>("extract_cover_art", { dir, mime, dataBase64 });

// Playback: single-track preview. `playTrack` returns a request id used to
// match the `playback://finished` event to the track that was actually
// still playing when it fired (a fast stop+replay could otherwise let a
// stale "finished" from the previous track trigger auto-advance).
export const playTrack = (path: string) => invoke<number>("play_track", { path });

export const stopPlayback = () => invoke("stop_playback");

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

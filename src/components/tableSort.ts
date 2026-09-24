// Column sorting for the results table's headers.
//
// Pure logic, no React. The sort state itself lives in App.tsx, not
// ResultsTable — Play/Previous/Next (usePlaybackQueue) and the natural
// end-of-track auto-advance (usePlayback) both need to walk the same order
// the table is actually showing, not a copy of it only ResultsTable knows
// about. ResultsTable stays the one place that renders the chevrons and
// reports clicks; this module only knows how to compare two `FileAnalysis`
// values for a given column. No column is sorted by default — see
// `nextSort`: the third click on a column (or never clicking one at all)
// means the natural/drag order, which is what the table opens in.

import type { FileAnalysis } from "../types";
import { detectionLabels, stereoLabel } from "../format";

export type SortColumn =
  | "file"
  | "format"
  | "codec"
  | "badge"
  | "rate"
  | "bits"
  | "realBits"
  | "bitrate"
  | "length"
  | "modified"
  | "size"
  | "detections"
  | "cutoff"
  | "channels"
  | "stereo"
  | "balance"
  | "hfStereo"
  | "clicks"
  | "dropouts"
  | "clipping"
  | "truePeak"
  | "fileMd5"
  | "fileCrc32"
  | "dynamics"
  | "loudness"
  | "lra"
  | "md5";

export type SortDirection = "asc" | "desc";

export interface SortState {
  column: SortColumn;
  direction: SortDirection;
}

/// Not alphabetical on the state name (that would put "Error" before "Match"
/// for no good reason) — worst first, so sorting the MD5 column surfaces the
/// files worth looking at.
function md5Rank(m: FileAnalysis["flac_md5"]): number | null {
  if (!m) return null;
  switch (m.state) {
    case "Mismatch":
      return 0;
    case "Error":
      return 1;
    case "Present":
      return 2;
    case "NoSignature":
      return 3;
    case "Match":
      return 4;
  }
}

/// The value to sort `col` by. For a column whose cell only *formats* a
/// number (Rate, Size, Clipping's event count, …) this is that raw number, so
/// "20 events" sorts after "9 events" instead of before it as plain text
/// would. For a column whose cell is inherently a short label (Format,
/// Detections, Stereo) it's that exact label, lower-cased, so sorting matches
/// what's on screen. `null` means "no measurement for this file" and always
/// sorts last, in either direction — it isn't the smallest value, it's simply
/// unranked.
function sortValue(f: FileAnalysis, col: SortColumn): string | number | null {
  switch (col) {
    case "file":
      return f.file_name.toLowerCase();
    case "format":
      return f.format.toLowerCase();
    case "codec":
      return f.codec?.toLowerCase() ?? null;
    case "badge":
      return f.badge?.toLowerCase() ?? null;
    case "rate":
      return f.sample_rate;
    case "bits":
      return f.declared_bits;
    case "realBits":
      return f.real_bit_depth;
    // Hex strings sort lexicographically, which for a fixed-width hex value
    // is the same order as sorting the number it encodes.
    case "fileMd5":
      return f.file_md5;
    case "fileCrc32":
      return f.file_crc32;
    case "bitrate":
      return f.bitrate_kbps;
    case "length":
      return f.duration_secs;
    case "modified":
      return f.modified_unix;
    case "size":
      return f.size_bytes;
    case "detections":
      return detectionLabels(f.detections).join(" ").toLowerCase();
    case "cutoff":
      return f.cutoff_hz;
    case "channels":
      return f.channels;
    case "stereo":
      return stereoLabel(f);
    case "balance": {
      const balance = f.stereo_balance;
      if (!balance) return null;
      // Sort from left-heavy to right-heavy; keep silent-channel endpoints
      // finite so comparing two identical endpoints cannot produce NaN.
      if (balance.state === "LeftSilent") return Number.MAX_VALUE;
      if (balance.state === "RightSilent") return -Number.MAX_VALUE;
      return Number.isFinite(balance.right_minus_left_db) ? balance.right_minus_left_db : null;
    }
    case "hfStereo": {
      const measurement = f.high_frequency_stereo;
      return measurement && Number.isFinite(measurement.side_to_mid_db)
        ? measurement.side_to_mid_db
        : null;
    }
    case "clipping":
      return f.clipping.clipped ? f.clipping.clip_events : 0;
    case "clicks":
    case "dropouts":
      return f.discontinuities?.[col].count ?? null;
    case "truePeak":
      return Number.isFinite(f.clipping.true_peak_dbtp) ? f.clipping.true_peak_dbtp : null;
    case "dynamics":
      return f.dr_db;
    case "loudness":
      return f.integrated_lufs ?? null;
    case "lra":
      return f.loudness_range_lu ?? null;
    case "md5":
      return md5Rank(f.flac_md5);
  }
}

function compare(a: FileAnalysis, b: FileAnalysis, col: SortColumn, dir: SortDirection): number {
  const va = sortValue(a, col);
  const vb = sortValue(b, col);
  if (va == null && vb == null) return 0;
  if (va == null) return 1;
  if (vb == null) return -1;
  const cmp = typeof va === "string" && typeof vb === "string" ? va.localeCompare(vb) : (va as number) - (vb as number);
  return dir === "asc" ? cmp : -cmp;
}

/// A new, sorted array — `files` itself is never mutated, since callers
/// (`ResultsTable`) also rely on it for the natural/manual order underneath.
export function sortFiles(files: FileAnalysis[], sort: SortState): FileAnalysis[] {
  return [...files].sort((a, b) => compare(a, b, sort.column, sort.direction));
}

/// The next state a header click moves to: ascending, then descending, then
/// back to no sort at all (`null`, i.e. the natural/drag order) — a third
/// click on the same column, or a first click on a different one, resets to
/// ascending on that column.
export function nextSort(current: SortState | null, col: SortColumn): SortState | null {
  if (current?.column !== col) return { column: col, direction: "asc" };
  if (current.direction === "asc") return { column: col, direction: "desc" };
  return null;
}

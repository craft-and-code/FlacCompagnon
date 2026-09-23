// Definitions for the results table's toggleable, reorderable columns —
// everything shown/hidden and reordered via the header's right-click menu
// (ColumnMenu.tsx, state in useColumnPrefs.ts).
//
// Quality (badge) and MD5 are columns like any other here — reorderable and
// toggleable the same way Format or Size are — but each carries a
// `conditional` marker: ResultsTable additionally hides it when the current
// files don't warrant it at all (no file has a badge; no file is FLAC), on
// top of whatever the user's own show/hide choice says. That's an AND, not
// an override — a user who explicitly hid Quality doesn't see it come back
// just because a badge shows up, and a table with no FLAC files doesn't grow
// an all-"—" MD5 column nobody asked to see. An earlier version pinned these
// two outside the reorderable system entirely, which is exactly the "can't
// move Quality/MD5" complaint this shape fixes.
//
// Tag-derived columns (Artist, Album, Title, Track, Year, Genre, Encoder)
// read from the same tags cache the thumbnail column and tag panel already
// share (`useTagCache`) — no extra fetch, since every visible file's tags
// are prefetched already (see `useTagPrefetch` in App.tsx). They're also not
// sortable yet: `tableSort.ts` only ever compares `FileAnalysis` values, and
// wiring the tags map through the sort path is a bigger change than adding a
// column — left for later if it turns out to matter in practice.

import type { ReactNode } from "react";
import type { FileAnalysis, TagSet } from "../types";
import type { SortColumn } from "./tableSort";
import { fmtBitrate, fmtCutoff, fmtDuration, fmtModified, fmtSize } from "../format";
import {
  ClippingCell,
  DetectionsCell,
  DynamicRangeCell,
  LoudnessCell,
  FileHashCell,
  Md5Cell,
  QualityBadgeCell,
  RealBitsCell,
  StereoCell,
  TruePeakCell,
} from "./ResultCells";

export type ColumnKey =
  | "format"
  | "quality"
  | "codec"
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
  | "clipping"
  | "truePeak"
  | "fileMd5"
  | "fileCrc32"
  | "dynamics"
  | "loudness"
  | "lra"
  | "artist"
  | "album"
  | "title"
  | "track"
  | "year"
  | "genre"
  | "encoder"
  | "md5";

export interface ColumnDef {
  key: ColumnKey;
  label: string;
  /// Present for columns `tableSort.ts` knows how to compare — omitted for
  /// the tag-derived ones (see the module doc comment above).
  sort?: SortColumn;
  /// Whether a fresh install, or a genuinely new column an existing install
  /// has never seen before, starts with this column shown.
  defaultVisible: boolean;
  /// Quality/MD5 only: an extra, data-driven condition ResultsTable applies
  /// on top of the user's own visibility choice — see the module doc
  /// comment. `undefined` for every other column, which is shown/hidden by
  /// user preference alone.
  conditional?: "badge" | "md5";
  /// Renders this column's `<td>` for one row. `tag` is `null` until that
  /// file's tags have loaded (or it has none/failed to read) — same
  /// "placeholder first, fill in later" contract as the thumbnail column.
  render: (f: FileAnalysis, tag: TagSet | null) => ReactNode;
}

/// A plain text tag field, muted when absent — the shape every tag-derived
/// column below shares.
function tagCell(value: string | null | undefined): ReactNode {
  return <td className={value ? undefined : "c-muted"}>{value || "—"}</td>;
}

export const ALL_COLUMNS: ColumnDef[] = [
  {
    key: "format",
    label: "Format",
    sort: "format",
    defaultVisible: true,
    render: (f) =>
      f.ext_mismatch ? (
        <td
          className="c-bad has-tip"
          title={`Real container is ${f.format}, which does not match the file extension`}
        >
          {f.format}
        </td>
      ) : (
        <td>{f.format}</td>
      ),
  },
  {
    key: "quality",
    label: "Quality",
    sort: "badge",
    defaultVisible: true,
    conditional: "badge",
    render: (f) => <QualityBadgeCell badge={f.badge} />,
  },
  {
    key: "codec",
    label: "Codec",
    sort: "codec",
    defaultVisible: false,
    render: (f) => <td className={f.codec ? undefined : "c-muted"}>{f.codec ?? "—"}</td>,
  },
  {
    key: "rate",
    label: "Rate",
    sort: "rate",
    defaultVisible: true,
    render: (f) => <td>{(f.sample_rate / 1000).toFixed(1)}k</td>,
  },
  {
    key: "bits",
    label: "Bits",
    sort: "bits",
    defaultVisible: true,
    render: (f) => <td>{f.declared_bits != null ? `${f.declared_bits}-bit` : "float"}</td>,
  },
  {
    key: "realBits",
    label: "Real bits",
    sort: "realBits",
    defaultVisible: true,
    render: (f) => <RealBitsCell f={f} />,
  },
  {
    key: "bitrate",
    label: "Bitrate",
    sort: "bitrate",
    defaultVisible: false,
    render: (f) => (
      <td className={f.bitrate_kbps ? undefined : "c-muted"}>{fmtBitrate(f.bitrate_kbps)}</td>
    ),
  },
  {
    key: "length",
    label: "Length",
    sort: "length",
    defaultVisible: true,
    render: (f) => <td>{fmtDuration(f.duration_secs)}</td>,
  },
  {
    key: "modified",
    label: "Modified",
    sort: "modified",
    defaultVisible: false,
    render: (f) => (
      <td className={f.modified_unix ? undefined : "c-muted"}>{fmtModified(f.modified_unix)}</td>
    ),
  },
  {
    key: "size",
    label: "Size",
    sort: "size",
    defaultVisible: true,
    render: (f) => (
      <td className="size has-tip" title={`${f.size_bytes.toLocaleString()} bytes`}>
        {fmtSize(f.size_bytes)}
      </td>
    ),
  },
  {
    key: "detections",
    label: "Detections",
    sort: "detections",
    defaultVisible: true,
    render: (f) => <DetectionsCell d={f.detections} />,
  },
  {
    key: "cutoff",
    label: "Cutoff",
    sort: "cutoff",
    defaultVisible: true,
    render: (f) => <td>{fmtCutoff(f)}</td>,
  },
  {
    key: "channels",
    label: "Ch",
    sort: "channels",
    defaultVisible: true,
    render: (f) => <td>{f.channels}</td>,
  },
  {
    key: "stereo",
    label: "Stereo",
    sort: "stereo",
    defaultVisible: true,
    render: (f) => <StereoCell f={f} />,
  },
  {
    key: "clipping",
    label: "Clipping",
    sort: "clipping",
    defaultVisible: true,
    render: (f) => <ClippingCell c={f.clipping} />,
  },
  {
    key: "truePeak",
    label: "True Peak",
    sort: "truePeak",
    defaultVisible: true,
    render: (f) => <TruePeakCell dbtp={f.clipping.true_peak_dbtp} />,
  },
  {
    key: "loudness",
    label: "LUFS",
    sort: "loudness",
    defaultVisible: true,
    render: (f) => <LoudnessCell value={f.integrated_lufs} mode="integrated" />,
  },
  {
    key: "lra",
    label: "LRA",
    sort: "lra",
    defaultVisible: true,
    render: (f) => <LoudnessCell value={f.loudness_range_lu} mode="range" />,
  },
  {
    key: "dynamics",
    label: "Dynamics",
    sort: "dynamics",
    defaultVisible: true,
    render: (f) => <DynamicRangeCell dr={f.dr_db} />,
  },
  {
    key: "artist",
    label: "Artist",
    defaultVisible: false,
    render: (_f, t) => tagCell(t?.artist),
  },
  {
    key: "album",
    label: "Album",
    defaultVisible: false,
    render: (_f, t) => tagCell(t?.album),
  },
  {
    key: "title",
    label: "Title",
    defaultVisible: false,
    render: (_f, t) => tagCell(t?.title),
  },
  {
    key: "track",
    label: "Track",
    defaultVisible: false,
    render: (_f, t) => tagCell(t?.track),
  },
  {
    key: "year",
    label: "Year",
    defaultVisible: false,
    render: (_f, t) => tagCell(t?.year),
  },
  {
    key: "genre",
    label: "Genre",
    defaultVisible: false,
    render: (_f, t) => tagCell(t?.genre),
  },
  {
    key: "encoder",
    label: "Encoder",
    defaultVisible: false,
    render: (_f, t) => tagCell(t?.encoder),
  },
  {
    key: "md5",
    label: "MD5",
    sort: "md5",
    defaultVisible: true,
    conditional: "md5",
    render: (f) => <Md5Cell m={f.flac_md5} />,
  },
  // Last, and hidden by default: fingerprints of the file's bytes, for
  // checksum comparison and duplicate hunting. Distinct from the MD5 column
  // above, which is FLAC's own signature over the decoded *audio* — hence the
  // "File " prefix on both labels rather than a bare "MD5" that would sit two
  // columns away from a different MD5.
  {
    key: "fileMd5",
    label: "File MD5",
    sort: "fileMd5",
    defaultVisible: false,
    render: (f) => <FileHashCell hash={f.file_md5} />,
  },
  {
    key: "fileCrc32",
    label: "File CRC32",
    sort: "fileCrc32",
    defaultVisible: false,
    render: (f) => <FileHashCell hash={f.file_crc32} />,
  },
];

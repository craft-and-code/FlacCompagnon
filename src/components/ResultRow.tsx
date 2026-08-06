// One row of the results table.
//
// The thumbnail is `undefined` until its tags load in the background, which
// renders the same placeholder as "no cover at all" — so the row appears
// immediately and fills in later without needing a distinct loading state.

import { Music, Play, Search, Square, Trash2 } from "lucide-react";

import type { CoverArt, FileAnalysis } from "../types";
import { coverDataUrl, fmtCutoff, fmtDuration, fmtSize } from "../format";
import { IconButton } from "./IconButton";
import { LiveEqualizerBars } from "./LiveEqualizerBars";
import {
  ClippingCell,
  DetectionsCell,
  DynamicRangeCell,
  Md5Cell,
  QualityBadgeCell,
  RealBitsCell,
  StereoCell,
  TruePeakCell,
} from "./ResultCells";

export interface ResultRowProps {
  file: FileAnalysis;
  cover: CoverArt | null | undefined;
  playing: boolean;
  /// Set together with `playing` — identifies which `playback://level` events
  /// belong to this row's track (see `LiveEqualizerBars`).
  playingRequestId: number | null;
  selected: boolean;
  /// Faded placeholder look while this row is being dragged elsewhere.
  dragging: boolean;
  /// Insertion marker during a drag.
  dropEdge: "before" | "after" | null;
  columnCount: number;
  showMd5: boolean;
  showBadge: boolean;
  onReveal: () => void;
  onTogglePlay: () => void;
  onDelete: () => void;
  onMouseDown: (ev: React.MouseEvent) => void;
  onRowClick: (ev: React.MouseEvent) => void;
}

/// Row buttons must not also start a drag or change the selection, so they
/// swallow both events before they reach the row.
function stop(ev: React.MouseEvent) {
  ev.stopPropagation();
}

function rowClass(selected: boolean, dragging: boolean, dropEdge: "before" | "after" | null) {
  const parts: string[] = [];
  if (selected) parts.push("selected");
  if (dragging) parts.push("dragging");
  if (dropEdge) parts.push(dropEdge === "before" ? "drop-before" : "drop-after");
  return parts.join(" ");
}

export function ResultRow({
  file: f,
  cover,
  playing,
  playingRequestId,
  selected,
  dragging,
  dropEdge,
  columnCount,
  showMd5,
  showBadge,
  onReveal,
  onTogglePlay,
  onDelete,
  onMouseDown,
  onRowClick,
}: ResultRowProps) {
  const url = cover ? coverDataUrl(cover) : null;

  const revealCell = (
    <td className="reveal">
      <IconButton
        icon={<Search size={14} strokeWidth={1.8} />}
        title="Reveal in file browser"
        onMouseDown={stop}
        onClick={(ev) => {
          stop(ev);
          onReveal();
        }}
      />
    </td>
  );

  const thumbCell = (
    <td className="thumb">
      <button
        className={playing ? "play-btn playing" : "play-btn"}
        title={playing ? "Stop" : "Play"}
        onMouseDown={stop}
        onClick={(ev) => {
          stop(ev);
          onTogglePlay();
        }}
      >
        {url ? (
          <img className="thumb-img" src={url} alt="" />
        ) : (
          <span className="thumb-img thumb-placeholder">
            <Music size={14} strokeWidth={1.4} />
          </span>
        )}
        <span className="play-overlay">
          {playing && playingRequestId != null ? (
            <>
              {/* Shown while playing; swapped for the stop icon on hover
                  (see .play-btn.playing:hover in styles.css) so the row
                  reads "now playing" at rest and "click to stop" on hover. */}
              <span className="play-overlay-idle">
                <LiveEqualizerBars requestId={playingRequestId} />
              </span>
              <span className="play-overlay-hover">
                <Square size={13} fill="currentColor" stroke="none" />
              </span>
            </>
          ) : (
            <Play size={13} fill="currentColor" stroke="none" />
          )}
        </span>
      </button>
    </td>
  );

  const nameCell = (
    <td className="fname has-tip" title={f.path}>
      {f.file_name}
    </td>
  );

  const deleteCell = (
    <td className="rowdel">
      <IconButton
        icon={<Trash2 size={14} strokeWidth={1.6} />}
        title="Remove this row"
        variant="danger"
        onMouseDown={stop}
        onClick={(ev) => {
          stop(ev);
          onDelete();
        }}
      />
    </td>
  );

  if (f.error) {
    // reveal + thumbnail + name precede, delete trails.
    return (
      <tr
        data-path={f.path}
        className={rowClass(selected, dragging, dropEdge)}
        onMouseDown={onMouseDown}
        onClick={onRowClick}
      >
        {revealCell}
        <td className="thumb" />
        {nameCell}
        <td colSpan={columnCount - 4} className="c-bad">
          Error: {f.error}
        </td>
        {deleteCell}
      </tr>
    );
  }

  return (
    <tr
      data-path={f.path}
      className={rowClass(selected, dragging, dropEdge)}
      onMouseDown={onMouseDown}
      onClick={onRowClick}
    >
      {revealCell}
      {thumbCell}
      {nameCell}
      {f.ext_mismatch ? (
        <td
          className="c-bad has-tip"
          title={`Real container is ${f.format}, which does not match the file extension`}
        >
          {f.format}
        </td>
      ) : (
        <td>{f.format}</td>
      )}
      {showBadge && <QualityBadgeCell badge={f.badge} />}
      <td>{(f.sample_rate / 1000).toFixed(1)}k</td>
      <td>{f.declared_bits != null ? `${f.declared_bits}-bit` : "float"}</td>
      <RealBitsCell f={f} />
      <td>{fmtDuration(f.duration_secs)}</td>
      <td className="size has-tip" title={`${f.size_bytes.toLocaleString()} bytes`}>
        {fmtSize(f.size_bytes)}
      </td>
      <DetectionsCell d={f.detections} />
      <td>{fmtCutoff(f)}</td>
      <td>{f.channels}</td>
      <StereoCell f={f} />
      <ClippingCell c={f.clipping} />
      <TruePeakCell dbtp={f.clipping.true_peak_dbtp} />
      <DynamicRangeCell dr={f.dr_db} />
      {showMd5 && <Md5Cell m={f.flac_md5} />}
      {deleteCell}
    </tr>
  );
}

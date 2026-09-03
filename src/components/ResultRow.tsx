// One row of the results table.
//
// The thumbnail is `undefined` until its tags load in the background, which
// renders the same placeholder as "no cover at all" — so the row appears
// immediately and fills in later without needing a distinct loading state.

import { Fragment, useEffect, useRef, useState } from "react";
import { GripVertical, Music, Play, Search, Square, Trash2, Unlink } from "lucide-react";

import type { CoverArt, FileAnalysis, TagSet } from "../types";
import { coverDataUrl, splitStem } from "../format";
import { IconButton } from "./IconButton";
import { LiveEqualizerBars } from "./LiveEqualizerBars";
import type { ColumnDef } from "./resultColumns";
import "./ResultRow.css";

export interface ResultRowProps {
  file: FileAnalysis;
  cover: CoverArt | null | undefined;
  /// This file's tags, for the optional tag-derived columns — `null` until
  /// loaded (or if it has none/failed to read), same contract as `cover`.
  tag: TagSet | null;
  /// Everything between the fixed filename cell and the fixed delete button,
  /// shown/hidden/ordered from the header's right-click menu — see
  /// resultColumns.tsx. Drives both the cell count (this row's `colSpan` on
  /// an error, and the header's own column count in ResultsTable) and what
  /// actually renders here.
  columns: ColumnDef[];
  playing: boolean;
  /// Set together with `playing` — identifies which `playback://level` events
  /// belong to this row's track (see `LiveEqualizerBars`).
  playingRequestId: number | null;
  selected: boolean;
  /// Faded placeholder look while this row is being dragged elsewhere.
  dragging: boolean;
  /// Insertion marker during a drag.
  dropEdge: "before" | "after" | null;
  /// Shows an editable field instead of plain text — ResultsTable decides
  /// when this turns on (a second, well-spaced click on an already-sole-
  /// selected row's name; see its `RENAME_CLICK_GAP_MS`).
  editing: boolean;
  /// True while a submitted rename is still in flight — the field goes
  /// read-only (not `disabled`: a disabled input forces a blur in most
  /// browsers, which would fire `onCancelRename` mid-request).
  renameBusy: boolean;
  /// The file is no longer at `file.path`. Frontend-only, never part of the
  /// analysis or of any export — see useMissingFiles.ts.
  missing: boolean;
  onReveal: () => void;
  onTogglePlay: () => void;
  onDelete: () => void;
  /// Escape, or clicking anywhere else (the field's blur) — discards
  /// whatever was typed.
  onCancelRename: () => void;
  onSubmitRename: (newStem: string) => void;
  /// Starts a manual reorder — wired only to the drag-handle cell (the dots
  /// before the reveal button), not the row as a whole. Hovering *anywhere*
  /// on the row used to show a grab cursor, but a handful of cells
  /// (`.has-tip`, for the filename's full-path tooltip and several detection
  /// columns) set `cursor: help` on themselves, which — being more specific
  /// than the row's own cursor rule — silently won there instead, so those
  /// cells looked undraggable even though the row could still be picked up
  /// from them. A dedicated handle removes the ambiguity: it's the only
  /// place the cursor changes, and the only place a drag can start.
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
  tag,
  columns,
  playing,
  playingRequestId,
  selected,
  dragging,
  dropEdge,
  editing,
  renameBusy,
  missing,
  onReveal,
  onTogglePlay,
  onDelete,
  onCancelRename,
  onSubmitRename,
  onMouseDown,
  onRowClick,
}: ResultRowProps) {
  const url = cover ? coverDataUrl(cover) : null;

  const [draftName, setDraftName] = useState("");
  const nameInputRef = useRef<HTMLInputElement>(null);
  // Re-seeds from the current name (not just once on mount) every time
  // editing turns on, and focuses + selects it so typing immediately
  // replaces the whole stem — the extension after it stays plain text,
  // never part of the selection or the value submitted.
  useEffect(() => {
    if (!editing) return;
    setDraftName(splitStem(f.file_name).stem);
    nameInputRef.current?.focus();
    nameInputRef.current?.select();
  }, [editing, f.file_name]);

  // The only place the row can be picked up — see `onMouseDown`'s doc
  // comment above for why this replaced a whole-row grab cursor. Not an
  // `IconButton`: it has no click of its own (only a drag), so it skips that
  // component's hover background/click-target box and just changes the
  // cursor, kept as narrow as the glyph itself so this column doesn't widen
  // the table next to the other 32px icon columns.
  const dragHandleCell = (
    <td className="drag-handle" onMouseDown={onMouseDown}>
      <GripVertical size={11} strokeWidth={1.6} aria-hidden="true" />
    </td>
  );

  const revealCell = (
    <td className="reveal">
      <IconButton
        // Lucide's Search glyph isn't vertically balanced within its own
        // 24×24 box: the magnifying glass's circle (the icon's actual visual
        // weight) is centered a full unit above the box's true center, with
        // only the thin diagonal handle reaching down to square up the
        // bounding box on paper — so flush-centering the box (which is
        // otherwise pixel-exact here, 20 − 14 = 6, an even remainder) still
        // reads as sitting slightly high. Same class of fix as the play
        // button's own icon nudge (PlaybackTransport.tsx): nudge the glyph,
        // not the box.
        icon={<Search size={14} strokeWidth={1.8} className="reveal-icon" />}
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
                  (see .play-btn.playing:hover in ResultRow.css) so the row
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

  const { ext } = splitStem(f.file_name);
  // Mouse events inside the field are stopped the same way the row's own
  // buttons already are (`stop`, above) — without it, clicking to place the
  // caret would also arm `onRowMouseDown`'s drag session, and dragging to
  // select text inside the field would try to reorder rows instead.
  const nameCell = editing ? (
    <td className="fname fname-editing">
      <input
        ref={nameInputRef}
        type="text"
        className="fname-input"
        value={draftName}
        readOnly={renameBusy}
        onMouseDown={stop}
        onClick={stop}
        onChange={(ev) => setDraftName(ev.target.value)}
        onKeyDown={(ev) => {
          if (ev.key === "Escape") {
            ev.preventDefault();
            onCancelRename();
          } else if (ev.key === "Enter") {
            ev.preventDefault();
            onSubmitRename(draftName);
          }
        }}
        onBlur={onCancelRename}
      />
      {ext && <span className="fname-ext">{ext}</span>}
    </td>
  ) : (
    // A missing file gets a broken-link glyph and a struck-through name — the
    // one visual idiom that already means "this is gone" without a legend.
    // The legend exists anyway, once, in the summary strip above the table
    // ("N missing"), because a struck-through row on its own still leaves
    // *why* to guesswork.
    <td
      className={missing ? "fname fname-missing has-tip" : "fname has-tip"}
      title={missing ? `Not found: ${f.path}` : f.path}
    >
      {missing && <Unlink className="fname-missing-icon" size={12} strokeWidth={1.8} />}
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
    // drag handle, then reveal + thumbnail + name precede, delete trails.
    return (
      <tr
        data-path={f.path}
        className={rowClass(selected, dragging, dropEdge)}
        onClick={onRowClick}
      >
        {dragHandleCell}
        {revealCell}
        <td className="thumb" />
        {nameCell}
        <td colSpan={columns.length} className="c-bad">
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
      onClick={onRowClick}
    >
      {dragHandleCell}
      {revealCell}
      {thumbCell}
      {nameCell}
      {columns.map((col) => (
        <Fragment key={col.key}>{col.render(f, tag)}</Fragment>
      ))}
      {deleteCell}
    </tr>
  );
}

// The results table: header, rows, selection clicks and drag-to-reorder.
//
// Two columns are conditional rather than always-present-but-empty: MD5 only
// means something when the analysis actually included FLAC files, and Quality
// only when at least one file earned a badge.

import { useMemo, useRef } from "react";

import type { CoverArt, FileAnalysis } from "../types";
import { ResultRow } from "./ResultRow";
import { useRowDrag } from "./useRowDrag";
import type { SelectionModifiers } from "./useSelection";

export interface ResultsTableProps {
  files: FileAnalysis[];
  covers: Map<string, CoverArt | null>;
  nowPlaying: { path: string; requestId: number } | null;
  selectedPaths: string[];
  onSelectRow: (path: string, ev: SelectionModifiers) => void;
  onReorder: (order: string[]) => void;
  onReveal: (path: string) => void;
  onTogglePlay: (path: string) => void;
  onDelete: (path: string, isSelected: boolean) => void;
}

export function ResultsTable({
  files,
  covers,
  nowPlaying,
  selectedPaths,
  onSelectRow,
  onReorder,
  onReveal,
  onTogglePlay,
  onDelete,
}: ResultsTableProps) {
  const tableRef = useRef<HTMLTableElement>(null);
  const orderedPaths = useMemo(() => files.map((f) => f.path), [files]);

  const { dragState, onRowMouseDown, consumeClickSuppression } = useRowDrag({
    tableRef,
    orderedPaths,
    selectedPaths,
    onReorder,
  });

  const showMd5 = files.some((f) => f.flac_md5 != null);
  const showBadge = files.some((f) => f.badge != null);

  const headers = [
    "", // reveal button
    "", // thumbnail / play button
    "File",
    "Format",
    ...(showBadge ? ["Quality"] : []),
    "Rate",
    "Bits",
    "Real bits",
    "Length",
    "Size",
    "Detections",
    "Cutoff",
    "Ch",
    "Stereo",
    "Clipping",
    "True Peak",
    "Dynamics",
    ...(showMd5 ? ["MD5"] : []),
    "", // delete button
  ];

  const selected = new Set(selectedPaths);
  const dragging = new Set(dragState.active ? dragState.paths : []);

  return (
    <div className="table-wrap">
      <table ref={tableRef}>
        <thead>
          <tr>
            {headers.map((h, i) => (
              <th key={`${h}-${i}`}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {files.map((f) => (
            <ResultRow
              key={f.path}
              file={f}
              cover={covers.get(f.path)}
              playing={nowPlaying?.path === f.path}
              playingRequestId={nowPlaying?.path === f.path ? nowPlaying.requestId : null}
              selected={selected.has(f.path)}
              dragging={dragging.has(f.path)}
              dropEdge={
                dragState.dropTarget?.path === f.path
                  ? dragState.dropTarget.before
                    ? "before"
                    : "after"
                  : null
              }
              columnCount={headers.length}
              showMd5={showMd5}
              showBadge={showBadge}
              onReveal={() => onReveal(f.path)}
              onTogglePlay={() => onTogglePlay(f.path)}
              onDelete={() => onDelete(f.path, selected.has(f.path))}
              onMouseDown={(ev) => onRowMouseDown(f.path, ev)}
              onRowClick={(ev) => {
                if (consumeClickSuppression()) return;
                onSelectRow(f.path, ev);
              }}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

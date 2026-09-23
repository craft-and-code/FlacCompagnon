// The results table coordinates columns, selection and manual reorder.
// useTableWindow bounds rendering independently of the complete file list.

import { Fragment, useMemo, useRef, useState, type Ref } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";

import type { CoverArt, FileAnalysis, TagSet } from "../types";
import { ColumnMenu } from "./ColumnMenu";
import { dropZone } from "./dropZones";
import { ResultRow } from "./ResultRow";
import type { SortColumn, SortState } from "./tableSort";
import { useColumnDrag } from "./useColumnDrag";
import { useColumnPrefs } from "./useColumnPrefs";
import { useTableColumnWidths } from "./useTableColumnWidths";
import { useRowDrag } from "./useRowDrag";
import { useTableWindow, type TableScrollHandle } from "./useTableWindow";
import type { SelectionModifiers } from "./useSelection";
import "./ResultsTable.css";

// Columns that precede the reorderable ones and never move: the drag handle,
// reveal, thumbnail and filename cells. "File" sorts (there's a real column
// beneath it) but its position is fixed, same as the trailing delete column.
const LEAD_HEADERS: { label: string; sort?: SortColumn }[] = [
  { label: "" }, // drag handle
  { label: "" }, // reveal button
  { label: "" }, // thumbnail / play button
  { label: "File", sort: "file" },
];

// A slow second click renames; an ordinary double-click still just selects.
const RENAME_CLICK_GAP_MS = 500;

export interface ResultsTableProps {
  files: FileAnalysis[];
  covers: Map<string, CoverArt | null>;
  /// Backs the optional tag-derived columns (Artist, Album, ...) — the same
  /// cache the tag panel and thumbnail column already share, prefetched for
  /// every file in `App.tsx`, so showing one of these columns for the first
  /// time doesn't trigger a new read.
  tags: Map<string, TagSet | null>;
  nowPlaying: { path: string; requestId: number } | null;
  selectedPaths: string[];
  /// The parent's filtered/sorted list, shared with the playback queue.
  displayedFiles: FileAnalysis[];
  filterKey: string;
  scrollRef?: Ref<TableScrollHandle>;
  /// `null` means the natural/manual (drag-reordered) order — lifted up to
  /// App.tsx (not owned here) because Play/Previous/Next and the natural
  /// end-of-track auto-advance need to walk the exact same order this table
  /// is showing, not a copy only this component knows about. See
  /// tableSort.ts's doc comment.
  sort: SortState | null;
  onSortChange: (column: SortColumn) => void;
  /// The one row currently showing an editable name field, or `null`.
  editingPath: string | null;
  renameBusy: boolean;
  /// Paths that no longer resolve on disk — see useMissingFiles.ts.
  missing: Set<string>;
  onSelectRow: (path: string, ev: SelectionModifiers) => void;
  /// Opens the rename field for `path` — only ever called for the sole
  /// selected row, well after the click that selected it (see
  /// `RENAME_CLICK_GAP_MS`).
  onStartRename: (path: string) => void;
  onCancelRename: () => void;
  onSubmitRename: (path: string, newStem: string) => void;
  onReorder: (order: string[]) => void;
  onReveal: (path: string) => void;
  onTogglePlay: (path: string) => void;
  onDelete: (path: string, isSelected: boolean) => void;
}

export function ResultsTable({
  files,
  covers,
  tags,
  nowPlaying,
  selectedPaths,
  displayedFiles,
  filterKey,
  scrollRef,
  sort,
  onSortChange,
  editingPath,
  renameBusy,
  missing,
  onSelectRow,
  onStartRename,
  onCancelRename,
  onSubmitRename,
  onReorder,
  onReveal,
  onTogglePlay,
  onDelete,
}: ResultsTableProps) {
  const tableRef = useRef<HTMLTableElement>(null);
  const orderedPaths = useMemo(() => files.map((f) => f.path), [files]);

  const displayedPaths = useMemo(() => displayedFiles.map(f => f.path), [displayedFiles]);
  const { viewportRef, onScroll, indices, offsets } = useTableWindow(
    displayedPaths, JSON.stringify([filterKey, sort]), editingPath, tableRef, scrollRef,
  );
  const noMatches = files.length > 0 && displayedFiles.length === 0;

  // Tracks only the most recent click on *a* name cell — clicking a
  // different row's name in between resets eligibility for both, same as
  // Finder: the "second click" has to be the second click on that same row.
  const lastNameClick = useRef<{ path: string; time: number } | null>(null);

  const { dragState, onRowMouseDown, consumeClickSuppression } = useRowDrag({
    tableRef,
    orderedPaths,
    selectedPaths,
    onReorder,
  });

  const showMd5 = files.some((f) => f.flac_md5 != null);
  const showBadge = files.some((f) => f.badge != null);

  // Quality/MD5 additionally require relevant data, even when enabled.
  const { visibleColumns, menuRows, toggle, reorder } = useColumnPrefs();
  const middleColumns = visibleColumns.filter((col) => {
    if (col.conditional === "badge") return showBadge;
    if (col.conditional === "md5") return showMd5;
    return true;
  });
  const columnWidths = useTableColumnWidths(tableRef, middleColumns.map(col => col.key).join(","));

  const [colMenu, setColMenu] = useState<{ x: number; y: number } | null>(null);

  const {
    dragState: colDragState,
    onColumnMouseDown,
    consumeClickSuppression: consumeColumnClickSuppression,
  } = useColumnDrag({ onReorder: reorder });

  const selected = new Set(selectedPaths);
  const dragging = new Set(dragState.active ? dragState.paths : []);
  const totalColumns = LEAD_HEADERS.length + middleColumns.length + 1; // +1: delete button

  const sortIndicator = (col?: SortColumn) =>
    col &&
    sort?.column === col &&
    (sort.direction === "asc" ? (
      <ChevronDown size={12} strokeWidth={2.4} />
    ) : (
      <ChevronUp size={12} strokeWidth={2.4} />
    ));

  return (
    // Dropping audio anywhere on this list adds it, the same way the
    // empty-state dropzone this replaces does — see dropZones.ts.
    <div ref={viewportRef} onScroll={onScroll} className="table-wrap" {...dropZone("list")}>
      <table ref={tableRef} className={sort ? "sorted" : undefined} aria-rowcount={displayedFiles.length + 1}>
        <colgroup>{columnWidths.map((width, i) => <col key={i} style={{ width }} />)}</colgroup>
        <thead
          onContextMenu={(ev) => {
            ev.preventDefault();
            setColMenu({ x: ev.clientX, y: ev.clientY });
          }}
        >
          <tr>
            {LEAD_HEADERS.map((h, i) =>
              h.sort ? (
                <th key={`${h.label}-${i}`}>
                  <button
                    type="button"
                    className="th-sort"
                    onClick={() => onSortChange(h.sort as SortColumn)}
                  >
                    {h.label}
                    {sortIndicator(h.sort)}
                  </button>
                </th>
              ) : (
                <th key={`${h.label}-${i}`}>{h.label}</th>
              ),
            )}
            {middleColumns.map((col) => (
              <th
                key={col.key}
                data-col-key={col.key}
                className={
                  colDragState.key === col.key && colDragState.dragging
                    ? "col-dragging"
                    : colDragState.dropTarget?.key === col.key
                      ? colDragState.dropTarget.before
                        ? "col-drop-before"
                        : "col-drop-after"
                      : undefined
                }
                onMouseDown={(ev) => onColumnMouseDown(col.key, ev)}
              >
                {col.sort ? (
                  <button
                    type="button"
                    className="th-sort"
                    onClick={() => {
                      if (consumeColumnClickSuppression()) return;
                      onSortChange(col.sort as SortColumn);
                    }}
                  >
                    {col.label}
                    {sortIndicator(col.sort)}
                  </button>
                ) : (
                  col.label
                )}
              </th>
            ))}
            <th key="delete" />
          </tr>
        </thead>
        <tbody>
          {noMatches && (
            <tr>
              <td className="no-matches muted" colSpan={totalColumns}>
                No files match the filter.
              </td>
            </tr>
          )}
          {indices.map((index, position) => {
            const f = displayedFiles[index];
            const previousEnd = position === 0 ? 0 : offsets[indices[position - 1] + 1];
            return (
              <Fragment key={f.path}>
                {offsets[index] > previousEnd && <tr className="table-spacer" aria-hidden="true">
                  <td colSpan={totalColumns} style={{ height: offsets[index] - previousEnd }} />
                </tr>}
                <ResultRow
                  file={f}
                  rowIndex={index + 2}
                  cover={covers.get(f.path)}
                  tag={tags.get(f.path) ?? null}
                  columns={middleColumns}
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
                  editing={editingPath === f.path}
                  renameBusy={renameBusy}
                  missing={missing.has(f.path)}
                  onReveal={() => onReveal(f.path)}
                  onTogglePlay={() => onTogglePlay(f.path)}
                  onDelete={() => onDelete(f.path, selected.has(f.path))}
                  onCancelRename={onCancelRename}
                  onSubmitRename={(newStem) => onSubmitRename(f.path, newStem)}
                  onMouseDown={(ev) => {
                    // See the `sort` doc comment above: dragging only makes
                    // sense against the natural order the header isn't
                    // currently overriding.
                    if (!sort) onRowMouseDown(f.path, ev);
                  }}
                  onRowClick={(ev) => {
                    if (consumeClickSuppression()) return;

                    const clickedName = (ev.target as HTMLElement).closest(".fname") != null;
                    const now = ev.timeStamp;
                    const prev = lastNameClick.current;
                    const secondClick =
                      prev != null && prev.path === f.path && now - prev.time > RENAME_CLICK_GAP_MS;
                    lastNameClick.current = clickedName ? { path: f.path, time: now } : null;

                    const sole = selected.has(f.path) && selectedPaths.length === 1;
                    if (clickedName && sole && secondClick && editingPath !== f.path) {
                      onStartRename(f.path);
                      return;
                    }
                    onSelectRow(f.path, ev);
                  }}
                />
              </Fragment>
            );
          })}
          <tr className="table-spacer" aria-hidden="true">
            <td colSpan={totalColumns} style={{ height: offsets[displayedFiles.length] -
              (indices.length ? offsets[indices[indices.length - 1] + 1] : 0) }} />
          </tr>
        </tbody>
      </table>
      <ColumnMenu
        open={colMenu != null}
        x={colMenu?.x ?? 0}
        y={colMenu?.y ?? 0}
        rows={menuRows}
        onToggle={toggle}
        onClose={() => setColMenu(null)}
      />
    </div>
  );
}

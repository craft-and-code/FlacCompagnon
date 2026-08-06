// Manual row reordering.
//
// Tauri's native OS file-drop needs `dragDropEnabled: true`, which disables
// the browser's own HTML5 drag-and-drop inside the webview (dragstart/dragover
// /drop never fire). Since dropping folders onto the window is the app's main
// entry point, reordering is done with plain mouse events instead, which don't
// go through Tauri's interception at all.
//
// What the table renders — which rows are being dragged, where the insertion
// marker sits — is React state. Only two things stay imperative, because they
// are geometry rather than state: hit-testing the pointer, and the floating
// ghost, which is cloned from the live rows so it keeps their exact column
// widths without this hook having to know anything about the columns.

import { useCallback, useEffect, useRef, useState } from "react";

const DRAG_THRESHOLD_PX = 4;

// Beyond this many rows the ghost summarizes the rest as "+N", so a large
// selection's ghost doesn't become a full-height copy of the table.
const GHOST_STACK_CAP = 8;

export interface DropTarget {
  path: string;
  before: boolean;
}

export interface RowDragState {
  /// Rows being moved, in display order. Empty when no drag is in progress.
  paths: string[];
  /// True once the pointer has moved past the click threshold — before that
  /// it's still potentially a plain click.
  active: boolean;
  dropTarget: DropTarget | null;
}

const IDLE: RowDragState = { paths: [], active: false, dropTarget: null };

export interface UseRowDragArgs {
  tableRef: React.RefObject<HTMLTableElement | null>;
  /// Current display order — read at drag start and used to compute the new one.
  orderedPaths: string[];
  selectedPaths: string[];
  onReorder: (order: string[]) => void;
}

export function useRowDrag({ tableRef, orderedPaths, selectedPaths, onReorder }: UseRowDragArgs) {
  const [state, setState] = useState<RowDragState>(IDLE);

  // Everything the move/up handlers need without re-subscribing on each render.
  const session = useRef<{
    paths: string[];
    startX: number;
    startY: number;
    primary: string;
    active: boolean;
    ghost: HTMLElement | null;
    offsetX: number;
    offsetY: number;
    dropTarget: DropTarget | null;
    order: string[];
  } | null>(null);

  // Set when a real drag just ended, so the click that follows the mouseup
  // doesn't also change the selection.
  const suppressClick = useRef(false);

  const buildGhost = useCallback(
    (paths: string[], primary: string, rawOffsetX: number, rawOffsetY: number) => {
      const table = tableRef.current;
      if (!table) return null;

      const rowByPath = new Map<string, HTMLTableRowElement>();
      for (const row of table.tBodies[0]?.rows ?? []) {
        const p = row.getAttribute("data-path");
        if (p) rowByPath.set(p, row);
      }

      let shown = paths;
      let extra = 0;
      if (paths.length > GHOST_STACK_CAP) {
        const idx = paths.indexOf(primary);
        const start = Math.max(0, Math.min(idx, paths.length - GHOST_STACK_CAP));
        shown = paths.slice(start, start + GHOST_STACK_CAP);
        extra = paths.length - GHOST_STACK_CAP;
      }

      const primaryRow = rowByPath.get(primary);
      const rect = primaryRow?.getBoundingClientRect();
      const rowHeight = rect?.height ?? 0;

      const wrapper = document.createElement("div");
      wrapper.className = "row-drag-ghost";
      if (rect) wrapper.style.width = `${rect.width}px`;

      const ghostTable = document.createElement("table");
      const tbody = document.createElement("tbody");
      for (const p of shown) {
        const row = rowByPath.get(p);
        if (!row) continue;
        const clone = row.cloneNode(true) as HTMLTableRowElement;
        clone.classList.remove("dragging", "selected", "drop-before", "drop-after");
        for (let i = 0; i < clone.cells.length && i < row.cells.length; i++) {
          clone.cells[i].style.width = `${row.cells[i].getBoundingClientRect().width}px`;
        }
        tbody.appendChild(clone);
      }
      ghostTable.appendChild(tbody);
      wrapper.appendChild(ghostTable);

      if (extra > 0) {
        const more = document.createElement("div");
        more.className = "row-drag-more";
        more.textContent = `+${extra}`;
        wrapper.appendChild(more);
      }
      document.body.appendChild(wrapper);

      // The cursor grabbed the primary row, which may sit partway down the
      // stack — offset by its index so that row, not the top of the stack,
      // stays under the pointer.
      const indexInShown = Math.max(0, shown.indexOf(primary));
      return {
        el: wrapper,
        offsetX: rawOffsetX,
        offsetY: rawOffsetY + indexInShown * rowHeight,
      };
    },
    [tableRef],
  );

  useEffect(() => {
    const onMove = (ev: MouseEvent) => {
      const s = session.current;
      if (!s) return;

      if (!s.active) {
        if (Math.hypot(ev.clientX - s.startX, ev.clientY - s.startY) < DRAG_THRESHOLD_PX) return;
        s.active = true;
        const table = tableRef.current;
        const primaryRow = table?.querySelector<HTMLTableRowElement>(
          `tr[data-path="${CSS.escape(s.primary)}"]`,
        );
        const rect = primaryRow?.getBoundingClientRect();
        const built = buildGhost(
          s.paths,
          s.primary,
          s.startX - (rect?.left ?? s.startX),
          s.startY - (rect?.top ?? s.startY),
        );
        if (built) {
          s.ghost = built.el;
          s.offsetX = built.offsetX;
          s.offsetY = built.offsetY;
        }
        setState({ paths: s.paths, active: true, dropTarget: null });
      }

      if (s.ghost) {
        s.ghost.style.left = `${ev.clientX - s.offsetX}px`;
        s.ghost.style.top = `${ev.clientY - s.offsetY}px`;
      }

      const hovered = (
        document.elementFromPoint(ev.clientX, ev.clientY) as HTMLElement | null
      )?.closest<HTMLTableRowElement>("tr[data-path]");
      const path = hovered?.getAttribute("data-path");
      let next: DropTarget | null = null;
      if (hovered && path && !s.paths.includes(path)) {
        const r = hovered.getBoundingClientRect();
        next = { path, before: ev.clientY < r.top + r.height / 2 };
      }
      // Only re-render when the marker actually moves to a different edge.
      if (next?.path !== s.dropTarget?.path || next?.before !== s.dropTarget?.before) {
        s.dropTarget = next;
        setState({ paths: s.paths, active: true, dropTarget: next });
      }
    };

    const onUp = () => {
      const s = session.current;
      session.current = null;
      if (!s) return;
      s.ghost?.remove();
      setState(IDLE);
      if (!s.active) return; // no real movement — a plain click

      suppressClick.current = true;
      const target = s.dropTarget;
      if (!target) return;

      // Pull every dragged row out first, then reinsert them as one contiguous
      // block at the drop point — this keeps their relative order even when
      // the group wasn't contiguous to begin with.
      const remaining = s.order.filter((p) => !s.paths.includes(p));
      let to = remaining.indexOf(target.path);
      if (to === -1) return;
      if (!target.before) to += 1;
      remaining.splice(to, 0, ...s.paths);
      onReorder(remaining);
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
  }, [buildGhost, onReorder, tableRef]);

  const onRowMouseDown = useCallback(
    (path: string, ev: React.MouseEvent) => {
      // Left button only. A right-click's mouseup can be swallowed once the
      // native context menu takes over the event loop, which used to leave the
      // drag stuck active with its ghost row still on screen.
      if (ev.button !== 0) return;

      // Dragging a row that's part of the current multi-selection moves the
      // whole selection together, in display order; any other row moves alone.
      const paths =
        selectedPaths.includes(path) && selectedPaths.length > 1
          ? orderedPaths.filter((p) => selectedPaths.includes(p))
          : [path];

      // Belt-and-braces against text selection on top of the CSS
      // `user-select: none` — WKWebView doesn't always honor it once a mouse
      // drag is under way.
      ev.preventDefault();

      session.current = {
        paths,
        startX: ev.clientX,
        startY: ev.clientY,
        primary: path,
        active: false,
        ghost: null,
        offsetX: 0,
        offsetY: 0,
        dropTarget: null,
        order: orderedPaths,
      };
    },
    [orderedPaths, selectedPaths],
  );

  /// True when the click that just fired was the tail end of a real drag and
  /// so shouldn't also change the selection. Clears the flag either way.
  const consumeClickSuppression = useCallback(() => {
    const v = suppressClick.current;
    suppressClick.current = false;
    return v;
  }, []);

  return { dragState: state, onRowMouseDown, consumeClickSuppression };
}

// Manual column reordering: press and hold a header cell, then drag it left
// or right to reposition it — mirrors useRowDrag.ts's row reordering, plain
// mouse events for the same reason (Tauri's dragDropEnabled disables the
// browser's own HTML5 drag-and-drop inside the webview).
//
// Header cells double as sort triggers (a plain click already means "sort by
// this column"), so arming a drag can't use a movement threshold the way
// useRowDrag.ts does — a couple of pixels of jitter on an ordinary sort click
// would misfire as a drag. The trigger here is time instead: hold past
// LONG_PRESS_MS before anything happens, and a click that releases before
// then is left alone, completely undisturbed. Once armed, the sort click
// that would otherwise follow is suppressed too (see `consumeClickSuppression`),
// so a completed drag never also re-sorts the table it just reordered.
//
// No floating ghost clone like useRowDrag.ts's — a single header cell is
// small enough that dimming the source and marking the drop edge on the
// target header reads clearly without one, and it's one column at a time,
// never a multi-selection.

import { useCallback, useEffect, useRef, useState } from "react";
import type { ColumnKey } from "./resultColumns";

const LONG_PRESS_MS = 350;

// Cancels the pending long-press if the pointer wanders this far before the
// timer fires — beyond this the press reads as the start of an ordinary
// pointer move, not a steady hold, so it's left as a plain click instead.
const PRESS_JITTER_PX = 6;

export interface ColumnDropTarget {
  key: ColumnKey;
  before: boolean;
}

export interface ColumnDragState {
  key: ColumnKey | null;
  /// True once the long-press has elapsed — the header is "picked up" even
  /// before the pointer has actually moved.
  armed: boolean;
  /// True once the pointer has moved while armed — only then is there a
  /// `dropTarget` to show.
  dragging: boolean;
  dropTarget: ColumnDropTarget | null;
}

const IDLE: ColumnDragState = { key: null, armed: false, dragging: false, dropTarget: null };

export interface UseColumnDragArgs {
  onReorder: (key: ColumnKey, target: ColumnDropTarget) => void;
}

export function useColumnDrag({ onReorder }: UseColumnDragArgs) {
  const [state, setState] = useState<ColumnDragState>(IDLE);

  // Everything the move/up handlers need without re-subscribing on each
  // render — same shape as useRowDrag.ts's own `session` ref.
  const session = useRef<{
    key: ColumnKey;
    startX: number;
    startY: number;
    armed: boolean;
    dragging: boolean;
    timer: ReturnType<typeof setTimeout> | null;
    dropTarget: ColumnDropTarget | null;
  } | null>(null);

  // Set once a long-press has armed, so the click that follows the mouseup
  // doesn't also trigger that header's sort — whether or not the press ever
  // turned into an actual drag.
  const suppressClick = useRef(false);

  useEffect(() => {
    const onMove = (ev: MouseEvent) => {
      const s = session.current;
      if (!s) return;

      if (!s.armed) {
        if (Math.hypot(ev.clientX - s.startX, ev.clientY - s.startY) > PRESS_JITTER_PX) {
          if (s.timer) clearTimeout(s.timer);
          session.current = null;
        }
        return;
      }

      if (!s.dragging) {
        s.dragging = true;
        setState({ key: s.key, armed: true, dragging: true, dropTarget: null });
      }

      const hovered = (
        document.elementFromPoint(ev.clientX, ev.clientY) as HTMLElement | null
      )?.closest<HTMLElement>("th[data-col-key]");
      const key = hovered?.getAttribute("data-col-key") as ColumnKey | undefined;
      let next: ColumnDropTarget | null = null;
      if (hovered && key && key !== s.key) {
        const r = hovered.getBoundingClientRect();
        next = { key, before: ev.clientX < r.left + r.width / 2 };
      }
      // Only re-render when the marker actually moves to a different edge.
      if (next?.key !== s.dropTarget?.key || next?.before !== s.dropTarget?.before) {
        s.dropTarget = next;
        setState({ key: s.key, armed: true, dragging: true, dropTarget: next });
      }
    };

    const onUp = () => {
      const s = session.current;
      session.current = null;
      if (!s) return;
      if (s.timer) clearTimeout(s.timer);
      setState(IDLE);
      if (s.armed) suppressClick.current = true;
      if (s.dragging && s.dropTarget) onReorder(s.key, s.dropTarget);
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      if (session.current?.timer) clearTimeout(session.current.timer);
      session.current = null;
    };
  }, [onReorder]);

  const onColumnMouseDown = useCallback((key: ColumnKey, ev: React.MouseEvent) => {
    if (ev.button !== 0) return;

    // Belt-and-braces against text selection on top of the CSS
    // `user-select: none` — WKWebView doesn't always honor it once a mouse
    // drag is under way (same reasoning as useRowDrag.ts's own mousedown).
    ev.preventDefault();

    const startX = ev.clientX;
    const startY = ev.clientY;
    const timer = setTimeout(() => {
      const s = session.current;
      if (!s) return;
      s.armed = true;
      setState({ key: s.key, armed: true, dragging: false, dropTarget: null });
    }, LONG_PRESS_MS);

    session.current = {
      key,
      startX,
      startY,
      armed: false,
      dragging: false,
      timer,
      dropTarget: null,
    };
  }, []);

  /// True when the click that just fired was the tail end of an armed
  /// press-and-hold and so shouldn't also sort the column. Clears the flag
  /// either way.
  const consumeClickSuppression = useCallback(() => {
    const v = suppressClick.current;
    suppressClick.current = false;
    return v;
  }, []);

  return { dragState: state, onColumnMouseDown, consumeClickSuppression };
}

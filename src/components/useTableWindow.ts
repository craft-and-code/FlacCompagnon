import { useImperativeHandle, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Ref, RefObject } from "react";
import { nearestRowScroll, rowOffsets, tableWindow, windowIndices } from "./tableWindow";

export interface TableScrollHandle {
  scrollToPath: (path: string) => void;
}

/// Only the viewport's rows enter React/the DOM. The data remains complete in
/// the owner, including offscreen selections and playback/export ordering.
export function useTableWindow(
  paths: string[],
  resetKey: string,
  editingPath: string | null,
  tableRef: RefObject<HTMLTableElement | null>,
  scrollRef?: Ref<TableScrollHandle>,
) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const heights = useRef(new Map<string, number>());
  const requestedPath = useRef<string | null>(null);
  const [revision, setRevision] = useState(0);
  const [viewport, setViewport] = useState({ key: resetKey, top: 0, height: 600 });
  const offsets = useMemo(() => rowOffsets(paths, heights.current), [paths, revision]);
  // Apply a new filter and its scroll position in the same paint, never a
  // frame showing the new shorter list at the old (possibly huge) offset.
  const range = tableWindow(offsets, viewport.key === resetKey ? viewport.top : 0, viewport.height);
  const indices = windowIndices(range.start, range.end, editingPath ? paths.indexOf(editingPath) : -1);

  const readViewport = () => {
    const el = viewportRef.current;
    if (!el) return;
    const height = Math.max(1, el.clientHeight - (tableRef.current?.tHead?.offsetHeight ?? 0));
    setViewport(prev => prev.key === resetKey && prev.top === el.scrollTop && prev.height === height
      ? prev : { key: resetKey, top: el.scrollTop, height });
  };

  useLayoutEffect(() => {
    const el = viewportRef.current;
    const table = tableRef.current;
    if (!el || !table) return;
    if (viewport.key !== resetKey) requestedPath.current = null;
    if (viewport.key !== resetKey || viewport.top !== range.top) el.scrollTop = range.top;
    readViewport();
    const measure = () => {
      let changed = false;
      for (const row of table.tBodies[0]?.rows ?? []) {
        const path = row.dataset.path;
        const height = row.getBoundingClientRect().height;
        if (path && height > 0 && heights.current.get(path) !== height) {
          heights.current.set(path, height);
          changed = true;
        }
      }
      if (changed) setRevision(value => value + 1);
      return changed;
    };
    const changed = measure();
    // The first jump uses estimates for unseen rows. Once mounted, finish
    // against actual heights so a wrapped keyboard target is fully visible.
    if (!changed && requestedPath.current) {
      const index = paths.indexOf(requestedPath.current);
      requestedPath.current = null;
      if (index >= 0) el.scrollTop = nearestRowScroll(
        offsets[index], offsets[index + 1], el.scrollTop, viewport.height,
      );
      readViewport();
    }
    const observer = new ResizeObserver(() => { readViewport(); measure(); });
    observer.observe(el);
    for (const row of table.tBodies[0]?.rows ?? []) if (row.dataset.path) observer.observe(row);
    return () => observer.disconnect();
  });

  useImperativeHandle(scrollRef, () => ({
    scrollToPath(path) {
      const el = viewportRef.current;
      const index = paths.indexOf(path);
      if (!el || index < 0) return;
      const top = nearestRowScroll(offsets[index], offsets[index + 1], el.scrollTop, viewport.height);
      requestedPath.current = top === el.scrollTop ? null : path;
      el.scrollTop = top;
      readViewport();
    },
  }));

  return { viewportRef, onScroll: readViewport, indices, offsets };
}

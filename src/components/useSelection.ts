// Finder/Explorer-style row selection.
//
// The selection keeps click order, not display order: building a playlist from
// a selection follows the order you picked the tracks in. Every other consumer
// (the tag panel's bulk edit) only cares about membership.

import { useCallback, useState } from "react";

export interface SelectionModifiers {
  shiftKey: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
}

export function useSelection(orderedPaths: string[]) {
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  // The last row clicked without a modifier — Shift+click extends from here.
  const [anchor, setAnchor] = useState<string | null>(null);

  const selectRow = useCallback(
    (path: string, ev: SelectionModifiers) => {
      if (ev.shiftKey && anchor) {
        const a = orderedPaths.indexOf(anchor);
        const b = orderedPaths.indexOf(path);
        if (a !== -1 && b !== -1) {
          const [lo, hi] = a < b ? [a, b] : [b, a];
          setSelectedPaths(orderedPaths.slice(lo, hi + 1));
          return;
        }
        setSelectedPaths([path]);
        setAnchor(path);
        return;
      }
      if (ev.metaKey || ev.ctrlKey) {
        setSelectedPaths((prev) =>
          prev.includes(path) ? prev.filter((p) => p !== path) : [...prev, path],
        );
        setAnchor(path);
        return;
      }
      setSelectedPaths([path]);
      setAnchor(path);
    },
    [anchor, orderedPaths],
  );

  const clearSelection = useCallback(() => {
    setSelectedPaths([]);
    setAnchor(null);
  }, []);

  /// Drops paths that no longer exist (a deleted row, a fresh analysis).
  const pruneSelection = useCallback((present: Set<string>) => {
    setSelectedPaths((prev) => prev.filter((p) => present.has(p)));
    setAnchor((a) => (a && present.has(a) ? a : null));
  }, []);

  return { selectedPaths, selectRow, clearSelection, pruneSelection };
}

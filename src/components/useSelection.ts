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
  // The moving end of the selection: the row an extension last reached.
  //
  // Distinct from both `anchor` (the fixed end) and from `selectedPaths`'s
  // last entry, and that last distinction is the one that matters. A
  // Shift-extended selection is stored in *display* order, so its last entry
  // is always its bottom row — fine while extending downwards, useless while
  // extending upwards, where the bottom row is the anchor and never moves.
  // Keyboard extension read that entry and so could never get past two rows
  // going up.
  const [cursor, setCursor] = useState<string | null>(null);

  const selectRow = useCallback(
    (path: string, ev: SelectionModifiers) => {
      if (ev.shiftKey && anchor) {
        const a = orderedPaths.indexOf(anchor);
        const b = orderedPaths.indexOf(path);
        if (a !== -1 && b !== -1) {
          const [lo, hi] = a < b ? [a, b] : [b, a];
          setSelectedPaths(orderedPaths.slice(lo, hi + 1));
          // The anchor stays put — that is what makes it an anchor — but the
          // cursor follows the row just reached.
          setCursor(path);
          return;
        }
        setSelectedPaths([path]);
        setAnchor(path);
        setCursor(path);
        return;
      }
      if (ev.metaKey || ev.ctrlKey) {
        setSelectedPaths((prev) =>
          prev.includes(path) ? prev.filter((p) => p !== path) : [...prev, path],
        );
        setAnchor(path);
        setCursor(path);
        return;
      }
      setSelectedPaths([path]);
      setAnchor(path);
      setCursor(path);
    },
    [anchor, orderedPaths],
  );

  const clearSelection = useCallback(() => {
    setSelectedPaths([]);
    setAnchor(null);
    setCursor(null);
  }, []);

  /// Selects every row currently in `orderedPaths` — the whole list, not just
  /// whatever the search filter happens to be showing (that filter is
  /// display-only, see App.tsx's `visiblePaths`, and never reaches this far).
  const selectAll = useCallback(() => {
    setSelectedPaths(orderedPaths);
    const last = orderedPaths.length > 0 ? orderedPaths[orderedPaths.length - 1] : null;
    setAnchor(last);
    setCursor(last);
  }, [orderedPaths]);

  /// Selects everything currently unselected and drops everything currently
  /// selected — same "whole list, not just the filtered view" scope as
  /// `selectAll` above, for the same reason (the search filter is
  /// display-only).
  const invertSelection = useCallback(() => {
    const selected = new Set(selectedPaths);
    const next = orderedPaths.filter((p) => !selected.has(p));
    setSelectedPaths(next);
    const last = next.length > 0 ? next[next.length - 1] : null;
    setAnchor(last);
    setCursor(last);
  }, [orderedPaths, selectedPaths]);

  /// Drops paths that no longer exist (a deleted row, a fresh analysis).
  const pruneSelection = useCallback((present: Set<string>) => {
    setSelectedPaths((prev) => prev.filter((p) => present.has(p)));
    setAnchor((a) => (a && present.has(a) ? a : null));
    setCursor((c) => (c && present.has(c) ? c : null));
  }, []);

  /// A renamed file's identity changes mid-selection — without this, the
  /// pruning above (run right after, once the file list catches up) would
  /// just drop `oldPath` as "no longer present" instead of following it to
  /// `newPath`, silently closing the tag panel on the file the user was
  /// looking at the moment they renamed it.
  const replacePath = useCallback((oldPath: string, newPath: string) => {
    setSelectedPaths((prev) => prev.map((p) => (p === oldPath ? newPath : p)));
    setAnchor((a) => (a === oldPath ? newPath : a));
    setCursor((c) => (c === oldPath ? newPath : c));
  }, []);

  return {
    selectedPaths,
    /// The moving end of the selection — what keyboard navigation steps from.
    cursor,
    selectRow,
    selectAll,
    invertSelection,
    clearSelection,
    pruneSelection,
    replacePath,
  };
}

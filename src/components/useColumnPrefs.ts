// Which of the results table's toggleable columns are shown, and in what
// order — visibility set from the header's right-click menu (ColumnMenu.tsx),
// order set by dragging a header cell directly (useColumnDrag.ts), persisted
// the same way useTheme.ts persists the theme choice: localStorage, wrapped
// since a webview with storage disabled would otherwise throw on startup.
//
// A column a saved preference has never seen falls back to its own
// `defaultVisible` rather than being silently hidden or shown — the only way
// "new columns start hidden, already-shown ones stay shown" can hold once a
// column that didn't exist yet in an older version of the app ships: it has
// to be told apart from one the user deliberately hid, which a plain
// `Set<ColumnKey>` of "currently visible" can't do on its own.

import { useCallback, useEffect, useState } from "react";
import { ALL_COLUMNS, type ColumnDef, type ColumnKey } from "./resultColumns";

const KEY = "resultColumns";

interface StoredPrefs {
  order: string[];
  hidden: string[];
}

interface State {
  order: ColumnKey[];
  hidden: Set<ColumnKey>;
}

function isColumnKey(k: string): k is ColumnKey {
  return ALL_COLUMNS.some((c) => c.key === k);
}

function load(): StoredPrefs {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<StoredPrefs>;
      if (Array.isArray(parsed.order) && Array.isArray(parsed.hidden)) {
        return { order: parsed.order, hidden: parsed.hidden };
      }
    }
  } catch {
    /* ignore */
  }
  return { order: [], hidden: [] };
}

/// Reconciles a saved (possibly stale) preference against the columns this
/// version of the app actually knows about: drops keys that no longer exist,
/// inserts new columns at their requested neighbour (or the end), and applies each new column's
/// own `defaultVisible` since a saved `hidden` list from before it existed
/// obviously never mentions it either way.
export function reconcile(stored: StoredPrefs): State {
  const order = stored.order.filter(isColumnKey);
  const seen = new Set(order);
  const hidden = new Set(stored.hidden.filter(isColumnKey));
  for (const col of ALL_COLUMNS) {
    if (!seen.has(col.key)) {
      const anchor = col.initialAfter ? order.indexOf(col.initialAfter) : -1;
      order.splice(anchor < 0 ? order.length : anchor + 1, 0, col.key);
      if (!col.defaultVisible) hidden.add(col.key);
    }
  }
  return { order, hidden };
}

function persist(state: State) {
  try {
    localStorage.setItem(
      KEY,
      JSON.stringify({ order: state.order, hidden: [...state.hidden] } satisfies StoredPrefs),
    );
  } catch {
    /* ignore */
  }
}

export function useColumnPrefs() {
  const [state, setState] = useState<State>(() => reconcile(load()));

  useEffect(() => persist(state), [state]);

  const toggle = useCallback((key: ColumnKey) => {
    setState((s) => {
      const hidden = new Set(s.hidden);
      if (hidden.has(key)) hidden.delete(key);
      else hidden.add(key);
      return { ...s, hidden };
    });
  }, []);

  // Drag-to-reorder (useColumnDrag.ts) reports where the dragged column was
  // dropped relative to another one, rather than a fixed step — mirrors
  // useRowDrag.ts's own "pull it out, splice it back in next to the target"
  // logic for rows.
  const reorder = useCallback((key: ColumnKey, target: { key: ColumnKey; before: boolean }) => {
    setState((s) => {
      if (key === target.key) return s;
      const remaining = s.order.filter((k) => k !== key);
      let to = remaining.indexOf(target.key);
      if (to === -1) return s;
      if (!target.before) to += 1;
      remaining.splice(to, 0, key);
      return { ...s, order: remaining };
    });
  }, []);

  const reset = useCallback(() => setState(reconcile({ order: [], hidden: [] })), []);

  const byKey = new Map(ALL_COLUMNS.map((c) => [c.key, c]));
  const inOrder: ColumnDef[] = state.order
    .map((k) => byKey.get(k))
    .filter((c): c is ColumnDef => c != null);
  const visibleColumns = inOrder.filter((c) => !state.hidden.has(c.key));
  const menuRows = inOrder.map((col) => ({ col, visible: !state.hidden.has(col.key) }));

  return { visibleColumns, menuRows, toggle, reorder, reset };
}

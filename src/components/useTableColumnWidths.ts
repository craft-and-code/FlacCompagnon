import { useLayoutEffect, useState, type RefObject } from "react";

/// Retain measured column widths as rows leave the viewport. Without this,
/// table auto-layout narrows columns whenever their longest cell unmounts.
/// New content can still widen a column; toggling columns starts a new layout.
export function useTableColumnWidths(tableRef: RefObject<HTMLTableElement | null>, key: string) {
  const [layout, setLayout] = useState<{ key: string; widths: number[] }>({ key, widths: [] });
  const widths = layout.key === key ? layout.widths : [];
  useLayoutEffect(() => {
    const cells = tableRef.current?.tHead?.rows[0]?.cells;
    if (!cells) return;
    const measured = Array.from(cells, (cell, i) =>
      Math.max(widths[i] ?? 0, cell.getBoundingClientRect().width));
    if (measured.length !== widths.length || measured.some((w, i) => w > widths[i] + 0.5)) {
      setLayout({ key, widths: measured });
    }
  });
  return widths;
}

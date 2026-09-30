import { useLayoutEffect, useState, type RefObject } from "react";

/// Capture the table's natural widths once a result set is rendered.
///
/// The virtual window deliberately replaces offscreen rows with spacers. Letting
/// table auto-layout keep measuring those replacement rows makes a column jump
/// whenever a longer cell enters the viewport. A new result set or a column
/// change gets a fresh natural layout; scrolling does not.
export function useTableColumnWidths(tableRef: RefObject<HTMLTableElement | null>, key: string) {
  const [layout, setLayout] = useState<{ key: string; widths: number[] }>({ key, widths: [] });
  const widths = layout.key === key ? layout.widths : [];
  useLayoutEffect(() => {
    if (layout.key === key && widths.length > 0) return;
    const cells = tableRef.current?.tHead?.rows[0]?.cells;
    if (!cells) return;
    const measured = Array.from(cells, (cell) => cell.getBoundingClientRect().width);
    if (measured.length > 0) setLayout({ key, widths: measured });
  }, [key, layout.key, tableRef, widths.length]);
  return widths;
}

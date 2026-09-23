// Pixel offsets support wrapped detection cells and shorter error rows without
// imposing a new row height on the table. Unvisited rows start at an estimate.
export function rowOffsets(paths: readonly string[], heights: ReadonlyMap<string, number>, estimate = 36) {
  const offsets = [0];
  for (const path of paths) offsets.push(offsets[offsets.length - 1] + (heights.get(path) ?? estimate));
  return offsets;
}

function rowAt(offsets: readonly number[], position: number) {
  let lo = 0;
  let hi = offsets.length - 1;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (offsets[mid] <= position) lo = mid;
    else hi = mid - 1;
  }
  return Math.min(lo, Math.max(0, offsets.length - 2));
}

/// Rows covering the viewport plus a small buffer, even after a list shrinks.
export function tableWindow(offsets: readonly number[], scrollTop: number, height: number, buffer = 300) {
  const count = offsets.length - 1;
  const total = offsets[count];
  const top = Math.max(0, Math.min(scrollTop, total - height));
  return {
    start: count ? rowAt(offsets, Math.max(0, top - buffer)) : 0,
    end: count ? Math.min(count, rowAt(offsets, top + height + buffer) + 1) : 0,
    top,
  };
}

/// Keep an active rename mounted without rendering every intervening row.
export function windowIndices(start: number, end: number, pinned: number) {
  const indices = Array.from({ length: end - start }, (_, i) => start + i);
  if (pinned >= 0 && pinned < start) indices.unshift(pinned);
  if (pinned >= end && pinned >= 0) indices.push(pinned);
  return indices;
}

/// Reveal a keyboard target with the smallest scroll, using its measured size.
export function nearestRowScroll(top: number, bottom: number, scrollTop: number, height: number) {
  if (top < scrollTop || bottom - top > height) return top;
  return bottom > scrollTop + height ? bottom - height : scrollTop;
}

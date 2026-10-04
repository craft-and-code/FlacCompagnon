// Row membership must stay linear for large multi-selections. The array is
// still in display order because dragging and exports share that ordering.

export interface RowDropTarget {
  path: string;
  before: boolean;
}

export function draggedPaths(order: string[], selection: string[], primary: string): string[] {
  const selected = new Set(selection);
  return selected.has(primary) && selected.size > 1
    ? order.filter((path) => selected.has(path))
    : [primary];
}

export function reorderPaths(
  order: string[],
  moving: string[],
  target: RowDropTarget,
): string[] | null {
  const moved = new Set(moving);
  const remaining = order.filter((path) => !moved.has(path));
  const index = remaining.indexOf(target.path);
  if (index < 0) return null;
  const to = index + (target.before ? 0 : 1);
  // Array spread does not pass one function argument per moved row, unlike
  // splice(...moving), which exceeds the engine's argument limit on libraries.
  return [...remaining.slice(0, to), ...moving, ...remaining.slice(to)];
}

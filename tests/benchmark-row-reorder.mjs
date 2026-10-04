import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";
import { loadTypeScript } from "./load-typescript.mjs";

const { draggedPaths, reorderPaths } = await loadTypeScript(
  new URL("../src/components/rowReorder.ts", import.meta.url),
);
const order = Array.from({ length: 10000 }, (_, index) => `file-${index}`);
const selection = order.filter((_, index) => index % 2 === 0);
const iterations = 10;

// The pre-audit implementation, retained here solely as the comparison case.
function previous() {
  const moving = order.filter((path) => selection.includes(path));
  const remaining = order.filter((path) => !moving.includes(path));
  remaining.splice(remaining.length, 0, ...moving);
  return remaining;
}

function current() {
  const moving = draggedPaths(order, selection, order[0]);
  return reorderPaths(order, moving, { path: order.at(-1), before: false });
}

function measure(action) {
  const start = performance.now();
  for (let i = 0; i < iterations; i++) action();
  return (performance.now() - start) / iterations;
}

assert.deepEqual(previous(), current()); // Also warms both paths once.
console.log(
  JSON.stringify(
    {
      files: order.length,
      selected: selection.length,
      iterations,
      previous_ms: measure(previous),
      current_ms: measure(current),
    },
    null,
    2,
  ),
);

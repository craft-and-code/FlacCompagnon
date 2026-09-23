import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { rowOffsets, tableWindow, windowIndices, nearestRowScroll } = await loadTypeScript(
  new URL("../src/components/tableWindow.ts", import.meta.url),
);

test("clearing a filter keeps rendering bounded for ten thousand files", () => {
  for (const count of [20, 2000, 10000]) {
    const offsets = Array.from({ length: count + 1 }, (_, i) => i * 36);
    const { start, end } = tableWindow(offsets, 0, 600);
    assert.equal(start, 0);
    assert.ok(end <= 26);
    assert.ok(offsets[end] >= Math.min(600, count * 36));
  }
});

test("filtering a deeply scrolled list never produces a blank window", () => {
  assert.deepEqual(tableWindow([0, 36, 72], 300000, 600), { start: 0, end: 2, top: 0 });
  assert.deepEqual(tableWindow([0], 300000, 600), { start: 0, end: 0, top: 0 });
});

test("wrapped cells and error rows retain their own heights after sorting", () => {
  const heights = new Map([["error", 31], ["wrapped", 60]]);
  const offsets = rowOffsets(["error", "wrapped", "regular"], heights);
  assert.deepEqual(offsets, [0, 31, 91, 127]);
  assert.deepEqual(tableWindow(offsets, 32, 20, 0), { start: 1, end: 2, top: 32 });
  assert.deepEqual(rowOffsets(["regular", "error", "wrapped"], heights), [0, 36, 67, 127]);
});

test("scrolling to the last file covers the viewport without exceeding the list", () => {
  const offsets = Array.from({ length: 10001 }, (_, i) => i * 36);
  const { start, end, top } = tableWindow(offsets, 999999, 600);
  assert.equal(end, 10000);
  assert.equal(top, 360000 - 600);
  assert.ok(offsets[start] <= top);
  assert.ok(end - start <= 26);
});

test("scrolling during rename retains only the editor and viewport rows", () => {
  assert.deepEqual(windowIndices(100, 103, 2), [2, 100, 101, 102]);
  assert.deepEqual(windowIndices(0, 3, 9999), [0, 1, 2, 9999]);
  assert.deepEqual(windowIndices(0, 3, 1), [0, 1, 2]);
  assert.deepEqual(windowIndices(0, 0, -1), []);
});

test("keyboard navigation reveals a wrapped row after its height is measured", () => {
  assert.equal(nearestRowScroll(600, 636, 0, 600), 36);
  assert.equal(nearestRowScroll(600, 680, 36, 600), 80);
  assert.equal(nearestRowScroll(36, 72, 80, 600), 36);
  assert.equal(nearestRowScroll(100, 136, 36, 600), 36);
});

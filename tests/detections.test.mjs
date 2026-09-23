import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { detectionLabels } = await loadTypeScript(new URL("../src/format.ts", import.meta.url));
const base = { upscaling: false, upsampling: false, transcoding: false, summary: "Clean", detail: "Lattice search not applicable at 96 kHz." };

test("an unavailable codec check does not introduce a third detection label", () => {
  assert.deepEqual(detectionLabels(base), ["Clean"]);
});

test("each positive detection stays visible regardless of the summary", () => {
  assert.deepEqual(detectionLabels({ ...base, summary: "Flagged", upscaling: true }), ["Upscaled"]);
  assert.deepEqual(detectionLabels({ ...base, upscaling: true, upsampling: true, transcoding: true }), ["Upscaled", "Upsampled", "Transcoded"]);
});

test("unmeasured placeholders display a dash instead of claiming Clean", () => {
  assert.deepEqual(detectionLabels({ ...base, summary: "Not analyzed" }), ["—"]);
});

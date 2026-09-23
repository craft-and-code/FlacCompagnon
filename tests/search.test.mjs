import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { fileSearchFields, matchesSearch } = await loadTypeScript(new URL("../src/format.ts", import.meta.url));

test("a full name cannot be assembled from unrelated fields", () => {
  assert.equal(matchesSearch(["Zakk", "Wylde"], "Zakk Wylde"), false);
  assert.equal(matchesSearch(["Zakk plays with Ozzy Wylde"], "Zakk Wylde"), false);
  assert.equal(matchesSearch(["Zakk Wylde"], "Zakk Wylde"), true);
  assert.equal(matchesSearch(["Zakk-Wylde"], "Zakk Wylde"), true);
});

test("embedded tags are searchable without joining unrelated values", () => {
  const file = {
    file_name: "Untitled.flac",
    format: "FLAC",
    ext_mismatch: false,
    declared_bits: 16,
    real_bit_depth: 16,
    sample_rate: 44100,
    duration_secs: 60,
    size_bytes: 5_000_000,
    channels: 2,
    fake_stereo: false,
    clipping: { clipped: false, true_peak_dbtp: -1 },
    dr_db: null,
    detections: { upscaling: false, upsampling: false, transcoding: false, detail: "", summary: "Clean" },
    badge: null,
    flac_md5: null,
    file_md5: null,
    file_crc32: null,
    error: null,
  };
  const splitTags = { artist: "Zakk", composer: "Wylde", extra: [] };
  assert.equal(matchesSearch(fileSearchFields(file, splitTags), "Zakk Wylde"), false);
  assert.equal(
    matchesSearch(fileSearchFields(file, { ...splitTags, artist: "Zakk Wylde" }), "Zakk Wylde"),
    true,
  );
});

test("partial words, closed numbers, and accented names remain searchable", () => {
  assert.equal(matchesSearch(["16-bit"], "16-"), true);
  assert.equal(matchesSearch(["160 MB"], "16-"), false);
  assert.equal(matchesSearch(["Beyoncé"], "beyonce"), true);
  assert.equal(matchesSearch(["Zakk Wylde"], "zakk wy"), true);
});

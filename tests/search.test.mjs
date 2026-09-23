import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { fileSearchFields, matchesSearch } = await loadTypeScript(new URL("../src/format.ts", import.meta.url));

const sampleFile = {
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

test("a full name cannot be assembled from unrelated fields", () => {
  assert.equal(matchesSearch(["Zakk", "Wylde"], "Zakk Wylde"), false);
  assert.equal(matchesSearch(["Zakk plays with Ozzy Wylde"], "Zakk Wylde"), false);
  assert.equal(matchesSearch(["Zakk Wylde"], "Zakk Wylde"), true);
  assert.equal(matchesSearch(["Zakk-Wylde"], "Zakk Wylde"), true);
});

test("embedded tags are searchable without joining unrelated values", () => {
  const splitTags = { artist: "Zakk", composer: "Wylde", extra: [] };
  assert.equal(matchesSearch(fileSearchFields(sampleFile, splitTags), "Zakk Wylde"), false);
  assert.equal(
    matchesSearch(fileSearchFields(sampleFile, { ...splitTags, artist: "Zakk Wylde" }), "Zakk Wylde"),
    true,
  );
});

test("integrated loudness is searchable, while older reports remain usable", () => {
  assert.equal(matchesSearch(fileSearchFields({ ...sampleFile, integrated_lufs: -23.0 }), "-23.0 lufs"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "lufs"), false);
});

test("balance is searchable by direction, amount and silent channel", () => {
  const fields = fileSearchFields({ ...sampleFile, stereo_balance: { state: "Measured", right_minus_left_db: -6.0206 } });
  assert.equal(matchesSearch(fields, "L +6.0 dB"), true);
  assert.equal(matchesSearch(fields, "balance"), true);
  assert.equal(matchesSearch(fileSearchFields({ ...sampleFile, stereo_balance: { state: "RightSilent" } }), "R silent"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "balance"), false);
});

test("suspected discontinuities are searchable without flagging old or clear reports", () => {
  const fields = fileSearchFields({ ...sampleFile, discontinuities: {
    clicks: { count: 1, events: [] }, dropouts: { count: 2, events: [] },
  } });
  assert.equal(matchesSearch(fields, "impulse"), true);
  assert.equal(matchesSearch(fields, "dropout"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "impulse"), false);
  assert.equal(matchesSearch(fileSearchFields({ ...sampleFile, discontinuities: {
    clicks: { count: 0, events: [] }, dropouts: { count: 0, events: [] },
  } }), "impulse"), false);
});

test("loudness range is searchable by its LU value", () => {
  const fields = fileSearchFields({ ...sampleFile, loudness_range_lu: 10.0 });
  assert.equal(matchesSearch(fields, "10.0 lu"), true);
  assert.equal(matchesSearch(fields, "lra"), true);
});

test("partial words, closed numbers, and accented names remain searchable", () => {
  assert.equal(matchesSearch(["16-bit"], "16-"), true);
  assert.equal(matchesSearch(["160 MB"], "16-"), false);
  assert.equal(matchesSearch(["Beyoncé"], "beyonce"), true);
  assert.equal(matchesSearch(["Zakk Wylde"], "zakk wy"), true);
});

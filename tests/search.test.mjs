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

test("momentary and short-term loudness maxima are searchable independently", () => {
  const fields = fileSearchFields({ ...sampleFile, loudness_peaks: {
    momentary: { lufs: -9.4, start_secs: 2 }, short_term: { lufs: -17, start_secs: 1 },
  } });
  assert.equal(matchesSearch(fields, "-9.4 LUFS-M max"), true);
  assert.equal(matchesSearch(fields, "-17.0 LUFS-S max"), true);
  assert.equal(matchesSearch(fields, "momentary"), true);
  assert.equal(matchesSearch(fields, "short-term"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "LUFS-M"), false);
});

test("DC offset is searchable in displayed percent while old reports stay absent", () => {
  const fields = fileSearchFields({ ...sampleFile, dc_offset: { channel_means: [0.001, -0.002], max_abs: 0.002 } });
  assert.equal(matchesSearch(fields, "0.200%"), true);
  assert.equal(matchesSearch(fields, "dc offset"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "dc offset"), false);
  assert.equal(matchesSearch(fileSearchFields({ ...sampleFile, dc_offset: { channel_means: [0], max_abs: 0 } }), "0.000%"), true);
});

test("local and band phase search follows the numeric columns and omits unavailable readings", () => {
  const fields = fileSearchFields({ ...sampleFile, local_phase: {
    broadband: { minimum_correlation: 0.6 }, bands: [{ summary: { minimum_correlation: -1 } }, { summary: null }],
  } });
  assert.equal(matchesSearch(fields, "0.600 local phase"), true);
  assert.equal(matchesSearch(fields, "-1.000 band phase"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "local phase"), false);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "band phase"), false);
});

test("balance is searchable by direction, amount and silent channel", () => {
  const fields = fileSearchFields({ ...sampleFile, stereo_balance: { state: "Measured", right_minus_left_db: -6.0206 } });
  assert.equal(matchesSearch(fields, "L +6.0 dB"), true);
  assert.equal(matchesSearch(fields, "balance"), true);
  assert.equal(matchesSearch(fileSearchFields({ ...sampleFile, stereo_balance: { state: "RightSilent" } }), "R silent"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "balance"), false);
});

test("high-frequency stereo readings are searchable without matching old reports", () => {
  const fields = fileSearchFields({ ...sampleFile, high_frequency_stereo: {
    side_to_mid_db: -31.3, reference_side_to_mid_db: -4, narrowed_block_fraction: 0.8, narrowed: true,
  } });
  assert.equal(matchesSearch(fields, "-31.3 dB"), true);
  assert.equal(matchesSearch(fields, "hf stereo"), true);
  assert.equal(matchesSearch(fields, "intensity"), true);
  assert.equal(matchesSearch(fileSearchFields(sampleFile), "hf stereo"), false);
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

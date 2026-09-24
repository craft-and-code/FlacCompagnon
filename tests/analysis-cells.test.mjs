import assert from "node:assert/strict";
import test from "node:test";
import { build } from "esbuild";

// Exercise the actual table definition, rendering and sorting together.
const { outputFiles } = await build({
  stdin: {
    contents: `export { ALL_COLUMNS } from "./src/components/resultColumns";
      export { sortFiles } from "./src/components/tableSort";
      export { renderToStaticMarkup } from "react-dom/server";`,
    resolveDir: process.cwd(),
  },
  bundle: true, write: false, platform: "node", format: "esm", jsx: "automatic",
  loader: { ".css": "empty" },
  // React's server renderer uses node built-ins via require.
  banner: { js: 'import { createRequire } from "node:module"; const require = createRequire(process.cwd() + "/package.json");' },
});
const { ALL_COLUMNS, sortFiles, renderToStaticMarkup } = await import(
  `data:text/javascript;base64,${Buffer.from(outputFiles[0].text).toString("base64")}`
);
const render = (key, file) => renderToStaticMarkup(ALL_COLUMNS.find((c) => c.key === key).render(file, null));

test("local and band phase expose distinct signed minima, locations and coverage", () => {
  const summary = (minimum) => ({ correlation: 0.4, minimum_correlation: minimum, minimum_start_secs: 2.5, opposed_fraction: 0.25, eligible_windows: 12 });
  const phase = (local, band) => ({ window_secs: 16384 / 48000, hop_secs: 8192 / 48000, analyzed_windows: 15,
    broadband: summary(local), bands: [
      { low_hz: 20, high_hz: 200, summary: null },
      { low_hz: 6000, high_hz: 20000, summary: summary(band) },
    ],
  });
  const files = [
    { file_name: "old", sample_rate: 48000 },
    { file_name: "local opposition", sample_rate: 48000, local_phase: phase(-0.9, -0.2) },
    { file_name: "band opposition", sample_rate: 48000, local_phase: phase(0.6, -1) },
    { file_name: "zero", sample_rate: 48000, local_phase: phase(0, 0) },
  ];
  const names = (column, direction) => sortFiles(files, { column, direction }).map((f) => f.file_name);
  assert.deepEqual(names("localPhase", "asc"), ["local opposition", "zero", "band opposition", "old"]);
  assert.deepEqual(names("bandPhase", "asc"), ["band opposition", "local opposition", "zero", "old"]);
  assert.deepEqual(names("bandPhase", "desc"), ["zero", "local opposition", "band opposition", "old"]);
  assert.match(render("localPhase", files[1]), />-0\.900</);
  assert.match(render("localPhase", files[1]), /at 2\.500 s/);
  assert.match(render("localPhase", files[1]), /25\.0% of 12 eligible windows/);
  assert.match(render("localPhase", files[1]), /341\.3 ms/);
  assert.match(render("bandPhase", files[2]), />-1\.000</);
  assert.match(render("bandPhase", files[2]), /6000–20000 Hz/);
  assert.match(render("bandPhase", files[2]), /20–200 Hz: unavailable/);
  assert.match(render("bandPhase", files[3]), />0\.000</);
  for (const key of ["localPhase", "bandPhase"]) {
    assert.match(render(key, files[0]), />—</);
    assert.match(render(key, { local_phase: phase(NaN, NaN) }), />—</);
  }
  assert.doesNotMatch(render("localPhase", { sample_rate: 48000, local_phase: phase(-1e-10, 0) }), /-0\.000/);
});

test("DC means retain channel signs while the column sorts absolute magnitudes", () => {
  const files = [
    { file_name: "old" },
    { file_name: "small", dc_offset: { channel_means: [0.001, -0.002], max_abs: 0.002 } },
    { file_name: "large", dc_offset: { channel_means: [-0.1], max_abs: 0.1 } },
    { file_name: "zero", dc_offset: { channel_means: [0, 0], max_abs: 0 } },
  ];
  const names = (direction) => sortFiles(files, { column: "dcOffset", direction }).map((f) => f.file_name);
  assert.deepEqual(names("asc"), ["zero", "small", "large", "old"]);
  assert.deepEqual(names("desc"), ["large", "small", "zero", "old"]);
  assert.equal(ALL_COLUMNS.find((c) => c.key === "dcOffset").label, "DC (%)");
  assert.match(render("dcOffset", files[1]), />0\.200</);
  assert.match(render("dcOffset", files[1]), /Ch 1: \+0\.100%/);
  assert.match(render("dcOffset", files[1]), /Ch 2: -0\.200%/);
  assert.match(render("dcOffset", files[3]), />0\.000</);
  assert.match(render("dcOffset", files[0]), />—</);
  assert.doesNotMatch(render("dcOffset", { dc_offset: { channel_means: [-1e-10], max_abs: 1e-10 } }), /-0\.000/);
});

test("balance sorts from left to right, with absent results last in both directions", () => {
  const files = [
    { file_name: "old" },
    { file_name: "right", stereo_balance: { state: "Measured", right_minus_left_db: 6 } },
    { file_name: "left only", stereo_balance: { state: "RightSilent" } },
    { file_name: "left", stereo_balance: { state: "Measured", right_minus_left_db: -6 } },
    { file_name: "right only", stereo_balance: { state: "LeftSilent" } },
    { file_name: "equal", stereo_balance: { state: "Measured", right_minus_left_db: 0 } },
  ];
  const names = (direction) => sortFiles(files, { column: "balance", direction }).map((f) => f.file_name);
  assert.deepEqual(names("asc"), ["left only", "left", "equal", "right", "right only", "old"]);
  assert.deepEqual(names("desc"), ["right only", "right", "equal", "left", "left only", "old"]);
  assert.match(render("balance", files[2]), />R silent</);
  assert.match(render("balance", files[0]), />—</);
});

test("HF stereo shows the high-band ratio and sorts missing readings last", () => {
  const files = [
    { file_name: "old" },
    { file_name: "wide", high_frequency_stereo: { side_to_mid_db: -8, reference_side_to_mid_db: -4, narrowed_block_fraction: 0, narrowed: false } },
    { file_name: "narrow", high_frequency_stereo: { side_to_mid_db: -31.3, reference_side_to_mid_db: -4, narrowed_block_fraction: 0.8, narrowed: true } },
  ];
  const names = (direction) => sortFiles(files, { column: "hfStereo", direction }).map((f) => f.file_name);
  assert.deepEqual(names("asc"), ["narrow", "wide", "old"]);
  assert.deepEqual(names("desc"), ["wide", "narrow", "old"]);
  assert.match(render("hfStereo", files[2]), />-31\.3 dB</);
  assert.match(render("hfStereo", files[2]), /intensity stereo/);
  assert.match(render("hfStereo", files[2]), /6–20 kHz/);
  assert.match(render("hfStereo", files[2]), /Experimental/);
  assert.match(render("hfStereo", { high_frequency_stereo: {
    side_to_mid_db: -120, reference_side_to_mid_db: -120, narrowed_block_fraction: 0, narrowed: false,
  } }), /reporting floor/);
  assert.match(render("hfStereo", files[0]), />—</);
});

test("short LRA readings are visibly approximate and explained", () => {
  assert.match(render("lra", { loudness_range_lu: 10, duration_secs: 40 }), /≈10\.0 LU/);
  assert.match(render("lra", { loudness_range_lu: 10, duration_secs: 40 }), /Under 60 s/);
  assert.doesNotMatch(render("lra", { loudness_range_lu: 10, duration_secs: 60 }), /≈|Under 60 s/);
  assert.match(render("lra", { duration_secs: 60 }), />—</);
  assert.match(render("loudness", { integrated_lufs: -23, duration_secs: 5 }), />-23\.0</);
  assert.doesNotMatch(render("loudness", { integrated_lufs: -23, duration_secs: 5 }), />-23\.0 LUFS</);
  assert.doesNotMatch(render("loudness", { integrated_lufs: -23, duration_secs: 5 }), /≈/);
});

test("suspected discontinuities expose channel/time and distinguish zero from unavailable", () => {
  const summary = { count: 2, events: [{ channel: 2, start_secs: 1.25, duration_secs: 0.02 }] };
  for (const kind of ["clicks", "dropouts"]) {
    const measured = { discontinuities: { [kind]: summary } };
    assert.match(render(kind, measured), />2</);
    assert.match(render(kind, measured), /Ch 2 · 1\.250 s · 20\.000 ms/);
    assert.match(render(kind, measured), /First 1 locations shown/);
    assert.match(render(kind, measured), /Counts are per channel/);
    assert.match(render(kind, { discontinuities: { [kind]: { count: 0, events: [] } } }), />0</);
    assert.match(render(kind, { discontinuities: { [kind]: { count: 1, events: [] } } }), />1</);
    assert.match(render(kind, {}), />—</);
  }
});

test("discontinuity counts sort numerically and leave missing reports last", () => {
  for (const column of ["clicks", "dropouts"]) {
    const files = [
      { file_name: "old" },
      { file_name: "ten", discontinuities: { [column]: { count: 10, events: [] } } },
      { file_name: "two", discontinuities: { [column]: { count: 2, events: [] } } },
      { file_name: "zero", discontinuities: { [column]: { count: 0, events: [] } } },
    ];
    const names = (direction) => sortFiles(files, { column, direction }).map((f) => f.file_name);
    assert.deepEqual(names("asc"), ["zero", "two", "ten", "old"]);
    assert.deepEqual(names("desc"), ["ten", "two", "zero", "old"]);
  }
});

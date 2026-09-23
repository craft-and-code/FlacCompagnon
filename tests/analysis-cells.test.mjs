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

test("short LRA readings are visibly approximate and explained", () => {
  assert.match(render("lra", { loudness_range_lu: 10, duration_secs: 40 }), /≈10\.0 LU/);
  assert.match(render("lra", { loudness_range_lu: 10, duration_secs: 40 }), /Under 60 s/);
  assert.doesNotMatch(render("lra", { loudness_range_lu: 10, duration_secs: 60 }), /≈|Under 60 s/);
  assert.match(render("lra", { duration_secs: 60 }), />—</);
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

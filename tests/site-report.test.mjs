import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { createReportImport, MAX_REPORT_BYTES } from "../site/report-import.js";
import { reportPage } from "../scripts/site/report-page.mjs";
import { reportTranslate } from "../site/report-copy.js";

// The parser's only browser initialization is Solid's event registration; no DOM rendering is simulated.
globalThis.window = { document: { addEventListener() {} } };
const { parseAnalysisDocument } =
  await import("../site/vendor/phemios-analysis/flac-analysis-report.js");
delete globalThis.window;
const track = (name = "Track.flac") => ({
  file_name: name,
  format: "FLAC",
  flac_md5: { state: "Match" },
});
const envelope = (files) =>
  JSON.stringify({ format: "flaccompagnon-report", version: 1, report: { files } });
const file = (text, extra = {}) => ({
  name: "report.json",
  size: text.length,
  text: async () => text,
  ...extra,
});
function fixture() {
  let shown = null;
  const states = [];
  const importer = createReportImport({
    show: async (text, current) => {
      const parsed = parseAnalysisDocument(text);
      if (current()) shown = parsed;
    },
    clear: () => {
      shown = null;
    },
    state: (next) => states.push(next),
  });
  return {
    importer,
    states,
    get shown() {
      return shown;
    },
  };
}

test("website import preserves album order and accepts legacy and single-track reports", async () => {
  const f = fixture();
  await f.importer.open([file(envelope([track("B.flac"), track("A.flac")]))]);
  assert.deepEqual(
    f.shown.map((row) => row.source_data.file_name),
    ["B.flac", "A.flac"],
  );
  await f.importer.open([file(JSON.stringify({ files: [track()] }))]);
  assert.equal(f.shown.length, 1);
  await f.importer.open([file(JSON.stringify(track()))]);
  assert.equal(f.shown[0].source_data.flac_md5.state, "Match");
});

test("invalid JSON, unsupported versions and malformed evidence preserve the displayed report", async () => {
  const f = fixture();
  await f.importer.open([file(envelope([track()]))]);
  const previous = f.shown;
  for (const text of [
    "not JSON",
    "{}",
    envelope([]),
    envelope([{}, track()]),
    envelope([{ ...track(), sample_rate: -1 }]),
    envelope([track()]).replace('"version":1', '"version":2'),
  ]) {
    await f.importer.open([file(text)]);
    assert.equal(f.shown, previous);
    assert.ok(f.states.at(-1).error);
  }
});

test("oversized or multiple dropped files are rejected before reading", async () => {
  const f = fixture();
  const unreadable = file("", {
    size: MAX_REPORT_BYTES + 1,
    text: () => {
      throw new Error("must not read");
    },
  });
  await f.importer.open([unreadable]);
  assert.match(f.states.at(-1).error, /16 MiB/);
  await f.importer.open([unreadable, unreadable]);
  assert.match(f.states.at(-1).error, /one JSON/);
  await f.importer.open([file("{}", { name: "audio.flac" })]);
  assert.match(f.states.at(-1).error, /JSON report/);
  assert.throws(() => parseAnalysisDocument(envelope(Array.from({ length: 2001 }, () => track()))));
});

test("clear and newer drops prevent late reads from restoring an earlier report", async () => {
  const f = fixture();
  let finish;
  const delayed = file("", {
    text: () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  });
  const first = f.importer.open([delayed]);
  f.importer.reset();
  finish(envelope([track("old.flac")]));
  await first;
  assert.equal(f.shown, null);
  const second = f.importer.open([delayed]);
  await f.importer.open([file(envelope([track("new.flac")]))]);
  finish(envelope([track("old.flac")]));
  await second;
  assert.equal(f.shown[0].source_data.file_name, "new.flac");
  await f.importer.open([]);
  assert.equal(f.shown[0].source_data.file_name, "new.flac");
});

test("untrusted filenames and diagnostics remain literal report data", () => {
  const value = '<img src="https://example.invalid/leak" onerror="alert(1)">';
  const rows = parseAnalysisDocument(
    envelope([{ ...track(value), detections: { detail: value } }]),
  );
  assert.equal(rows[0].source_data.file_name, value);
  assert.equal(rows[0].source_data.detections.detail, value);
});

test("both reader pages deny network connections and expose local-only import", () => {
  for (const language of ["fr", "en"]) {
    const html = reportPage(language);
    assert.match(html, /connect-src 'none'/);
    assert.match(html, /form-action 'none'/);
    assert.match(html, /<flac-report-viewer>/);
    assert.doesNotMatch(html, /src="https?:/);
    assert.match(html, new RegExp('<html lang="' + language + '"'));
  }
  assert.match(reportTranslate("en")("Le fichier JSON est illisible."), /JSON file/);
  assert.doesNotMatch(
    reportTranslate("en")(
      "Conforme dans le rapport ; le fichier actuel n’a pas été vérifié par Phémios.",
    ),
    /Phémios/,
  );
});

test("the vendored renderer matches its immutable provenance", async () => {
  const bundle = await readFile(
    new URL("../site/vendor/phemios-analysis/flac-analysis-report.js", import.meta.url),
  );
  const provenance = JSON.parse(
    await readFile(new URL("../site/vendor/phemios-analysis/provenance.json", import.meta.url)),
  );
  assert.equal(createHash("sha256").update(bundle).digest("hex"), provenance.sha256);
  assert.match(provenance.revision, /^[a-f0-9]{40}$/);
  assert.doesNotMatch(bundle.toString(), /\b(?:fetch|XMLHttpRequest|WebSocket|sendBeacon)\s*\(/);
});

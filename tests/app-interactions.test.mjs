import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { dispatchMenuAction } = await loadTypeScript(
  new URL("../src/components/menuActions.ts", import.meta.url),
);
const { dispatchGlobalShortcut } = await loadTypeScript(
  new URL("../src/components/globalShortcut.ts", import.meta.url),
);

function menuFixture() {
  const calls = [];
  const actions = {
    exportM3u: () => calls.push("simple"),
    exportM3uExtended: () => calls.push("extended"),
    exportCsv: () => calls.push("csv"),
    exportJson: () => calls.push("json"),
    reset: () => calls.push("reset"),
    setSpectrogramSize: (size) => calls.push(size),
  };
  return { calls, actions };
}

const nativeActions = [
  "export_m3u",
  "export_m3u_extended",
  "export_csv",
  "export_json",
  "reset",
  "spectrogram_size_small",
  "spectrogram_size_full",
];

test("a native Reset cannot clear the listing while the toolbar is unavailable", () => {
  let files = ["existing.flac"];
  const { actions } = menuFixture();
  actions.reset = () => {
    files = [];
  };
  dispatchMenuAction("reset", actions, false);
  assert.deepEqual(files, ["existing.flac"]);
  dispatchMenuAction("reset", actions, true);
  assert.deepEqual(files, []);
});

test("conversion blocks every routed native action until the app becomes available", () => {
  const { calls, actions } = menuFixture();
  for (const action of nativeActions) dispatchMenuAction(action, actions, false);
  assert.deepEqual(calls, []);
  for (const action of nativeActions) dispatchMenuAction(action, actions, true);
  assert.deepEqual(calls, ["simple", "extended", "csv", "json", "reset", "half", "full"]);
  dispatchMenuAction("unknown", actions, true);
  assert.equal(calls.length, nativeActions.length);
});

test("conversion blocks delete, selection and navigation shortcuts even without a modal", () => {
  const handled = [];
  const options = { enabled: false };
  const keys = ["Delete", "Backspace", "ArrowDown", "ArrowUp", "a"];
  for (const key of keys) {
    dispatchGlobalShortcut(
      { key, target: { tagName: "DIV", isContentEditable: false } },
      (event) => handled.push(event.key),
      options,
      () => false,
    );
  }
  assert.deepEqual(handled, []);
  options.enabled = true;
  for (const key of keys) {
    dispatchGlobalShortcut(
      { key, target: { tagName: "DIV", isContentEditable: false } },
      (event) => handled.push(event.key),
      options,
      () => false,
    );
  }
  assert.deepEqual(handled, keys);
});

test("enabling page shortcuts preserves text-field and modal keyboard ownership", () => {
  const handled = [];
  const onKey = (event) => handled.push(event.key);
  for (const target of [
    { tagName: "INPUT" },
    { tagName: "TEXTAREA" },
    { tagName: "DIV", isContentEditable: true },
  ]) {
    dispatchGlobalShortcut({ key: "Backspace", target }, onKey, { enabled: true }, () => false);
  }
  const event = { key: "Delete", target: { tagName: "DIV" } };
  dispatchGlobalShortcut(event, onKey, { enabled: true }, () => true);
  assert.deepEqual(handled, []);
  dispatchGlobalShortcut(event, onKey, { enabled: true, insideModal: true }, () => true);
  assert.deepEqual(handled, ["Delete"]);
  dispatchGlobalShortcut(event, onKey, { enabled: false, insideModal: true }, () => true);
  assert.deepEqual(handled, ["Delete"]);
});

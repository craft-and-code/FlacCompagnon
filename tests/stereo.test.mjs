import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { stereoLabel, stereoStatus, stereoBalanceLabel } = await loadTypeScript(new URL("../src/format.ts", import.meta.url));
const stereo = { channels: 2, fake_stereo: false };

test("balance identifies the louder channel without calling equal energy dual-mono", () => {
  const measured = (db) => ({ state: "Measured", right_minus_left_db: db });
  assert.equal(stereoBalanceLabel(measured(-6.0206)), "L +6.0 dB");
  assert.equal(stereoBalanceLabel(measured(6.0206)), "R +6.0 dB");
  assert.equal(stereoBalanceLabel(measured(-0.001)), "0.0 dB");
  assert.equal(stereoBalanceLabel({ state: "LeftSilent" }), "L silent");
  assert.equal(stereoBalanceLabel({ state: "RightSilent" }), "R silent");
  assert.equal(stereoBalanceLabel(null), "—");
  assert.equal(stereoBalanceLabel(undefined), "—");
  assert.equal(stereoBalanceLabel(measured(NaN)), "—");
});

test("opposite channels are identified as a likely polarity problem", () => {
  const file = { ...stereo, phase_correlation: -1, phase_inverted: true };
  assert.equal(stereoStatus(file), "polarity");
  assert.equal(stereoLabel(file), "polarity?");
});

test("negative correlation without severe cancellation is a phase risk", () => {
  const file = { ...stereo, phase_correlation: -0.5, phase_inverted: false };
  assert.equal(stereoStatus(file), "phase-risk");
  assert.equal(stereoLabel(file), "phase risk");
});

test("near-zero correlation and older reports remain ordinary stereo", () => {
  assert.equal(stereoStatus({ ...stereo, phase_correlation: -0.05 }), "stereo");
  assert.equal(stereoStatus(stereo), "stereo");
});

test("mono and dual-mono keep their existing classifications", () => {
  assert.equal(stereoStatus({ channels: 1, fake_stereo: null }), "mono");
  assert.equal(stereoStatus({ ...stereo, fake_stereo: true }), "dual-mono");
});

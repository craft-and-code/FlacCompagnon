import assert from "node:assert/strict";
import test from "node:test";
import { checkTauriVersions } from "../scripts/check-tauri-versions.mjs";

function fixtures({
  tauri = "2.12.1",
  api = "2.12.1",
  dialogRust = "2.7.1",
  dialogJs = "2.7.0",
} = {}) {
  return [
    `version = 4\n\n[[package]]\nname = "tauri"\nversion = "${tauri}"\n\n[[package]]\nname = "tauri-plugin-dialog"\nversion = "${dialogRust}"\n`,
    JSON.stringify({
      lockfileVersion: 3,
      packages: {
        "": { version: "0.9.6" },
        "node_modules/@tauri-apps/api": { version: api },
        "node_modules/@tauri-apps/plugin-dialog": { version: dialogJs },
        "node_modules/@tauri-apps/cli": { version: "2.11.1" },
      },
    }),
  ];
}

test("the 0.9.6 release's API minor-version mismatch fails before bundling", () => {
  assert.throws(
    () => checkTauriVersions(...fixtures({ api: "2.11.1" })),
    /tauri 2\.12\.1 is incompatible with @tauri-apps\/api 2\.11\.1.*major\/minor[\s\S]*regenerate/,
  );
});

test("compatible bridge patches pass without constraining plugin or CLI release cycles", () => {
  assert.deepEqual(checkTauriVersions(...fixtures({ api: "2.12.0", dialogJs: "2.6.0" })), {
    rust: "2.12.1",
    javascript: "2.12.0",
  });
});

test("major and minor mismatches in either direction fail", () => {
  for (const override of [{ api: "3.12.1" }, { api: "2.11.1" }, { tauri: "2.11.1" }]) {
    assert.throws(
      () => checkTauriVersions(...fixtures(override)),
      /major\/minor versions must match/,
    );
  }
});

test("missing or ambiguous resolved Rust packages cannot silently pass", () => {
  const [cargo, npm] = fixtures();
  assert.throws(
    () => checkTauriVersions(cargo.replace('name = "tauri"', 'name = "other"'), npm),
    /exactly one resolved tauri package/,
  );
  assert.throws(
    () => checkTauriVersions(`${cargo}\n[[package]]\nname = "tauri"\nversion = "2.11.1"\n`, npm),
    /exactly one resolved tauri package/,
  );
  assert.throws(
    () => checkTauriVersions(cargo.replace('version = "2.12.1"', "version = 2"), npm),
    /missing or malformed version for tauri/,
  );
});

test("missing and invalid JavaScript versions are rejected", () => {
  const [cargo, npm] = fixtures();
  const lock = JSON.parse(npm);
  delete lock.packages["node_modules/@tauri-apps/api"];
  assert.throws(
    () => checkTauriVersions(cargo, JSON.stringify(lock)),
    /invalid resolved version for @tauri-apps\/api/,
  );
  for (const api of [
    null,
    2,
    "2.12",
    "v2.12.1",
    "2.12.1 garbage",
    "02.12.1",
    "2.12.1-alpha_1",
    "2.12.1-01",
    "2.12.1+build..1",
  ]) {
    assert.throws(() => checkTauriVersions(...fixtures({ api })), /invalid resolved version/);
  }
  assert.throws(
    () => checkTauriVersions(...fixtures({ tauri: "2.12" })),
    /invalid resolved version for tauri/,
  );
});

test("malformed lockfiles fail with an actionable error", () => {
  const [cargo, npm] = fixtures();
  assert.throws(() => checkTauriVersions(null, npm), /Cargo.lock must be readable text/);
  for (const invalid of [
    "{",
    "null",
    "{}",
    '{"packages":[]}',
    '{"lockfileVersion":1,"packages":{}}',
  ]) {
    assert.throws(() => checkTauriVersions(cargo, invalid), /package-lock.json must contain/);
  }
});

test("valid SemVer prerelease and build metadata retain the major/minor series", () => {
  assert.deepEqual(
    checkTauriVersions(...fixtures({ tauri: "2.12.1-alpha.1", api: "2.12.0+build.001" })),
    {
      rust: "2.12.1-alpha.1",
      javascript: "2.12.0+build.001",
    },
  );
});

test("Cargo.lock CRLF and LF files describe the same resolved versions", () => {
  const [cargo, npm] = fixtures();
  assert.deepEqual(
    checkTauriVersions(cargo.replaceAll("\n", "\r\n"), npm),
    checkTauriVersions(cargo, npm),
  );
});

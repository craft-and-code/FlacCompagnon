import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("native dev/build hooks and the CLI prepare stable audio before compilation", () => {
  const config = JSON.parse(
    readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
  );
  for (const hook of [config.build.beforeDevCommand, config.build.beforeBuildCommand])
    assert.match(hook, /^node scripts\/audio-source\.mjs && npm run /);
  const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  assert.match(pkg.scripts["build:cli"], /^node scripts\/audio-source\.mjs && cargo build/);
});

test("each CI/release workflow freezes one stable selection before its Cargo jobs", () => {
  for (const [file, jobs] of [
    ["ci.yml", ["windows-check", "build-and-test"]],
    ["release.yml", ["release", "cli"]],
    ["docs.yml", ["docs"]],
  ]) {
    const workflow = readFileSync(new URL(`../.github/workflows/${file}`, import.meta.url), "utf8");
    assert.equal(
      (workflow.match(/run: node scripts\/audio-source\.mjs --resolve/g) ?? []).length,
      1,
      file,
    );
    for (const job of jobs) {
      const body = workflow.split(`  ${job}:\n`)[1]?.split(/\n  [a-z][a-z-]*:\n/)[0];
      assert.ok(body, `${file}: ${job}`);
      assert.match(body, /needs: audio-source/);
      assert.match(body, /AEDE_AUDIO_RELEASE: \$\{\{ needs\.audio-source\.outputs\.release \}\}/);
      assert.match(body, /run: node scripts\/audio-source\.mjs/);
    }
  }
});

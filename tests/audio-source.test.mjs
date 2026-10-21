import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  rmSync,
  existsSync,
  readdirSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  repository,
  stableRelease,
  selectedRelease,
  resolveStableRelease,
  pinnedManifest,
  prepareAudioSource,
} from "../scripts/audio-source.mjs";

const old = "a".repeat(40),
  revision = "b".repeat(40);
const release = { tag: "v0.6.2", version: "0.6.2", revision };
const metadata = {
  draft: false,
  prerelease: false,
  tag_name: release.tag,
  published_at: "2026-10-20T12:00:00Z",
};
const manifest = `[workspace.dependencies]\naede-audio = { git = "${repository}", rev = "${old}", default-features = false }\n`;
const fetcher = async () => ({ ok: true, json: async () => metadata });

async function fixture(work) {
  const root = mkdtempSync(join(tmpdir(), "stable-audio-"));
  const source = join(root, ".audio-source", `aede-${revision}`);
  mkdirSync(source, { recursive: true });
  writeFileSync(join(source, "Cargo.toml"), '[workspace.package]\nversion = "0.6.2"\n');
  for (const name of ["aede-audio", "aede-dsp"]) {
    mkdirSync(join(source, "crates", name), { recursive: true });
    writeFileSync(join(source, "crates", name, "Cargo.toml"), `[package]\nname = "${name}"\n`);
  }
  writeFileSync(join(root, "Cargo.toml"), manifest);
  writeFileSync(join(root, "Cargo.lock"), "original lock\n");
  const calls = [];
  const run = (command, args, cwd) => {
    calls.push({ command, args, cwd });
    if (command === "git" && args[0] === "rev-parse") return revision;
    if (command === "git" && args[0] === "status") return "";
    if (command === "cargo" && args[0] === "update")
      writeFileSync(join(root, "Cargo.lock"), `resolved #${revision}"\n`);
    return "";
  };
  try {
    await work({ root, source, calls, run });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test("only published stable release metadata can select a source", () => {
  assert.deepEqual(stableRelease(metadata), { tag: release.tag, version: release.version });
  for (const patch of [
    { draft: true },
    { prerelease: true },
    { published_at: null },
    { tag_name: "main" },
    { tag_name: "v0.6.2-rc.1" },
    { tag_name: "v00.6.2" },
  ])
    assert.throws(() => stableRelease({ ...metadata, ...patch }));
  for (const patch of [
    { revision: "main" },
    { tag: "../../elsewhere" },
    { version: "0.6.1" },
    { revision: `${revision}\n` },
  ])
    assert.throws(() => selectedRelease({ ...release, ...patch }));
});

test("annotated release tags resolve to the exact commit, not the tag object", async () => {
  const chosen = await resolveStableRelease({
    fetcher,
    run: (_, args) => {
      assert.deepEqual(args.slice(-2), [`refs/tags/${release.tag}`, `refs/tags/${release.tag}^{}`]);
      return `${old}\trefs/tags/${release.tag}\n${revision}\trefs/tags/${release.tag}^{}\n`;
    },
  });
  assert.deepEqual(chosen, release);
});

test("unavailable or ambiguous releases fail instead of selecting an old pin", async () => {
  await assert.rejects(
    resolveStableRelease({
      fetcher: async () => ({ ok: false, status: 503 }),
      run: () => assert.fail("Git must not run"),
    }),
    /HTTP 503/,
  );
  await assert.rejects(
    resolveStableRelease({
      fetcher: async () => {
        throw new Error("network unavailable");
      },
    }),
    /network unavailable/,
  );
  for (const answer of [
    "",
    `${revision}\trefs/heads/master`,
    `${revision}\trefs/tags/${release.tag}\n${old}\trefs/tags/${release.tag}`,
  ])
    await assert.rejects(resolveStableRelease({ fetcher, run: () => answer }));
});

test("updating a build changes only the shared revision in the manifest", () => {
  const changed = pinnedManifest(manifest, revision);
  assert.equal(changed, manifest.replace(old, revision));
  for (const changed of [
    manifest.replace("rev =", "branch ="),
    manifest.replace(repository, "file:///elsewhere"),
    manifest + manifest,
  ])
    assert.throws(() => pinnedManifest(changed, revision));
});

test("a selected stable source prepares both the client and its identical embedded engine", async () =>
  fixture(async ({ root, source, calls, run }) => {
    const selected = await prepareAudioSource({ root, release, engine: true, run, log: () => {} });
    assert.equal(selected.source, source);
    assert.equal(readFileSync(join(root, "Cargo.toml"), "utf8"), manifest.replace(old, revision));
    assert.deepEqual(
      calls
        .filter(({ command, args }) => command === "cargo" && args[0] === "fetch")
        .map(({ cwd }) => cwd),
      [root, source],
    );
    assert.deepEqual(
      JSON.parse(readFileSync(join(root, ".audio-source", "selection.json"), "utf8")),
      release,
    );
    assert.ok(
      calls.every(({ args }) => !args.includes("build")),
      "preparation must not compile an App",
    );
    assert.ok(!existsSync(join(root, ".audio-source", "prepare.lock")));
  }));

test("an unchanged selection retains its lock while still fetching required dependencies", async () =>
  fixture(async ({ root, calls, run }) => {
    writeFileSync(join(root, "Cargo.toml"), manifest.replace(old, revision));
    writeFileSync(join(root, "Cargo.lock"), `already locked #${revision}"\n`);
    await prepareAudioSource({ root, release, run, log: () => {} });
    assert.ok(!calls.some(({ command, args }) => command === "cargo" && args[0] === "update"));
    assert.ok(calls.some(({ command, args }) => command === "cargo" && args[0] === "fetch"));
  }));

test("dependency failure restores the prior manifest and lock, without recording success", async () =>
  fixture(async ({ root, run }) => {
    await assert.rejects(
      prepareAudioSource({
        root,
        release,
        run: (command, args, cwd) => {
          const value = run(command, args, cwd);
          if (command === "cargo" && args[0] === "fetch") throw new Error("fetch failed");
          return value;
        },
        log: () => assert.fail("success must not be logged"),
      }),
      /fetch failed/,
    );
    assert.equal(readFileSync(join(root, "Cargo.toml"), "utf8"), manifest);
    assert.equal(readFileSync(join(root, "Cargo.lock"), "utf8"), "original lock\n");
    assert.ok(!existsSync(join(root, ".audio-source", "selection.json")));
    assert.ok(!existsSync(join(root, ".audio-source", "prepare.lock")));
  }));

test("a stable release predating shared audio leaves client files untouched", async () =>
  fixture(async ({ root, source, calls, run }) => {
    rmSync(join(source, "crates"), { recursive: true });
    await assert.rejects(
      prepareAudioSource({ root, release, run, log: () => {} }),
      /Publish the next stable/,
    );
    assert.equal(readFileSync(join(root, "Cargo.toml"), "utf8"), manifest);
    assert.equal(readFileSync(join(root, "Cargo.lock"), "utf8"), "original lock\n");
    assert.ok(!calls.some(({ command }) => command === "cargo"));
  }));

test("changed cached source and moved release tags are refused before client mutation", async () =>
  fixture(async ({ root, source, run }) => {
    await assert.rejects(
      prepareAudioSource({
        root,
        release,
        run: (command, args, cwd) =>
          args[0] === "status" ? " M Cargo.toml" : run(command, args, cwd),
        log: () => {},
      }),
      /cached Aède source changed/,
    );
    rmSync(source, { recursive: true });
    await assert.rejects(
      prepareAudioSource({
        root,
        release,
        run: (command, args) => (args[0] === "rev-parse" ? old : ""),
        log: () => {},
      }),
      /tag moved/,
    );
    assert.equal(readFileSync(join(root, "Cargo.toml"), "utf8"), manifest);
    assert.deepEqual(readdirSync(join(root, ".audio-source")), []);
  }));

test("a concurrent preparation cannot replace another process's lock or change the pin", async () =>
  fixture(async ({ root, run }) => {
    const lock = join(root, ".audio-source", "prepare.lock");
    mkdirSync(lock);
    await assert.rejects(
      prepareAudioSource({ root, release, run, log: () => {} }),
      /Another audio preparation/,
    );
    assert.ok(existsSync(lock));
    assert.equal(readFileSync(join(root, "Cargo.toml"), "utf8"), manifest);
  }));

test("concurrent manifest edits survive a failed dependency fetch", async () =>
  fixture(async ({ root, run }) => {
    const edited = "maintainer's concurrent manifest edit\n";
    await assert.rejects(
      prepareAudioSource({
        root,
        release,
        log: () => {},
        run: (command, args, cwd) => {
          const result = run(command, args, cwd);
          if (command === "cargo" && args[0] === "fetch") {
            writeFileSync(join(root, "Cargo.toml"), edited);
            throw new Error("fetch interrupted");
          }
          return result;
        },
      }),
    );
    assert.equal(readFileSync(join(root, "Cargo.toml"), "utf8"), edited);
    assert.ok(!existsSync(join(root, ".audio-source", "selection.json")));
  }));

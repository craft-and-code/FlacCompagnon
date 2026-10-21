// Build-time stable-source selection. Runtime playback never performs this update.
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import {
  appendFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const repository = "https://github.com/craft-and-code/aede";
const latest = "https://api.github.com/repos/craft-and-code/aede/releases/latest";
const sha = /^[0-9a-f]{40}$/;
const tag = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

/** Execute argument arrays without a shell; subprocess output never selects another source. */
export function runCommand(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
    timeout: 600_000,
    maxBuffer: 2 * 1024 * 1024,
  });
  if (result.error || result.status !== 0)
    throw new Error(`${command} failed during stable audio preparation.`);
  return result.stdout.trim();
}

/** Only an actually published stable release may advance a build's source. */
export function stableRelease(value) {
  if (
    !value ||
    value.draft !== false ||
    value.prerelease !== false ||
    typeof value.tag_name !== "string" ||
    !tag.test(value.tag_name) ||
    typeof value.published_at !== "string" ||
    !Number.isFinite(Date.parse(value.published_at))
  )
    throw new Error("The latest Aède release must be published, stable and version-tagged.");
  return { tag: value.tag_name, version: value.tag_name.replace(/^v/, "") };
}

/** A workflow resolves once, then passes this exact selection to every platform. */
export function selectedRelease(value) {
  if (
    !value ||
    typeof value.tag !== "string" ||
    !tag.test(value.tag) ||
    value.version !== value.tag.replace(/^v/, "") ||
    typeof value.revision !== "string" ||
    !sha.test(value.revision)
  )
    throw new Error("Invalid stable audio release selection.");
  return { tag: value.tag, version: value.version, revision: value.revision };
}

/** Query the official release and resolve annotated tags to their commit, never to a branch. */
export async function resolveStableRelease({ fetcher = fetch, run = runCommand } = {}) {
  const headers = { Accept: "application/vnd.github+json", "User-Agent": "aede-audio-build" };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  const response = await fetcher(latest, { headers, signal: AbortSignal.timeout(15_000) });
  if (!response.ok)
    throw new Error(`Cannot verify the latest stable Aède release (HTTP ${response.status}).`);
  const release = stableRelease(await response.json());
  const reference = `refs/tags/${release.tag}`;
  const lines = run("git", ["ls-remote", "--", repository, reference, `${reference}^{}`])
    .split("\n")
    .filter(Boolean)
    .map((line) => line.trim().split(/\s+/));
  if (
    lines.some(([id, ref]) => !sha.test(id ?? "") || ![reference, `${reference}^{}`].includes(ref))
  )
    throw new Error("GitHub returned an invalid release tag identity.");
  const matches = (ref) => lines.filter((entry) => entry[1] === ref);
  if (matches(reference).length !== 1 || matches(`${reference}^{}`).length > 1)
    throw new Error("The published Aède release tag is missing or ambiguous.");
  const revision = (matches(`${reference}^{}`)[0] ?? matches(reference)[0])[0];
  return selectedRelease({ ...release, revision });
}

function atomicWrite(path, value) {
  const temporary = `${path}.audio-${randomUUID()}.tmp`;
  try {
    writeFileSync(temporary, value, { flag: "wx" });
    renameSync(temporary, path);
  } finally {
    rmSync(temporary, { force: true });
  }
}

function matchesFile(path, value) {
  try {
    return readFileSync(path, "utf8") === value;
  } catch {
    return false;
  }
}

function validateSource(source, release, run) {
  if (
    run("git", ["rev-parse", "HEAD"], source) !== release.revision ||
    run("git", ["status", "--porcelain", "--untracked-files=normal"], source)
  )
    throw new Error("The cached Aède source changed; remove that cache before rebuilding.");
  for (const name of ["aede-audio", "aede-dsp"])
    if (!existsSync(join(source, "crates", name, "Cargo.toml")))
      throw new Error(
        `Aède ${release.tag} does not contain the shared audio foundation. Publish the next stable Aède release first.`,
      );
  const manifest = readFileSync(join(source, "Cargo.toml"), "utf8");
  if (manifest.match(/^version\s*=\s*"([^"]+)"/m)?.[1] !== release.version)
    throw new Error("The stable Aède tag and source version disagree.");
}

function acquireSource(cache, release, run) {
  const source = join(cache, `aede-${release.revision}`);
  if (!existsSync(source)) {
    const temporary = mkdtempSync(join(cache, "checkout-"));
    try {
      run("git", ["init", "--quiet"], temporary);
      run(
        "git",
        [
          "fetch",
          "--quiet",
          "--depth=1",
          "--no-tags",
          "--",
          repository,
          `refs/tags/${release.tag}`,
        ],
        temporary,
      );
      if (run("git", ["rev-parse", "FETCH_HEAD^{commit}"], temporary) !== release.revision)
        throw new Error("The Aède release tag moved after selection; restart the build.");
      run("git", ["checkout", "--quiet", "--detach", release.revision], temporary);
      validateSource(temporary, release, run);
      renameSync(temporary, source);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
  }
  validateSource(source, release, run);
  return source;
}

/** Keep a full SHA in Cargo for the selected build; unrelated dependencies remain locked. */
export function pinnedManifest(manifest, revision) {
  if (!sha.test(revision)) throw new Error("Audio source needs a full commit SHA.");
  const declarations = [...manifest.matchAll(/^aede-audio\s*=\s*\{[^\n}]+\}/gm)];
  if (
    declarations.length !== 1 ||
    !declarations[0][0].includes(`git = "${repository}"`) ||
    /\b(?:path|branch|tag)\s*=/.test(declarations[0][0]) ||
    !/\brev\s*=\s*"[0-9a-f]{40}"/.test(declarations[0][0])
  )
    throw new Error("The workspace must declare one Git-pinned aede-audio dependency.");
  return manifest.replace(
    declarations[0][0],
    declarations[0][0].replace(/\brev\s*=\s*"[0-9a-f]{40}"/, `rev = "${revision}"`),
  );
}

/** Prepare dependencies only, never compile an App. Failure cannot silently retain an older source. */
export async function prepareAudioSource({
  root,
  release,
  engine = false,
  fetcher = fetch,
  run = runCommand,
  log = console.log,
}) {
  const chosen = release ? selectedRelease(release) : await resolveStableRelease({ fetcher, run });
  const cache = join(root, ".audio-source");
  mkdirSync(cache, { recursive: true });
  const lock = join(cache, "prepare.lock");
  try {
    mkdirSync(lock);
  } catch (error) {
    if (error.code === "EEXIST")
      throw new Error(
        "Another audio preparation is active. If it was interrupted, remove .audio-source/prepare.lock before retrying.",
      );
    throw error;
  }
  try {
    const source = acquireSource(cache, chosen, run);
    const manifestPath = join(root, "Cargo.toml"),
      lockPath = join(root, "Cargo.lock");
    const before = readFileSync(manifestPath, "utf8"),
      beforeLock = readFileSync(lockPath, "utf8");
    const updated = pinnedManifest(before, chosen.revision);
    let preparedLock = beforeLock;
    try {
      if (updated !== before || !beforeLock.includes(`#${chosen.revision}"`)) {
        atomicWrite(manifestPath, updated);
        run("cargo", ["update", "--workspace"], root);
        preparedLock = readFileSync(lockPath, "utf8");
      }
      if (!matchesFile(manifestPath, updated))
        throw new Error("Manifest changed during preparation.");
      run("cargo", ["fetch", "--locked"], root);
      if (engine) run("cargo", ["fetch", "--locked"], source);
      if (!matchesFile(manifestPath, updated) || !matchesFile(lockPath, preparedLock))
        throw new Error("Build manifests changed during preparation.");
    } catch (error) {
      if (!matchesFile(manifestPath, updated) || !matchesFile(lockPath, preparedLock))
        throw new Error(
          "Audio preparation failed while build manifests changed concurrently; edits were left untouched. Review them before retrying.",
          { cause: error },
        );
      atomicWrite(manifestPath, before);
      atomicWrite(lockPath, beforeLock);
      throw error;
    }
    atomicWrite(join(cache, "selection.json"), JSON.stringify(chosen, null, 2) + "\n");
    log(`[audio] Aède ${chosen.tag} / ${chosen.revision} (latest stable release).`);
    return { ...chosen, source };
  } finally {
    rmSync(lock, { recursive: true, force: true });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const args = process.argv.slice(2);
    if (args.length === 1 && args[0] === "--resolve") {
      const release = await resolveStableRelease();
      console.log(JSON.stringify(release));
      if (process.env.GITHUB_OUTPUT)
        appendFileSync(
          process.env.GITHUB_OUTPUT,
          `release=${JSON.stringify(release)}\nrevision=${release.revision}\n`,
        );
    } else if (args.length === 0) {
      const root = resolve(import.meta.dirname, "..");
      const release = process.env.AEDE_AUDIO_RELEASE
        ? selectedRelease(JSON.parse(process.env.AEDE_AUDIO_RELEASE))
        : undefined;
      await prepareAudioSource({
        root,
        release,
        engine: JSON.parse(readFileSync(join(root, "package.json"), "utf8")).name === "phemios",
      });
    } else throw new Error("Usage: node scripts/audio-source.mjs [--resolve]");
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

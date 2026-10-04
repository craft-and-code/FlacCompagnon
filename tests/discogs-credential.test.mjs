import assert from "node:assert/strict";
import test from "node:test";
import { build } from "esbuild";
import { loadTypeScript } from "./load-typescript.mjs";

const { DiscogsCredentialController, LEGACY_DISCOGS_TOKEN_KEY } = await loadTypeScript(
  new URL("../src/components/discogsCredential.ts", import.meta.url),
);
const { DraftRevision } = await loadTypeScript(
  new URL("../src/components/draftRevision.ts", import.meta.url),
);
const { finishCredentialDraft } = await loadTypeScript(
  new URL("../src/components/credentialDraft.ts", import.meta.url),
);
const { searchLookupProviders, lookupCandidateDetail } = await loadTypeScript(
  new URL("../src/components/lookupProviders.ts", import.meta.url),
);

function deferred() {
  let resolve, reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
const tick = () => Promise.resolve();

function fixture(legacy = null) {
  const stored = new Map(legacy == null ? [] : [[LEGACY_DISCOGS_TOKEN_KEY, legacy]]);
  const calls = [],
    snapshots = [],
    errors = [];
  let removalBlocked = false,
    readsBlocked = false,
    writes = 0;
  const storage = {
    getItem(key) {
      if (readsBlocked) throw new Error("browser storage disabled");
      return stored.get(key) ?? null;
    },
    removeItem(key) {
      if (removalBlocked) throw new Error("browser storage removal blocked");
      stored.delete(key);
    },
    setItem() {
      writes++;
      throw new Error("plaintext persistence is forbidden");
    },
  };
  const backend = {
    getDiscogsCredentialStatus: async () => {
      calls.push(["status"]);
      return { configured: false };
    },
    saveDiscogsToken: async (token) => {
      calls.push(["save", token]);
      return { configured: true };
    },
    deleteDiscogsToken: async () => {
      calls.push(["delete"]);
      return { configured: false };
    },
    migrateDiscogsToken: async (token) => {
      calls.push(["migrate", token]);
      return { configured: true };
    },
  };
  const controller = new DiscogsCredentialController(
    backend,
    () => storage,
    (state) => snapshots.push(state),
    (error) => errors.push(error),
  );
  return {
    controller,
    backend,
    stored,
    calls,
    snapshots,
    errors,
    state: () => snapshots.at(-1),
    writes: () => writes,
    blockRemoval: (blocked) => {
      removalBlocked = blocked;
    },
    blockReads: (blocked) => {
      readsBlocked = blocked;
    },
  };
}

test("startup removes the browser token before migration and StrictMode callers share one operation", async () => {
  const f = fixture(" legacy-secret ");
  const migration = deferred();
  f.backend.migrateDiscogsToken = async (token) => {
    assert.equal(f.stored.has(LEGACY_DISCOGS_TOKEN_KEY), false);
    f.calls.push(["migrate", token]);
    return migration.promise;
  };
  const first = f.controller.initialize();
  const second = f.controller.initialize();
  assert.equal(first, second);
  assert.equal(f.calls.length, 1);
  assert.deepEqual(f.calls[0], ["migrate", "legacy-secret"]);
  assert.equal(f.state().ready, false);
  migration.resolve({ configured: true });
  assert.equal(await first, true);
  assert.equal(f.state().configured, true);
  assert.equal(f.state().recoveryPending, false);
  assert.equal(f.state().legacyCleanupPending, false);
  assert.equal(f.writes(), 0);
  assert.equal(JSON.stringify(f.snapshots).includes("legacy-secret"), false);
});

test("migration respects an existing OS credential without exposing it to the frontend", async () => {
  const f = fixture("legacy-secret");
  f.backend.migrateDiscogsToken = async () => ({
    configured: true,
    ignoredBackendField: "os-secret",
  });
  await f.controller.initialize();
  assert.equal(f.stored.has(LEGACY_DISCOGS_TOKEN_KEY), false);
  assert.equal(f.state().configured, true);
  assert.equal(f.state().recoveryPending, false);
  assert.equal(JSON.stringify(f.snapshots).includes("os-secret"), false);
  assert.equal(
    f.calls.some(([operation]) => operation === "save"),
    false,
  );
});

test("a locked vault after browser removal retains a session recovery copy for retry", async () => {
  const f = fixture("legacy-secret");
  let attempts = 0;
  f.backend.migrateDiscogsToken = async (token) => {
    f.calls.push(["migrate", token]);
    if (++attempts === 1) throw new Error(`vault locked; sensitive detail ${token}`);
    return { configured: true };
  };
  assert.equal(await f.controller.initialize(), false);
  assert.equal(f.stored.has(LEGACY_DISCOGS_TOKEN_KEY), false);
  assert.equal(f.state().ready, true);
  assert.equal(f.state().recoveryPending, true);
  assert.match(f.state().error, /Unlock the store|memory/);
  assert.equal(f.state().error.includes("legacy-secret"), false);
  assert.equal(f.errors.length, 1);
  assert.equal(await f.controller.retry(), true);
  assert.deepEqual(f.calls, [
    ["migrate", "legacy-secret"],
    ["migrate", "legacy-secret"],
  ]);
  assert.equal(f.state().recoveryPending, false);
  assert.equal(f.state().error, null);
  assert.equal(f.writes(), 0);
});

test("blocked browser removal is reported even when the OS migration succeeds", async () => {
  const f = fixture("legacy-secret");
  f.blockRemoval(true);
  await f.controller.initialize();
  assert.equal(f.state().configured, true);
  assert.equal(f.state().legacyCleanupPending, true);
  assert.equal(f.stored.get(LEGACY_DISCOGS_TOKEN_KEY), "legacy-secret");
  assert.match(f.state().error, /Could not remove|plaintext copy may still remain/);
  f.blockRemoval(false);
  await f.controller.retry();
  assert.equal(f.stored.has(LEGACY_DISCOGS_TOKEN_KEY), false);
  assert.equal(f.state().legacyCleanupPending, false);
  assert.equal(f.state().error, null);
});

test("unreadable legacy storage does not claim cleanup or prevent a status check", async () => {
  const f = fixture("legacy-secret");
  f.blockReads(true);
  await f.controller.initialize();
  assert.deepEqual(f.calls, [["status"]]);
  assert.equal(f.state().legacyCleanupPending, true);
  assert.match(f.state().error, /Could not inspect.*plaintext copy may still remain/);
  assert.equal(f.stored.has(LEGACY_DISCOGS_TOKEN_KEY), true);
  f.blockReads(false);
  await f.controller.retry();
  assert.equal(f.state().configured, true);
  assert.equal(f.state().legacyCleanupPending, false);
});

test("an empty legacy value is removed without sending an empty token to the vault", async () => {
  const f = fixture("   ");
  await f.controller.initialize();
  assert.deepEqual(f.calls, [["status"]]);
  assert.equal(f.stored.has(LEGACY_DISCOGS_TOKEN_KEY), false);
  assert.equal(f.state().recoveryPending, false);
});

test("save and forget cannot overlap or let a stale save describe a newer delete", async () => {
  const f = fixture();
  await f.controller.initialize();
  const saving = deferred();
  f.backend.saveDiscogsToken = async (token) => {
    f.calls.push(["save", token]);
    return saving.promise;
  };
  const save = f.controller.save("new-secret");
  await tick();
  assert.equal(f.state().busy, true);
  assert.equal(await f.controller.forget(), false);
  assert.equal(
    f.calls.some(([operation]) => operation === "delete"),
    false,
  );
  let lookupReady = false;
  const waiting = f.controller.readyForLookup().then((ready) => {
    lookupReady = true;
    return ready;
  });
  await tick();
  assert.equal(lookupReady, false);
  saving.resolve({ configured: true });
  assert.equal(await save, true);
  assert.equal(await waiting, true);
  assert.equal(await f.controller.forget(), true);
  assert.equal(f.state().configured, false);
});

test("a failed save preserves the previous presence state and reports an explicit failure", async () => {
  const f = fixture();
  f.backend.getDiscogsCredentialStatus = async () => ({ configured: true });
  f.backend.saveDiscogsToken = async () => {
    throw new Error("locked vault");
  };
  await f.controller.initialize();
  assert.equal(await f.controller.save("replacement"), false);
  assert.equal(f.state().configured, true);
  assert.equal(f.state().busy, false);
  assert.match(f.state().error, /Could not save.*system credential storage/);
  assert.equal(f.writes(), 0);
});

test("forget discards a failed-migration recovery copy even if OS deletion is unavailable", async () => {
  const f = fixture("legacy-secret");
  f.backend.migrateDiscogsToken = async () => {
    throw new Error("no vault");
  };
  f.backend.deleteDiscogsToken = async () => {
    throw new Error("no vault");
  };
  await f.controller.initialize();
  assert.equal(f.state().recoveryPending, true);
  assert.equal(await f.controller.forget(), false);
  assert.equal(f.state().recoveryPending, false);
  assert.match(f.state().error, /Could not remove.*system credential storage/);
  f.backend.deleteDiscogsToken = async () => {
    f.calls.push(["delete"]);
    return { configured: false };
  };
  await f.controller.retry();
  assert.deepEqual(f.calls, [["delete"]]);
  assert.equal(f.state().error, null);
  assert.equal(f.state().recoveryPending, false);
  assert.equal(f.writes(), 0);
});

test("forget refuses to delete the vault entry when browser cleanup is blocked, and retry cannot resurrect it", async () => {
  const f = fixture("legacy-secret");
  f.blockRemoval(true);
  await f.controller.initialize();
  assert.equal(await f.controller.forget(), false);
  assert.equal(f.state().configured, true);
  assert.equal(f.state().legacyCleanupPending, true);
  assert.equal(f.state().recoveryPending, false);
  assert.match(f.state().error, /Could not remove.*plaintext copy may still remain/);
  assert.deepEqual(f.calls, [["migrate", "legacy-secret"]]);
  f.blockRemoval(false);
  assert.equal(await f.controller.retry(), true);
  assert.deepEqual(f.calls, [["migrate", "legacy-secret"], ["delete"]]);
  assert.equal(f.stored.has(LEGACY_DISCOGS_TOKEN_KEY), false);
  assert.equal(f.state().configured, false);
  assert.equal(f.state().legacyCleanupPending, false);
  assert.equal(f.state().error, null);
});

test("a credential save preserves newly typed input and clears only an unchanged successful draft", async () => {
  const revision = new DraftRevision("discogs");
  let input = "submitted";
  const writing = deferred();
  const save = finishCredentialDraft(
    revision,
    () => writing.promise,
    () => {
      input = "";
    },
  );
  revision.touch();
  input = "newly typed";
  writing.resolve(true);
  await save;
  assert.equal(input, "newly typed");
  await finishCredentialDraft(
    revision,
    async () => false,
    () => {
      input = "";
    },
  );
  assert.equal(input, "newly typed");
  await finishCredentialDraft(
    revision,
    async () => true,
    () => {
      input = "";
    },
  );
  assert.equal(input, "");
});

function providersFixture() {
  const calls = [];
  const providers = Object.fromEntries(
    ["lookupMusicbrainz", "lookupDiscogs", "lookupMusicbrainzDetail", "lookupDiscogsDetail"].map(
      (name) => [
        name,
        async (...args) => {
          calls.push([name, ...args]);
          return name.endsWith("Detail") ? { title: name } : [{ id: name }];
        },
      ],
    ),
  );
  return { providers, calls };
}

test("search starts MusicBrainz immediately while only Discogs waits for credential initialization", async () => {
  const { providers, calls } = providersFixture();
  const ready = deferred();
  const search = searchLookupProviders("album", () => ready.promise, providers);
  await tick();
  assert.deepEqual(calls, [["lookupMusicbrainz", "album"]]);
  ready.resolve(true);
  const result = await search;
  assert.deepEqual(calls, [
    ["lookupMusicbrainz", "album"],
    ["lookupDiscogs", "album"],
  ]);
  assert.equal(result.candidates.length, 2);
});

test("MusicBrainz detail does not wait for an unresolved credential initialization", async () => {
  const { providers, calls } = providersFixture();
  const ready = deferred();
  let readinessRequested = false;
  const release = await lookupCandidateDetail(
    { source: "MusicBrainz", id: "release" },
    () => {
      readinessRequested = true;
      return ready.promise;
    },
    providers,
  );
  assert.equal(release.title, "lookupMusicbrainzDetail");
  assert.equal(readinessRequested, false);
  assert.deepEqual(calls, [["lookupMusicbrainzDetail", "release"]]);
});

test("an unavailable vault skips Discogs while MusicBrainz search and detail still work", async () => {
  const { providers, calls } = providersFixture();
  await searchLookupProviders("album", async () => false, providers);
  await lookupCandidateDetail(
    { source: "MusicBrainz", id: "release" },
    async () => false,
    providers,
  );
  await assert.rejects(
    lookupCandidateDetail({ source: "Discogs", id: "123" }, async () => false, providers),
    /Discogs is unavailable/,
  );
  assert.deepEqual(calls, [
    ["lookupMusicbrainz", "album"],
    ["lookupMusicbrainzDetail", "release"],
  ]);
});

test("Discogs detail sends only an id after readiness, and provider failures remain independent", async () => {
  const { providers, calls } = providersFixture();
  await lookupCandidateDetail({ source: "Discogs", id: "123" }, async () => true, providers);
  assert.deepEqual(calls, [["lookupDiscogsDetail", "123"]]);
  providers.lookupDiscogs = async () => {
    throw new Error("service unavailable");
  };
  const result = await searchLookupProviders("album", async () => true, providers);
  assert.equal(result.candidates.length, 1);
  assert.match(result.errors[0], /Discogs.*service unavailable/);
});

const bundled = await build({
  stdin: {
    contents: `export * from "./src/api";
    export { calls } from "@tauri-apps/api/core";
    export { DiscogsCredentialSettings } from "./src/components/DiscogsCredentialSettings";
    export { createElement } from "react";
    export { renderToStaticMarkup } from "react-dom/server";`,
    resolveDir: process.cwd(),
  },
  bundle: true,
  write: false,
  platform: "node",
  format: "esm",
  jsx: "automatic",
  loader: { ".css": "empty" },
  banner: {
    js: 'import { createRequire } from "node:module"; const require = createRequire(process.cwd() + "/package.json");',
  },
  plugins: [
    {
      name: "credential-api-test",
      setup(builder) {
        builder.onResolve({ filter: /^@tauri-apps\/api\/(core|event)$/ }, (args) => ({
          path: args.path,
          namespace: "mock-tauri",
        }));
        builder.onLoad({ filter: /.*/, namespace: "mock-tauri" }, (args) => ({
          contents: args.path.endsWith("/core")
            ? "export const calls = []; export async function invoke(command, args) { calls.push({ command, args }); return { configured: true }; }"
            : "export async function emit() {}",
          loader: "js",
        }));
      },
    },
  ],
});
const actual = await import(
  `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`
);

test("credential readback exposes presence only and lookup IPC payloads never contain a token", async () => {
  actual.calls.length = 0;
  assert.deepEqual(await actual.getDiscogsCredentialStatus(), { configured: true });
  await actual.lookupDiscogs("album");
  await actual.lookupDiscogsDetail("123");
  await actual.deleteDiscogsToken();
  assert.deepEqual(actual.calls, [
    { command: "discogs_credential_status", args: undefined },
    { command: "lookup_discogs", args: { query: "album" } },
    { command: "lookup_discogs_detail", args: { id: "123" } },
    { command: "delete_discogs_token", args: undefined },
  ]);
  await actual.saveDiscogsToken("new-secret");
  await actual.migrateDiscogsToken("old-secret");
  assert.deepEqual(actual.calls.slice(-2), [
    { command: "save_discogs_token", args: { token: "new-secret" } },
    { command: "migrate_discogs_token", args: { token: "old-secret" } },
  ]);
});

test("saved and recovery credentials never fill the password input, while vault failures remain visible", () => {
  const credential = {
    configured: true,
    ready: true,
    busy: false,
    error: "Credential store is locked",
    recoveryPending: true,
    legacyCleanupPending: false,
    save: async () => true,
    forget: async () => true,
    retry: async () => true,
    readyForLookup: async () => true,
  };
  const html = actual.renderToStaticMarkup(
    actual.createElement(actual.DiscogsCredentialSettings, { credential }),
  );
  assert.match(html, /type="password"/);
  assert.match(html, /value=""/);
  assert.match(html, /Token saved in system credential storage/);
  assert.match(html, /role="alert"[^>]*>Credential store is locked/);
  assert.match(html, /Save token/);
  assert.match(html, /Forget token/);
  assert.match(html, /Retry secure storage/);
});

import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { TagCache } = await loadTypeScript(
  new URL("../src/components/tagCache.ts", import.meta.url),
);
const { FilePresence, filePresenceKey } = await loadTypeScript(
  new URL("../src/components/filePresence.ts", import.meta.url),
);
const { LatestRequest } = await loadTypeScript(
  new URL("../src/components/latestRequest.ts", import.meta.url),
);
const { DraftRevision } = await loadTypeScript(
  new URL("../src/components/draftRevision.ts", import.meta.url),
);

function deferred() {
  let resolve, reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function cacheFixture() {
  const reads = [];
  let snapshot;
  const cache = new TagCache(
    (paths) => {
      const read = { paths, ...deferred() };
      reads.push(read);
      return read.promise;
    },
    (next) => {
      snapshot = next;
    },
  );
  return { cache, reads, snapshot: () => snapshot };
}

const tags = (title) => ({ title, pictures: [] });
const flush = () => Promise.resolve();
async function assertOutcome(promise, expected) {
  const { isCurrent, ...outcome } = await promise;
  assert.deepEqual(outcome, expected);
  assert.equal(isCurrent(), expected.status !== "superseded");
}

test("overlapping tag reads request each path only once, including duplicate inputs", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  const first = cache.fetchMissing(["a", "a", "b"]);
  await cache.fetchMissing(["b", "a"]);
  assert.equal(reads.length, 1);
  assert.deepEqual(reads[0].paths, ["a", "b"]);
  reads[0].resolve([
    { path: "a", tags: tags("A") },
    { path: "b", tags: null },
  ]);
  await first;
  await cache.fetchMissing(["a", "b"]);
  assert.equal(reads.length, 1);
  assert.equal(snapshot().tags.get("b"), null);
  assert.equal(snapshot().covers.get("b"), null);
});

test("an old tag response cannot overwrite freshly saved tags and artwork", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  const old = cache.fetchMissing(["a"]);
  cache.invalidate(["a"]);
  const front = { picture_type: "CoverFront", data_base64: "fresh" };
  const back = { picture_type: "CoverBack", data_base64: "back" };
  reads[1].resolve([{ path: "a", tags: { title: "Fresh", pictures: [back, front] } }]);
  await flush();
  reads[0].resolve([{ path: "a", tags: tags("Stale") }]);
  await old;
  assert.equal(snapshot().tags.get("a").title, "Fresh");
  assert.equal(snapshot().covers.get("a"), front);
});

test("a failed old tag read cannot release a replacement request", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  const old = cache.fetchMissing(["a"]);
  cache.invalidate(["a"]);
  reads[0].reject(new Error("old read failed"));
  await old;
  await cache.fetchMissing(["a"]);
  assert.equal(reads.length, 2);
  reads[1].resolve([{ path: "a", tags: tags("Fresh") }]);
  await flush();
  assert.equal(snapshot().tags.get("a").title, "Fresh");
});

test("clearing the list prevents a late tag read from rebuilding its cache", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  const old = cache.fetchMissing(["a"]);
  cache.clear();
  reads[0].resolve([{ path: "a", tags: tags("Stale") }]);
  await old;
  assert.equal(snapshot().tags.size, 0);
  assert.equal(snapshot().covers.size, 0);
  cache.retain(new Set(["a"]));
  const fresh = cache.fetchMissing(["a"]);
  assert.equal(reads.length, 2);
  reads[1].resolve([{ path: "a", tags: null }]);
  await fresh;
});

test("a save finishing after Reset cannot refetch the cleared listing", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  cache.retain(new Set(["a"]));
  const first = cache.fetchMissing(["a"]);
  reads[0].resolve([{ path: "a", tags: tags("Original") }]);
  await first;
  const save = deferred();
  const completed = save.promise.then(() => cache.invalidate(["a"]));
  cache.clear();
  save.resolve();
  await completed;
  assert.equal(reads.length, 1);
  assert.equal(snapshot().tags.size, 0);
  assert.equal(snapshot().covers.size, 0);
});

test("a save finishing after row removal refreshes only paths still in the listing", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  cache.retain(new Set(["a", "b"]));
  const first = cache.fetchMissing(["a", "b"]);
  reads[0].resolve([
    { path: "a", tags: tags("Removed") },
    { path: "b", tags: tags("Kept") },
  ]);
  await first;
  const save = deferred();
  const completed = save.promise.then(() => cache.invalidate(["a", "b"]));
  cache.retain(new Set(["b"]));
  save.resolve();
  await completed;
  assert.equal(reads.length, 2);
  assert.deepEqual(reads[1].paths, ["b"]);
  reads[1].resolve([{ path: "b", tags: tags("Saved") }]);
  await flush();
  assert.equal(snapshot().tags.has("a"), false);
  assert.equal(snapshot().tags.get("b").title, "Saved");
});

test("a newly imported listing gets its first prefetch after an empty cache", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  cache.retain(new Set());
  await cache.fetchMissing(["old"]);
  assert.equal(reads.length, 0);
  cache.retain(new Set(["new"]));
  const first = cache.fetchMissing(["new"]);
  assert.deepEqual(reads[0].paths, ["new"]);
  reads[0].resolve([{ path: "new", tags: tags("Imported") }]);
  await first;
  assert.equal(snapshot().tags.get("new").title, "Imported");
});

test("removed rows release cached covers and reject outstanding reads for their paths", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  const old = cache.fetchMissing(["a", "b"]);
  cache.retain(new Set(["b"]));
  reads[0].resolve([
    { path: "a", tags: tags("Removed") },
    { path: "b", tags: tags("Kept") },
  ]);
  await old;
  assert.deepEqual([...snapshot().tags.keys()], ["b"]);
  cache.retain(new Set());
  assert.equal(snapshot().tags.size, 0);
  assert.equal(snapshot().covers.size, 0);
});

test("incomplete and failed tag batches remain retryable without admitting unrequested paths", async () => {
  const { cache, reads, snapshot } = cacheFixture();
  const first = cache.fetchMissing(["a", "b"]);
  reads[0].resolve([
    { path: "a", tags: null },
    { path: "unexpected", tags: tags("Wrong") },
  ]);
  await first;
  assert.equal(snapshot().tags.has("unexpected"), false);
  const retry = cache.fetchMissing(["a", "b"]);
  assert.deepEqual(reads[1].paths, ["b"]);
  reads[1].reject(new Error("retry failed"));
  await retry;
  const final = cache.fetchMissing(["b"]);
  reads[2].resolve([{ path: "b", tags: null }]);
  await final;
});

function presenceFixture() {
  const reads = [],
    reappeared = [],
    errors = [];
  let snapshot;
  const presence = new FilePresence(
    (paths) => {
      const read = { paths, ...deferred() };
      reads.push(read);
      return read.promise;
    },
    (next) => {
      snapshot = next;
    },
    (paths) => reappeared.push(paths),
    (error) => errors.push(error),
  );
  return { presence, reads, reappeared, errors, snapshot: () => snapshot };
}

test("reports with the same length and first path still get distinct presence checks", () => {
  assert.notEqual(filePresenceKey(["a", "b"]), filePresenceKey(["a", "c"]));
  assert.equal(filePresenceKey(["a", "b"]), filePresenceKey(["b", "a"]));
  assert.notEqual(filePresenceKey(["a|b", "c"]), filePresenceKey(["a", "b|c"]));
});

test("a stale presence check cannot replace a newer result or clear its spinner", async () => {
  const { presence, reads, snapshot } = presenceFixture();
  let staleNotifications = 0;
  const old = presence.check(["a"], () => staleNotifications++);
  const fresh = presence.check(["b"]);
  reads[0].resolve(["a"]);
  assert.equal(await old, null);
  assert.equal(staleNotifications, 0);
  assert.equal(snapshot().checking, true);
  reads[1].resolve(["b"]);
  assert.deepEqual([...(await fresh)], ["b"]);
  assert.deepEqual([...snapshot().missing], ["b"]);
  assert.equal(snapshot().checking, false);
});

test("presence reset suppresses late errors and stale missing paths", async () => {
  const { presence, reads, snapshot, errors } = presenceFixture();
  const old = presence.check(["a"]);
  presence.reset();
  reads[0].reject(new Error("old failure"));
  assert.equal(await old, null);
  assert.equal(errors.length, 0);
  assert.equal(snapshot().checking, false);
  assert.equal(snapshot().missing.size, 0);
});

test("only returning files still in the listing invalidate their tags, exactly once", async () => {
  const { presence, reads, reappeared } = presenceFixture();
  const first = presence.check(["a", "b"]);
  reads[0].resolve(["a", "b"]);
  await first;
  const second = presence.check(["b"]);
  reads[1].resolve([]);
  await second;
  assert.deepEqual(reappeared, [["b"]]);
  const third = presence.check(["b"]);
  reads[2].resolve([]);
  await third;
  assert.deepEqual(reappeared, [["b"]]);
});

test("failed presence refresh returns no success result and preserves known missing files", async () => {
  const { presence, reads, errors, snapshot } = presenceFixture();
  const first = presence.check(["a"]);
  reads[0].resolve(["a"]);
  await first;
  const refresh = presence.check(["a"]);
  const error = new Error("cannot read filesystem");
  reads[1].reject(error);
  assert.equal(await refresh, null);
  assert.deepEqual(errors, [error]);
  assert.deepEqual([...snapshot().missing], ["a"]);
  assert.equal(snapshot().checking, false);
});

test("play followed by stop discards a delayed play response", async () => {
  const requests = new LatestRequest();
  const read = deferred();
  const play = requests.run(() => read.promise);
  requests.cancel();
  read.resolve(42);
  await assertOutcome(play, { status: "superseded" });
});

test("older playback or lookup failures cannot replace a successful newer request", async () => {
  const requests = new LatestRequest();
  const old = deferred();
  const first = requests.run(() => old.promise);
  await assertOutcome(
    requests.run(async () => 43),
    { status: "success", value: 43 },
  );
  old.reject(new Error("outdated error"));
  await assertOutcome(first, { status: "superseded" });
});

test("closing lookup discards pending detail while current failures remain visible", async () => {
  const requests = new LatestRequest();
  const old = deferred();
  const detail = requests.run(() => old.promise);
  requests.cancel();
  old.resolve({ title: "Old release" });
  await assertOutcome(detail, { status: "superseded" });
  const error = new Error("current error");
  await assertOutcome(
    requests.run(async () => {
      throw error;
    }),
    { status: "error", error },
  );
});

test("a completed tag save cannot clear edits made while it was pending", () => {
  const draft = new DraftRevision(JSON.stringify(["a.flac"]));
  const saved = draft.capture();
  draft.touch();
  assert.equal(draft.isCurrent(saved), false);
  const current = draft.capture();
  assert.equal(draft.isCurrent(current), true);
  assert.equal(draft.setScope(JSON.stringify(["a.flac"])), false);
  assert.equal(draft.isCurrent(current), true);
});

test("a delayed tag save cannot clear another selection even after switching back", () => {
  const firstScope = JSON.stringify(["a.flac"]);
  const draft = new DraftRevision(firstScope);
  const saved = draft.capture();
  assert.equal(draft.setScope(JSON.stringify(["b.flac"])), true);
  assert.equal(draft.isCurrent(saved), false);
  draft.setScope(firstScope);
  assert.equal(draft.isCurrent(saved), false);
});

test("selection identity does not collide for filenames containing the old separator", () => {
  const draft = new DraftRevision(JSON.stringify(["a|b", "c"]));
  const old = draft.capture();
  assert.equal(draft.setScope(JSON.stringify(["a", "b|c"])), true);
  assert.equal(draft.isCurrent(old), false);
});

test("a dropped cover belongs to its original files and picture role", async () => {
  const scope = new DraftRevision(JSON.stringify([["a.flac"], "CoverFront"]));
  const reads = new LatestRequest();
  const original = scope.capture();
  const read = deferred();
  const dropped = reads.run(() => read.promise);
  scope.setScope(JSON.stringify([["a.flac"], "CoverBack"]));
  reads.cancel();
  read.resolve({ picture_type: "CoverFront", data_base64: "old" });
  await assertOutcome(dropped, { status: "superseded" });
  assert.equal(scope.isCurrent(original), false);
});

test("stop between request completion and the consumer continuation still rejects the result", async () => {
  const requests = new LatestRequest();
  const read = deferred();
  const pending = requests.run(() => read.promise);
  read.resolve(42);
  queueMicrotask(() => requests.cancel());
  const result = await pending;
  assert.equal(result.status, "success");
  assert.equal(result.isCurrent(), false);
});

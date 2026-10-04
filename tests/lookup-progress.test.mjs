import assert from "node:assert/strict";
import test from "node:test";
import { loadTypeScript } from "./load-typescript.mjs";

const { LatestRequest } = await loadTypeScript(
  new URL("../src/components/latestRequest.ts", import.meta.url),
);
const { searchLookupProviders, lookupCandidateDetail, lookupCandidateKey, lookupSearchStatus } =
  await loadTypeScript(new URL("../src/components/lookupProviders.ts", import.meta.url));

function deferred() {
  let resolve, reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function flush() {
  for (let i = 0; i < 4; i++) await Promise.resolve();
}

function candidate(source, id, title = id) {
  return { source, id, title, artist: "Artist", year: null, track_count: null };
}

function fixture() {
  const musicbrainz = deferred(),
    discogs = deferred(),
    ready = deferred(),
    detail = deferred();
  const calls = [],
    progress = [];
  const requests = new LatestRequest();
  const providers = {
    lookupMusicbrainz: (query) => {
      calls.push(["MusicBrainz", query]);
      return musicbrainz.promise;
    },
    lookupDiscogs: (query) => {
      calls.push(["Discogs", query]);
      return discogs.promise;
    },
    lookupMusicbrainzDetail: () => detail.promise,
    lookupDiscogsDetail: () => detail.promise,
  };
  const search = () =>
    requests.run((isCurrent) =>
      searchLookupProviders("album", () => ready.promise, providers, {
        isCurrent,
        publish: (snapshot) => progress.push(snapshot),
      }),
    );
  return { requests, providers, search, calls, progress, musicbrainz, discogs, ready, detail };
}

test("MusicBrainz publishes usable results while vault readiness never resolves", async () => {
  const f = fixture();
  let finished = false;
  void f.search().then(() => {
    finished = true;
  });
  const release = candidate("MusicBrainz", "mb-release");
  f.musicbrainz.resolve([release]);
  await flush();
  assert.deepEqual(f.progress, [
    {
      candidates: [release],
      errors: [],
      musicbrainzFinished: true,
    },
  ]);
  assert.equal(finished, false);
  assert.deepEqual(f.calls, [["MusicBrainz", "album"]]);
  f.requests.cancel();
});

test("each provider publishes a snapshot, preserving provider order and unique stable identities", async () => {
  const f = fixture();
  const pending = f.search();
  const dg = candidate("Discogs", "shared"),
    mb = candidate("MusicBrainz", "shared");
  f.ready.resolve(true);
  await flush();
  f.discogs.resolve([dg, candidate("Discogs", "shared", "duplicate")]);
  await flush();
  assert.deepEqual(f.progress, [{ candidates: [dg], errors: [], musicbrainzFinished: false }]);
  f.musicbrainz.resolve([mb, candidate("MusicBrainz", "shared", "duplicate")]);
  const result = await pending;
  assert.equal(result.status, "success");
  assert.deepEqual(result.value, { candidates: [mb, dg], errors: [] });
  assert.deepEqual(f.progress[1], { candidates: [mb, dg], errors: [], musicbrainzFinished: true });
  assert.deepEqual(f.progress[0].candidates, [dg]);
  assert.notEqual(lookupCandidateKey(mb), lookupCandidateKey(dg));
  assert.equal(lookupCandidateKey(mb), lookupCandidateKey({ ...mb, title: "Changed" }));
});

test("a MusicBrainz failure releases its progress gate and remains in the final partial-error result", async () => {
  const f = fixture();
  const pending = f.search();
  f.musicbrainz.reject(new Error("MusicBrainz unavailable"));
  await flush();
  assert.equal(f.progress[0].musicbrainzFinished, true);
  assert.deepEqual(f.progress[0].candidates, []);
  assert.match(f.progress[0].errors[0], /MusicBrainz unavailable/);
  assert.deepEqual(lookupSearchStatus(f.progress[0], false), {
    msg: "MusicBrainz: Error: MusicBrainz unavailable",
    kind: "error",
  });
  f.ready.resolve(true);
  await flush();
  const dg = candidate("Discogs", "dg-release");
  f.discogs.resolve([dg]);
  const result = await pending;
  assert.equal(result.status, "success");
  assert.deepEqual(result.value.candidates, [dg]);
  assert.deepEqual(result.value.errors, f.progress[0].errors);
});

test("an empty MusicBrainz response shows a pending status while the vault never resolves", async () => {
  const f = fixture();
  void f.search();
  f.musicbrainz.resolve([]);
  await flush();
  assert.equal(f.progress[0].musicbrainzFinished, true);
  assert.deepEqual(lookupSearchStatus(f.progress[0], false), {
    msg: "No results yet.",
    kind: "info",
  });
  assert.deepEqual(lookupSearchStatus(f.progress[0], true), { msg: "No results.", kind: "info" });
  f.requests.cancel();
});

test("Reset suppresses every late provider notification and the final search result", async () => {
  const f = fixture();
  const pending = f.search();
  f.requests.cancel();
  f.ready.resolve(true);
  f.musicbrainz.resolve([candidate("MusicBrainz", "old-mb")]);
  await flush();
  f.discogs.resolve([candidate("Discogs", "old-dg")]);
  const result = await pending;
  assert.equal(result.status, "superseded");
  assert.equal(result.isCurrent(), false);
  assert.deepEqual(f.progress, []);
});

test("a newer search keeps its results when both old provider responses arrive later", async () => {
  const f = fixture();
  const old = f.search();
  f.ready.resolve(true);
  await flush();
  const fresh = candidate("MusicBrainz", "fresh");
  const current = f.requests.run((isCurrent) =>
    searchLookupProviders(
      "new album",
      async () => false,
      { ...f.providers, lookupMusicbrainz: async () => [fresh] },
      { isCurrent, publish: (snapshot) => f.progress.push(snapshot) },
    ),
  );
  assert.equal((await current).status, "success");
  f.musicbrainz.resolve([candidate("MusicBrainz", "old-mb")]);
  f.discogs.resolve([candidate("Discogs", "old-dg")]);
  assert.equal((await old).status, "superseded");
  assert.deepEqual(f.progress, [{ candidates: [fresh], errors: [], musicbrainzFinished: true }]);
});

test("selecting a detail suppresses pending search progress without superseding the detail", async () => {
  const f = fixture();
  const search = f.search();
  const mb = candidate("MusicBrainz", "mb-release");
  f.musicbrainz.resolve([mb]);
  await flush();
  assert.equal(f.progress.length, 1);
  const detail = f.requests.run(() =>
    lookupCandidateDetail(mb, () => f.ready.promise, f.providers),
  );
  f.ready.resolve(true);
  await flush();
  f.discogs.resolve([candidate("Discogs", "late-dg")]);
  assert.equal((await search).status, "superseded");
  assert.equal(f.progress.length, 1);
  const release = { title: "Selected release" };
  f.detail.resolve(release);
  const result = await detail;
  assert.equal(result.status, "success");
  assert.equal(result.isCurrent(), true);
  assert.deepEqual(result.value, release);
});

test("a completed search notification still becomes stale before its consumer commits", async () => {
  const f = fixture();
  const search = f.search();
  f.ready.resolve(false);
  f.musicbrainz.resolve([candidate("MusicBrainz", "mb-release")]);
  const result = await search;
  assert.equal(result.status, "success");
  f.requests.cancel();
  assert.equal(result.isCurrent(), false);
});

import type { LookupCandidate, LookupRelease, LookupSource } from "../types";

interface LookupProviders {
  lookupMusicbrainz(query: string): Promise<LookupCandidate[]>;
  lookupDiscogs(query: string): Promise<LookupCandidate[]>;
  lookupMusicbrainzDetail(id: string): Promise<LookupRelease>;
  lookupDiscogsDetail(id: string): Promise<LookupRelease>;
}

interface LookupSearchResult {
  candidates: LookupCandidate[];
  errors: string[];
}

interface LookupSearchProgress extends LookupSearchResult {
  musicbrainzFinished: boolean;
}

interface LookupProgressListener {
  isCurrent: () => boolean;
  publish: (progress: LookupSearchProgress) => void;
}

export function lookupCandidateKey(candidate: Pick<LookupCandidate, "source" | "id">): string {
  return `${candidate.source}:${candidate.id}`;
}

export function lookupSearchStatus(
  result: LookupSearchResult,
  complete: boolean,
): { msg: string; kind: "info" | "error" } {
  if (result.candidates.length) return { msg: "", kind: "info" };
  if (result.errors.length) return { msg: result.errors.join(" · "), kind: "error" };
  return { msg: complete ? "No results." : "No results yet.", kind: "info" };
}

// Initialization/migration must settle before Discogs reads the vault. A
// missing or locked vault still permits independent MusicBrainz requests.
// Publish completed providers immediately: a native unlock prompt can remain
// open indefinitely, so waiting for the whole search would hide usable data.
export async function searchLookupProviders(
  query: string,
  readyForLookup: () => Promise<boolean>,
  providers: LookupProviders,
  progress?: LookupProgressListener,
): Promise<LookupSearchResult> {
  const errors: string[] = [];
  const candidates = new Map<LookupSource, LookupCandidate[]>();
  let musicbrainzFinished = false;
  const snapshot = (): LookupSearchResult => {
    const unique = new Map<string, LookupCandidate>();
    for (const source of ["MusicBrainz", "Discogs"] as const) {
      for (const candidate of candidates.get(source) ?? []) {
        const key = lookupCandidateKey(candidate);
        if (!unique.has(key)) unique.set(key, candidate);
      }
    }
    return { candidates: [...unique.values()], errors: [...errors] };
  };
  async function read(source: LookupSource, load: () => Promise<LookupCandidate[]>): Promise<void> {
    try {
      candidates.set(source, await load());
    } catch (error) {
      errors.push(`${source}: ${String(error)}`);
    }
    if (source === "MusicBrainz") musicbrainzFinished = true;
    if (progress?.isCurrent()) progress.publish({ ...snapshot(), musicbrainzFinished });
  }
  const musicbrainz = read("MusicBrainz", () => providers.lookupMusicbrainz(query));
  const discogs = (async () => {
    if (await readyForLookup().catch(() => false)) {
      await read("Discogs", () => providers.lookupDiscogs(query));
    }
  })();
  await Promise.all([musicbrainz, discogs]);
  return snapshot();
}

export async function lookupCandidateDetail(
  candidate: LookupCandidate,
  readyForLookup: () => Promise<boolean>,
  providers: LookupProviders,
): Promise<LookupRelease> {
  if (candidate.source === "MusicBrainz") return providers.lookupMusicbrainzDetail(candidate.id);
  const configured = await readyForLookup().catch(() => false);
  if (!configured)
    throw new Error(
      "Discogs is unavailable. Save a token in system credential storage, then retry.",
    );
  return providers.lookupDiscogsDetail(candidate.id);
}

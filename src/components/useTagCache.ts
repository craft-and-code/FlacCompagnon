// Lazily-fetched tags and cover art, shared by the table's thumbnail column
// and the tag panel.
//
// One cache, one fetch: selecting a row whose thumbnail already loaded doesn't
// re-read its tags. A `null` entry means "fetched, nothing there" — distinct
// from a missing key ("not fetched yet"), which is what lets the table render
// placeholders immediately instead of blocking on a batch read.

import { useCallback, useEffect, useRef, useState } from "react";

import type { CoverArt, TagSet } from "../types";
import * as api from "../api";

export function useTagCache() {
  const [tags, setTags] = useState<Map<string, TagSet | null>>(new Map());
  const [covers, setCovers] = useState<Map<string, CoverArt | null>>(new Map());
  // Paths already requested, so overlapping renders don't fire duplicate
  // batches for the same files while the first is still in flight.
  const inFlight = useRef(new Set<string>());

  const fetchMissing = useCallback(async (paths: string[]) => {
    const missing = paths.filter((p) => !inFlight.current.has(p));
    if (missing.length === 0) return;
    for (const p of missing) inFlight.current.add(p);

    let results;
    try {
      results = await api.readTagsBatch(missing);
    } catch {
      // Best-effort: a failed batch just leaves placeholders showing. Allow a
      // later attempt rather than marking these as permanently fetched.
      for (const p of missing) inFlight.current.delete(p);
      return;
    }

    setTags((prev) => {
      const next = new Map(prev);
      for (const r of results) next.set(r.path, r.tags);
      return next;
    });
    setCovers((prev) => {
      const next = new Map(prev);
      for (const r of results) next.set(r.path, r.tags?.cover ?? null);
      return next;
    });
  }, []);

  /// Re-read these paths from disk — used after a successful tag write.
  ///
  /// The refetch is kicked off here rather than left to the prefetch effect:
  /// that effect only fires when the *set of visible paths* changes, which a
  /// save doesn't do, so dropping the entries alone would leave the panel
  /// showing nothing until the selection changed.
  const invalidate = useCallback(
    (paths: string[]) => {
      for (const p of paths) inFlight.current.delete(p);
      setTags((prev) => {
        const next = new Map(prev);
        for (const p of paths) next.delete(p);
        return next;
      });
      setCovers((prev) => {
        const next = new Map(prev);
        for (const p of paths) next.delete(p);
        return next;
      });
      void fetchMissing(paths);
    },
    [fetchMissing],
  );

  const clear = useCallback(() => {
    inFlight.current.clear();
    setTags(new Map());
    setCovers(new Map());
  }, []);

  /// Tags for a selection, with unreadable/unsupported files dropped.
  const tagSetsFor = useCallback(
    (paths: string[]): TagSet[] =>
      paths.map((p) => tags.get(p)).filter((t): t is TagSet => t != null),
    [tags],
  );

  return { tags, covers, fetchMissing, invalidate, clear, tagSetsFor };
}

/// Keeps the cache filled for whatever paths are currently on screen.
export function useTagPrefetch(paths: string[], fetchMissing: (paths: string[]) => void) {
  useEffect(() => {
    if (paths.length > 0) fetchMissing(paths);
  }, [paths, fetchMissing]);
}

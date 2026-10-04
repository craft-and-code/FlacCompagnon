// Lazily-fetched tags and cover art, shared by the table's thumbnail column
// and the tag panel.
//
// One cache, one fetch: selecting a row whose thumbnail already loaded doesn't
// re-read its tags. A `null` entry means "fetched, nothing there" — distinct
// from a missing key ("not fetched yet"), which is what lets the table render
// placeholders immediately instead of blocking on a batch read.

import { useCallback, useEffect, useState } from "react";

import type { CoverArt, TagSet } from "../types";
import * as api from "../api";
import { TagCache, type TagCacheSnapshot } from "./tagCache";

export function useTagCache() {
  const [{ tags, covers }, setSnapshot] = useState<TagCacheSnapshot>(() => ({
    tags: new Map<string, TagSet | null>(),
    covers: new Map<string, CoverArt | null>(),
  }));
  const [cache] = useState(() => new TagCache(api.readTagsBatch, setSnapshot));
  const fetchMissing = useCallback((paths: string[]) => cache.fetchMissing(paths), [cache]);

  /// Re-read these paths from disk — used after a successful tag write.
  ///
  /// The refetch is kicked off here rather than left to the prefetch effect:
  /// that effect only fires when the *set of visible paths* changes, which a
  /// save doesn't do, so dropping the entries alone would leave the panel
  /// showing nothing until the selection changed.
  const invalidate = useCallback(
    (paths: string[]) => cache.invalidate(paths),
    [cache],
  );

  const clear = useCallback(() => cache.clear(), [cache]);
  const retain = useCallback((present: Set<string>) => cache.retain(present), [cache]);

  /// Tags for a selection, with unreadable/unsupported files dropped.
  const tagSetsFor = useCallback(
    (paths: string[]): TagSet[] =>
      paths.map((p) => tags.get(p)).filter((t): t is TagSet => t != null),
    [tags],
  );

  return { tags, covers, fetchMissing, invalidate, clear, retain, tagSetsFor };
}

/// Keeps the cache filled for whatever paths are currently on screen.
export function useTagPrefetch(paths: string[], fetchMissing: (paths: string[]) => void) {
  useEffect(() => {
    if (paths.length > 0) fetchMissing(paths);
  }, [paths, fetchMissing]);
}

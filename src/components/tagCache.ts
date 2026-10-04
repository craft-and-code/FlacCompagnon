// Each read owns the paths it requested until invalidation or removal. A slow
// read from before a save/reset must not restore stale tags or cover art.

import type { CoverArt, TagReadResult, TagSet } from "../types";

export interface TagCacheSnapshot {
  tags: Map<string, TagSet | null>;
  covers: Map<string, CoverArt | null>;
}

export class TagCache {
  private snapshot: TagCacheSnapshot = { tags: new Map(), covers: new Map() };
  private requests = new Map<string, symbol>();
  // Until the owner declares its listing, the first prefetch can establish
  // the cache. Afterwards only that listing may request or invalidate paths:
  // a tag write can finish after its rows have already been removed.
  private listedPaths: Set<string> | null = null;

  constructor(
    private read: (paths: string[]) => Promise<TagReadResult[]>,
    private publish: (snapshot: TagCacheSnapshot) => void,
  ) {}

  async fetchMissing(paths: string[]): Promise<void> {
    const missing = [...new Set(paths)].filter(
      (path) => this.owns(path) && !this.requests.has(path),
    );
    if (missing.length === 0) return;
    const request = Symbol();
    for (const path of missing) this.requests.set(path, request);

    let results: TagReadResult[];
    try {
      results = await this.read(missing);
    } catch {
      for (const path of missing) {
        if (this.requests.get(path) === request) this.requests.delete(path);
      }
      return;
    }

    const current = results.filter((result) => this.requests.get(result.path) === request);
    const returned = new Set(current.map((result) => result.path));
    for (const path of missing) {
      // An incomplete batch must remain retryable, like a rejected read.
      if (!returned.has(path) && this.requests.get(path) === request) this.requests.delete(path);
    }
    if (current.length === 0) return;

    const tags = new Map(this.snapshot.tags);
    const covers = new Map(this.snapshot.covers);
    for (const result of current) {
      tags.set(result.path, result.tags);
      const pictures = result.tags?.pictures ?? [];
      covers.set(
        result.path,
        pictures.find((picture) => picture.picture_type === "CoverFront") ?? pictures[0] ?? null,
      );
    }
    this.commit({ tags, covers });
  }

  invalidate(paths: string[]): void {
    const owned = paths.filter((path) => this.owns(path));
    if (owned.length === 0) return;
    const tags = new Map(this.snapshot.tags);
    const covers = new Map(this.snapshot.covers);
    for (const path of owned) {
      this.requests.delete(path);
      tags.delete(path);
      covers.delete(path);
    }
    this.commit({ tags, covers });
    void this.fetchMissing(owned);
  }

  retain(present: Set<string>): void {
    this.listedPaths = new Set(present);
    const removed = [...this.requests.keys()].filter((path) => !present.has(path));
    if (removed.length === 0) return;
    const tags = new Map(this.snapshot.tags);
    const covers = new Map(this.snapshot.covers);
    for (const path of removed) {
      this.requests.delete(path);
      tags.delete(path);
      covers.delete(path);
    }
    this.commit({ tags, covers });
  }

  clear(): void {
    this.listedPaths = new Set();
    this.requests.clear();
    this.commit({ tags: new Map(), covers: new Map() });
  }

  private owns(path: string): boolean {
    return this.listedPaths == null || this.listedPaths.has(path);
  }

  private commit(snapshot: TagCacheSnapshot): void {
    this.snapshot = snapshot;
    this.publish(snapshot);
  }
}

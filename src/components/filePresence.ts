// Filesystem checks may finish after a different report or a newer refresh.
// Keep both the result and its notifications tied to the request that owns it.

export interface FilePresenceSnapshot {
  missing: Set<string>;
  checking: boolean;
}

export class FilePresence {
  private missing = new Set<string>();
  private request: symbol | null = null;

  constructor(
    private read: (paths: string[]) => Promise<string[]>,
    private publish: (snapshot: FilePresenceSnapshot) => void,
    private onReappeared: (paths: string[]) => void,
    private onError: (error: unknown) => void,
  ) {}

  reset(): void {
    this.request = null;
    this.missing = new Set();
    this.publish({ missing: this.missing, checking: false });
  }

  async check(
    paths: string[],
    onChecked?: (missing: Set<string>) => void,
  ): Promise<Set<string> | null> {
    if (paths.length === 0) {
      this.reset();
      return this.missing;
    }
    const request = Symbol();
    this.request = request;
    this.publish({ missing: this.missing, checking: true });
    try {
      const now = new Set(await this.read(paths));
      if (request !== this.request) return null;
      const present = new Set(paths);
      const back = [...this.missing].filter((path) => present.has(path) && !now.has(path));
      this.missing = now;
      this.publish({ missing: now, checking: false });
      if (back.length > 0) this.onReappeared(back);
      onChecked?.(now);
      return now;
    } catch (error) {
      if (request !== this.request) return null;
      this.publish({ missing: this.missing, checking: false });
      this.onError(error);
      return null;
    }
  }
}

// Every path contributes to identity; equal length/first path does not mean
// that two imported reports describe the same files. Order is immaterial.
export function filePresenceKey(paths: string[]): string {
  return JSON.stringify([...new Set(paths)].sort());
}

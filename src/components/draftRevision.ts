// A completed save may clear only the draft it actually wrote. Selection
// changes and edits made while the write is pending create a new revision.

export class DraftRevision {
  private revision = Symbol();

  constructor(private scope: string) {}

  setScope(scope: string): boolean {
    if (scope === this.scope) return false;
    this.scope = scope;
    this.touch();
    return true;
  }

  touch(): void {
    this.revision = Symbol();
  }

  capture(): symbol {
    return this.revision;
  }

  isCurrent(revision: symbol): boolean {
    return revision === this.revision;
  }
}

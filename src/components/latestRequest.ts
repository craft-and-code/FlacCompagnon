// Backend operations cannot be aborted from the webview. Superseding a
// request must still discard both its result and any delayed failure.

export type RequestResult<T> = (
  { status: "success"; value: T } | { status: "error"; error: unknown } | { status: "superseded" }
) & { isCurrent: () => boolean };

export class LatestRequest {
  private current: symbol | null = null;

  cancel(): void {
    this.current = null;
  }

  async run<T>(task: () => Promise<T>): Promise<RequestResult<T>> {
    const request = Symbol();
    this.current = request;
    // Awaiting run() adds another microtask after this method's own await.
    // Consumers recheck immediately before committing, since cancel() can
    // run between those two continuations.
    const isCurrent = () => this.current === request;
    try {
      const value = await task();
      return this.current === request
        ? { status: "success", value, isCurrent }
        : { status: "superseded", isCurrent };
    } catch (error) {
      return this.current === request
        ? { status: "error", error, isCurrent }
        : { status: "superseded", isCurrent };
    }
  }
}

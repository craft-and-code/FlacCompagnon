// Show feedback while an optional panel's code and stylesheet are loading.
import { Suspense, type ReactNode } from "react";

export function DeferredPanel({ children }: { children: ReactNode }) {
  return (
    <Suspense
      fallback={
        <div role="status">
          <span className="spinner" /> Loading…
        </div>
      }
    >
      {children}
    </Suspense>
  );
}

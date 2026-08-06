// Transient status messages.
//
// Only one is ever visible, so each new message resets the pending hide timer —
// otherwise an earlier toast's timer would cut a later one short, which is
// what made the second of two errors in quick succession barely readable.

import { useCallback, useEffect, useRef, useState } from "react";

const VISIBLE_MS = 4200;

export interface ToastState {
  msg: string;
  kind: "info" | "error";
  /// Bumped on every call so re-showing an identical message still restarts
  /// the timer.
  seq: number;
}

export function useToast() {
  const [toast, setToast] = useState<ToastState | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const seq = useRef(0);

  const showToast = useCallback((msg: string, kind: "info" | "error" = "info") => {
    setToast({ msg, kind, seq: ++seq.current });
  }, []);

  useEffect(() => {
    if (!toast) return;
    if (timer.current != null) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setToast(null), VISIBLE_MS);
    return () => {
      if (timer.current != null) window.clearTimeout(timer.current);
    };
  }, [toast]);

  return { toast, showToast };
}

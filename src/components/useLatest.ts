// Keeps a ref pointing at the most recent value.
//
// Tauri's `listen`/`onDragDropEvent` return a *promise* of an unsubscribe
// function, so tearing a subscription down is asynchronous: if an effect
// re-runs because a callback identity changed, the new listener is attached
// before the old one is actually removed, and for that window both fire —
// which for the file-drop handler meant a dropped folder could be analyzed
// twice. Subscribing once and reading the live callback through a ref avoids
// the re-subscription entirely.

import { useRef } from "react";

export function useLatest<T>(value: T) {
  const ref = useRef(value);
  ref.current = value;
  return ref;
}

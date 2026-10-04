// Window-level keyboard shortcuts, with the two guards every one of them
// needs.
//
// Extracted because there were two hand-rolled `document.addEventListener`
// effects with the same preamble copied into both, and arrow-key navigation
// would have made three. The preamble is not boilerplate — each line of it is
// a bug that has to be prevented individually — so three copies means three
// places to forget one:
//
//   * a keystroke inside a text field belongs to that field. Backspace in the
//     search box must delete a character, not the selected rows; Cmd+A must
//     select the text, not every track.
//   * a modal on screen owns the keyboard. Every pop-in in this app renders
//     through `Modal`'s `.cover-modal` backdrop, so its presence is the test —
//     without it, Backspace pressed to dismiss a "Discard changes?" prompt
//     would delete the selection underneath it instead.

import { useEffect, useRef } from "react";
import { dispatchGlobalShortcut, type GlobalShortcutOptions } from "./globalShortcut";
export type { GlobalShortcutOptions } from "./globalShortcut";

/// Run `onKey` for keystrokes that are not meant for a text field, and (unless
/// `insideModal`) not meant for an open modal.
///
/// `onKey` is read through a ref, so callers can pass an inline arrow without
/// making the listener re-bind on every render. That is not a nicety: the same
/// dependency mistake in `useMissingFiles` produced a render loop that showed
/// as a button spinning forever, and a re-binding key listener fails in a
/// quieter, harder-to-find way.
export function useGlobalShortcut(
  onKey: (ev: KeyboardEvent) => void,
  { enabled = true, insideModal = false }: GlobalShortcutOptions = {},
) {
  const handler = useRef(onKey);
  handler.current = onKey;
  // Read through a ref for the same reason as the handler: an options object
  // written inline is a new object every render, and depending on it would
  // re-bind the listener each time.
  const state = useRef({ enabled, insideModal });
  state.current = { enabled, insideModal };

  useEffect(() => {
    const listener = (ev: KeyboardEvent) => {
      dispatchGlobalShortcut(ev, handler.current, state.current, () =>
        document.querySelector(".cover-modal") != null,
      );
    };
    document.addEventListener("keydown", listener);
    return () => document.removeEventListener("keydown", listener);
  }, []);
}

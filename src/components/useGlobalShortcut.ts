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

/// Run `onKey` for keystrokes that are not meant for a text field or a modal.
///
/// `onKey` is read through a ref, so callers can pass an inline arrow without
/// making the listener re-bind on every render. That is not a nicety: the same
/// dependency mistake in `useMissingFiles` produced a render loop that showed
/// as a button spinning forever, and a re-binding key listener fails in a
/// quieter, harder-to-find way.
export function useGlobalShortcut(onKey: (ev: KeyboardEvent) => void) {
  const handler = useRef(onKey);
  handler.current = onKey;

  useEffect(() => {
    const listener = (ev: KeyboardEvent) => {
      if (!ev || !ev.target) return;
      const target = ev.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }
      if (document.querySelector(".cover-modal")) return;
      handler.current(ev);
    };
    document.addEventListener("keydown", listener);
    return () => document.removeEventListener("keydown", listener);
  }, []);
}

// Keyboard navigation inside the extended-tags pop-in: Up/Down to move the
// highlight, Delete/Backspace to remove the highlighted tag.
//
// Its own file rather than another block inside `ExtendedTagsModal`, which
// already has more than enough to describe in one sentence (a local draft, a
// tag picker, a fetched list of addable keys, and the rows themselves).
//
// The bindings deliberately match the results table's — same keys, same
// meaning, same "start at the top going down, at the bottom going up" when
// nothing is highlighted yet. A pop-in that behaves differently from the list
// behind it is a pop-in you have to re-learn.

import { useGlobalShortcut } from "./useGlobalShortcut";

export interface UseExtendedTagsKeyboardArgs {
  /// The rows currently on screen, in display order.
  keys: string[];
  selectedKey: string | null;
  onSelect: (key: string) => void;
  /// Removes the highlighted row — the same action the "−" button performs,
  /// deliberately shared rather than reimplemented, so a keystroke and a
  /// click cannot ever mean two different things.
  onRemove: () => void;
  /// False while the pop-in is closed, while a row is being edited, or while
  /// the "+" list is open. The last one matters: with a list of tags on
  /// screen, Up/Down reads as "move through that list", so acting on the rows
  /// behind it would be the wrong answer to an unambiguous gesture.
  enabled: boolean;
}

export function useExtendedTagsKeyboard({
  keys,
  selectedKey,
  onSelect,
  onRemove,
  enabled,
}: UseExtendedTagsKeyboardArgs) {
  useGlobalShortcut(
    (ev) => {
      if (ev.key === "Delete" || ev.key === "Backspace") {
        if (selectedKey == null) return;
        ev.preventDefault();
        onRemove();
        return;
      }
      if (ev.key !== "ArrowDown" && ev.key !== "ArrowUp") return;
      if (keys.length === 0) return;
      ev.preventDefault();

      const down = ev.key === "ArrowDown";
      const from = selectedKey == null ? -1 : keys.indexOf(selectedKey);
      const next =
        from === -1
          ? down
            ? 0
            : keys.length - 1
          : Math.min(keys.length - 1, Math.max(0, from + (down ? 1 : -1)));
      const key = keys[next];
      if (key == null || key === selectedKey) return;
      onSelect(key);

      // Keep the row visible: the list scrolls on its own, and a highlight
      // that walks off the bottom edge is a highlight the user has lost track
      // of. `block: "nearest"` only scrolls when the row is actually out of
      // view, so holding a key moves the list one row at a time instead of
      // recentring on every step.
      document
        .querySelector(`.ext-row[data-key="${CSS.escape(key)}"]`)
        ?.scrollIntoView({ block: "nearest" });
    },
    { enabled, insideModal: true },
  );
}

// One row of the extended-tags pop-in: the tag's raw key, its value, and a
// copy button. Clicking anywhere on the row selects it — selection is what
// the pop-in's grouped +/− buttons act on (see ExtendedTagsModal), mirroring
// Mp3tag's own extended-tags list rather than a per-row remove button.
//
// Clicking the *value* opens it for editing straight away. It used to take
// two well-spaced clicks, borrowed from the inline file-rename field, but the
// two situations are not alike: renaming happens in a table where a click is
// mostly for selecting and an accidental rename is disruptive, whereas this
// pop-in exists to edit and its whole session is discarded by Cancel.
//
// The value is not selectable text (deliberately — the row is a click
// target), so the copy button is the only way to get a value out. That makes
// it part of the row's job rather than a convenience: without it, something
// like an AcoustID fingerprint can be read on screen and nowhere else.

import { useEffect, useRef, useState } from "react";
import { Copy } from "lucide-react";

import { IconButton } from "./IconButton";
import type { ExtendedRow } from "./tagSelection";

export interface ExtendedTagRowProps {
  row: ExtendedRow;
  selected: boolean;
  editing: boolean;
  onSelect: () => void;
  onStartEdit: () => void;
  onSubmit: (value: string) => void;
  onCancelEdit: () => void;
  /// Copies this row's value. The row reports the intent; the pop-in owns the
  /// clipboard call and the toast, so the feedback matches every other
  /// confirmation in the app.
  onCopy: () => void;
}

export function ExtendedTagRow({
  row,
  selected,
  editing,
  onSelect,
  onStartEdit,
  onSubmit,
  onCancelEdit,
  onCopy,
}: ExtendedTagRowProps) {
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!editing) return;
    setDraft(row.mixed ? "" : row.value);
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [editing, row.value, row.mixed]);

  return (
    <div className={selected ? "ext-row selected" : "ext-row"} onClick={onSelect}>
      <div className="ext-key">{row.key}</div>
      {editing ? (
        <input
          ref={inputRef}
          type="text"
          className="ext-value-input"
          value={draft}
          placeholder={row.mixed ? "Multiple values" : ""}
          onChange={(ev) => setDraft(ev.target.value)}
          onKeyDown={(ev) => {
            if (ev.key === "Escape") {
              ev.preventDefault();
              onCancelEdit();
            } else if (ev.key === "Enter") {
              ev.preventDefault();
              onSubmit(draft);
            }
          }}
          onBlur={() => onSubmit(draft)}
        />
      ) : (
        <div
          className={row.mixed ? "ext-value ext-mixed" : "ext-value"}
          onClick={onStartEdit}
        >
          {row.mixed ? "Multiple values" : row.value}
        </div>
      )}
      {/* Outside the editing branch on purpose: while a row is being edited
          the text is selectable in the input, so the button has nothing left
          to offer and would only compete with Enter/Escape. `stopPropagation`
          keeps the click from also selecting the row underneath. */}
      {!editing && (
        <span className="ext-copy" onClick={(ev) => ev.stopPropagation()}>
          <IconButton
            icon={<Copy size={13} strokeWidth={1.7} />}
            title={row.mixed ? "The selected files disagree — nothing to copy" : `Copy ${row.key}`}
            disabled={row.mixed}
            onClick={onCopy}
          />
        </span>
      )}
    </div>
  );
}

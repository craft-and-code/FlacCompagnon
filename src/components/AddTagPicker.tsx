// The extended-tags pop-in's "+" list: the curated set of common tags for
// the selection's format (fetched from the backend — lofty has no
// full-enumeration API, so the list is hand-picked on the Rust side).
// Rendered inline by ExtendedTagsModal (not a floating popover) — nothing
// here decides its own positioning or when it closes.
//
// No free-text "custom key" option: lofty's generic `Tag` API can only write
// one of its own known `ItemKey`s, not an arbitrary made-up frame the way
// Mp3tag's TXXX editor can — a free-text field would silently do nothing for
// any name that isn't already one of those known keys, which is worse than
// not offering it. Picking an item only ever chooses a *key* — ExtendedTagsModal
// opens the resulting row in edit mode for the value, exactly like a
// freshly-started rename.

import { X } from "lucide-react";

import { IconButton } from "./IconButton";
import type { AddableTag } from "../types";
import "./AddTagPicker.css";

export interface AddTagPickerProps {
  addable: AddableTag[];
  loading: boolean;
  /// Keys already shown in the list — filtered out of the curated options so
  /// there's never a confusing second "add" for something you'd edit in
  /// place instead.
  existingKeys: Set<string>;
  onPick: (key: string, label: string) => void;
  /// Dismisses the list without adding anything. The pop-in also closes it
  /// from the "+" button and from Escape, but neither is discoverable from
  /// inside the list itself — and a list with no visible way out reads as a
  /// step you are committed to.
  onClose: () => void;
}

export function AddTagPicker({
  addable,
  loading,
  existingKeys,
  onPick,
  onClose,
}: AddTagPickerProps) {
  const options = addable.filter((t) => !existingKeys.has(t.key));

  return (
    <div className="add-tag-picker">
      <div className="add-tag-head">
        <span className="add-tag-title">Add a tag</span>
        <IconButton
          icon={<X size={13} strokeWidth={1.8} />}
          title="Close without adding"
          onClick={onClose}
        />
      </div>
      {loading ? (
        <p className="add-tag-status">Loading…</p>
      ) : options.length === 0 ? (
        <p className="add-tag-status">No more common tags to add.</p>
      ) : (
        <ul className="add-tag-list">
          {options.map((t) => (
            <li key={t.key}>
              <button type="button" onClick={() => onPick(t.key, t.label)}>
                {t.label}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

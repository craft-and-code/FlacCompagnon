// Extended tags pop-in: every tag lofty read that isn't one of the tag
// panel's own fields (ISRC, BPM, ReplayGain, custom frames, ...). Laid out
// like Mp3tag's own extended-tags editor: click a row to select it, a
// grouped +/− acts on the selection, and Cancel/OK sit on that same bottom
// bar next to the format label.
//
// Editing here never touches disk directly: this pop-in keeps its own local
// draft (seeded from whatever the panel already has staged) and only its own
// OK button merges that draft up into the panel's edit buffer — the same one
// `TagFields`' boxes write into — via `onSave`. It's labeled "OK", not
// "Save", specifically so it doesn't read as writing to disk: the panel's own
// Save is still the only thing that ever does that. Cancel (or the backdrop,
// or Escape — all wired to `onClose`) discards the local draft since it's
// component state that gets reseeded fresh on every open.

import { useCallback, useEffect, useMemo, useState } from "react";
import { Minus, Plus } from "lucide-react";

import type { AddableTag, FieldEdit } from "../types";
import * as api from "../api";
import { copyText } from "../clipboard";
import { AddTagPicker } from "./AddTagPicker";
import { ExtendedTagRow } from "./ExtendedTagRow";
import { IconButton } from "./IconButton";
import { Modal } from "./Modal";
import { applyExtraEdits, type ExtendedRow } from "./tagSelection";
import "./ExtendedTagsModal.css";

export type { ExtendedRow };

export interface ExtendedTagsModalProps {
  open: boolean;
  /// The selection's extended tags before any edit — from `tagSelection`'s
  /// `extendedRows`, same as before this pop-in could edit anything.
  rows: ExtendedRow[];
  /// Whatever the panel already has staged for extra tags, so reopening this
  /// pop-in after a Save-and-close still shows it.
  pendingEdits: Record<string, FieldEdit>;
  selectedPaths: string[];
  /// Every file format in the selection — shown next to the +/− group, same
  /// as Mp3tag shows the container type next to its own extended tags.
  formats: string[];
  onClose: () => void;
  /// Merges this pop-in's local draft into the panel's edit buffer. Nothing
  /// reaches disk here — the panel's own Save does that, same as any other
  /// staged edit.
  onSave: (patch: Record<string, FieldEdit>) => void;
  onToast: (msg: string, kind?: "info" | "error") => void;
}

export function ExtendedTagsModal({
  open,
  rows,
  pendingEdits,
  selectedPaths,
  formats,
  onClose,
  onSave,
  onToast,
}: ExtendedTagsModalProps) {
  const [draft, setDraft] = useState<Record<string, FieldEdit>>({});
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  // A tag just chosen from the "+" picker, not yet given a value — rendered
  // as one extra row, already selected and in edit mode, but not staged into
  // `draft` until it's actually submitted with text (see `submitEdit`).
  const [newRow, setNewRow] = useState<{ key: string } | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [addable, setAddable] = useState<AddableTag[]>([]);
  const [addableLoading, setAddableLoading] = useState(false);

  // Opening starts a fresh local session from whatever the panel already has
  // staged — leftover selection/editing/new-row/picker state from a previous
  // open must not leak into this one.
  useEffect(() => {
    if (!open) return;
    setDraft(pendingEdits);
    setSelectedKey(null);
    setEditingKey(null);
    setNewRow(null);
    setPickerOpen(false);
    // Deliberately keyed on `open` alone, same as LookupModal's own prefill
    // effect — `pendingEdits` is read once at open time, so staging more
    // edits in the panel behind an open pop-in can't retrigger a reseed.
  }, [open]);

  // Escape closes the "+" list before it closes the pop-in. Bound in the
  // capture phase so it runs before Modal's own document-level listener, and
  // `stopPropagation` there keeps the event from ever reaching it — otherwise
  // one Escape would dismiss both, throwing away the whole session because
  // the user wanted to back out of a list.
  useEffect(() => {
    if (!open || !pickerOpen) return;
    const onKeyDown = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      ev.stopPropagation();
      setPickerOpen(false);
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, [open, pickerOpen]);

  const copyValue = useCallback(
    async (key: string, value: string) => {
      const ok = await copyText(value);
      onToast(ok ? `Copied ${key}.` : "Could not reach the clipboard.", ok ? "info" : "error");
    },
    [onToast],
  );

  const representativePath = selectedPaths[0] ?? null;
  useEffect(() => {
    if (!open || !representativePath) return;
    setAddableLoading(true);
    api
      .listAddableTags(representativePath)
      .then(setAddable)
      .catch((e) => {
        setAddable([]);
        // Surfaced rather than swallowed: a silent empty list here reads as
        // "the + button does nothing," which is a much harder bug to spot
        // than an error toast pointing at the actual cause.
        onToast(String(e), "error");
      })
      .finally(() => setAddableLoading(false));
  }, [open, representativePath, onToast]);

  const effectiveRows = useMemo(() => applyExtraEdits(rows, draft), [rows, draft]);
  const displayRows = useMemo(
    () => (newRow ? [...effectiveRows, { key: newRow.key, value: "", mixed: false }] : effectiveRows),
    [effectiveRows, newRow],
  );

  const submitEdit = useCallback(
    (key: string, value: string) => {
      const trimmed = value.trim();
      const isNew = newRow?.key === key;
      if (isNew) setNewRow(null);
      else setEditingKey(null);
      // A new row blurred/submitted without ever being typed into is
      // discarded rather than staged as an empty `Set` — it was never really
      // "added", just previewed.
      if (isNew && !trimmed) return;
      setDraft((prev) => ({ ...prev, [key]: trimmed === "" ? "Clear" : { Set: trimmed } }));
    },
    [newRow],
  );

  const cancelEdit = useCallback(
    (key: string) => {
      if (newRow?.key === key) setNewRow(null);
      else setEditingKey(null);
    },
    [newRow],
  );

  // The pop-in's "−": removes whatever row is currently selected. No
  // confirmation dialog — like Mp3tag's own grouped +/−, this only ever
  // touches the local draft, and Cancel discards the whole session in one
  // click if it was a mistake.
  const removeSelected = useCallback(() => {
    if (!selectedKey) return;
    if (newRow?.key === selectedKey) setNewRow(null);
    else setDraft((prev) => ({ ...prev, [selectedKey]: "Clear" }));
    if (editingKey === selectedKey) setEditingKey(null);
    setSelectedKey(null);
  }, [selectedKey, newRow, editingKey]);

  const formatLabel =
    formats.length === 0
      ? null
      : formats.length === 1
        ? formats[0]
        : `Mixed formats (${formats.join(", ")})`;

  return (
    <Modal open={open} onClose={onClose} innerClassName="modal-card extended-tags-inner" title="Extended tags">
      <div className="extended-tags-body">
        {displayRows.length === 0 ? (
          <p className="extended-tags-empty">No extended tags.</p>
        ) : (
          displayRows.map((r) => (
            <ExtendedTagRow
              key={r.key}
              row={r}
              selected={selectedKey === r.key}
              editing={editingKey === r.key || newRow?.key === r.key}
              onSelect={() => setSelectedKey(r.key)}
              onStartEdit={() => setEditingKey(r.key)}
              onSubmit={(value) => submitEdit(r.key, value)}
              onCancelEdit={() => cancelEdit(r.key)}
              onCopy={() => void copyValue(r.key, r.value)}
            />
          ))
        )}
      </div>

      {/* Inline, not a floating/anchored popover: no "click outside to
          close" listener to race against the click that opened it (that
          race is what made the list flash and disappear), and nothing that
          can end up visually clipped by an ancestor's own layout.
          Backing out therefore has to be explicit, and all three ways are
          offered — the list's own ✕, the + button again, or Escape — because
          the one that seems obvious depends entirely on who is looking. */}
      {pickerOpen && (
        <AddTagPicker
          addable={addable}
          loading={addableLoading}
          existingKeys={new Set(effectiveRows.map((r) => r.key))}
          onClose={() => setPickerOpen(false)}
          onPick={(key) => {
            setPickerOpen(false);
            setNewRow({ key });
            setSelectedKey(key);
          }}
        />
      )}

      <div className="extended-tags-footer">
        <div className="ext-plusminus">
          <IconButton
            icon={<Plus size={13} strokeWidth={1.8} />}
            title={pickerOpen ? "Close the tag list" : "Add a tag"}
            onClick={() => setPickerOpen((v) => !v)}
          />
          <IconButton
            icon={<Minus size={13} strokeWidth={1.8} />}
            title="Remove the selected tag"
            disabled={selectedKey == null}
            onClick={removeSelected}
          />
        </div>
        {formatLabel && <span className="extended-tags-format">{formatLabel}</span>}
        <div className="extended-tags-footer-spacer" />
        <button className="btn btn-ghost" type="button" onClick={onClose}>
          Cancel
        </button>
        <button
          className="btn"
          type="button"
          onClick={() => {
            onSave(draft);
            onClose();
          }}
        >
          OK
        </button>
      </div>
    </Modal>
  );
}

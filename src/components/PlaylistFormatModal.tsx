// Playlist format chooser, shown before the native save dialog: the OS dialog
// can't host the Simple/Extended choice itself, so it's asked here first.

import { useState } from "react";

import type { PlaylistFormat } from "../types";
import { Modal } from "./Modal";

export interface PlaylistFormatModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: (format: PlaylistFormat) => void;
}

const OPTIONS: { value: PlaylistFormat; label: string; hint: string }[] = [
  {
    value: "Extended",
    label: "Extended M3U",
    hint: "Includes track titles and durations — recommended.",
  },
  {
    value: "Simple",
    label: "Simple M3U",
    hint: "File paths only — understood by every player.",
  },
];

export function PlaylistFormatModal({ open, onClose, onConfirm }: PlaylistFormatModalProps) {
  const [format, setFormat] = useState<PlaylistFormat>("Extended");

  return (
    <Modal
      open={open}
      onClose={onClose}
      innerClassName="extended-tags-inner playlist-format-inner"
      title="Export playlist"
    >
      <div className="playlist-format-options">
        {OPTIONS.map((opt) => (
          <label
            className={format === opt.value ? "playlist-format-option selected" : "playlist-format-option"}
            key={opt.value}
          >
            <input
              type="radio"
              name="playlist-format"
              value={opt.value}
              checked={format === opt.value}
              onChange={() => setFormat(opt.value)}
            />
            <span>
              <strong>{opt.label}</strong>
              <small className="muted">{opt.hint}</small>
            </span>
          </label>
        ))}
      </div>
      <div className="playlist-format-actions">
        <button className="btn btn-ghost" type="button" onClick={onClose}>
          Cancel
        </button>
        <button className="btn" type="button" onClick={() => onConfirm(format)}>
          Export…
        </button>
      </div>
    </Modal>
  );
}

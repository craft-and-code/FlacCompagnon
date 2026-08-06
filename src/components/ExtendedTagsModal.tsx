// Extended tags pop-in: every tag lofty read that isn't one of the tag panel's
// own fields (ISRC, BPM, ReplayGain, custom frames, ...). Read-only, and kept
// out of the main field list so it doesn't crowd the fields people actually
// edit.

import { Modal } from "./Modal";

export interface ExtendedRow {
  key: string;
  value: string;
  /// The selection disagrees on this key — no single value to show.
  mixed: boolean;
}

export interface ExtendedTagsModalProps {
  open: boolean;
  rows: ExtendedRow[];
  onClose: () => void;
}

export function ExtendedTagsModal({ open, rows, onClose }: ExtendedTagsModalProps) {
  return (
    <Modal open={open} onClose={onClose} innerClassName="extended-tags-inner" title="Extended tags">
      <div className="extended-tags-body">
        {rows.length === 0 ? (
          <p className="extended-tags-empty">No extended tags.</p>
        ) : (
          rows.map((r) => (
            <div className="ext-row" key={r.key}>
              <div className="ext-key">{r.key}</div>
              <div className={r.mixed ? "ext-value ext-mixed" : "ext-value"}>
                {r.mixed ? "Multiple values" : r.value}
              </div>
            </div>
          ))
        )}
      </div>
    </Modal>
  );
}

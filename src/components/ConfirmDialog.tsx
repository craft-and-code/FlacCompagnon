// Generic yes/no confirmation pop-in.
//
// Used for anything destructive or data-losing enough to deserve a
// deliberate second click rather than firing on the first one: deleting a
// cover, discarding unsaved tag edits by switching the selection.

import { Modal } from "./Modal";

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  /// Renders the confirm button in the destructive (red) style instead of
  /// the default accent one — set for anything that can't be undone.
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = "Confirm",
  danger,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <Modal open={open} onClose={onCancel} innerClassName="confirm-dialog-inner" title={title}>
      <p className="confirm-dialog-message">{message}</p>
      <div className="confirm-dialog-actions">
        <button className="btn btn-ghost" type="button" onClick={onCancel}>
          Cancel
        </button>
        <button
          className={danger ? "btn btn-danger" : "btn"}
          type="button"
          onClick={onConfirm}
        >
          {confirmLabel}
        </button>
      </div>
    </Modal>
  );
}

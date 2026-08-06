// Progress bar for analysis and spectrogram rendering, plus the overlay that
// refuses drops while one is running.

import { X } from "lucide-react";

import { IconButton } from "./IconButton";

export interface ProgressProps {
  label: string;
  /// 0-100. Stays at 0 until the backend reports its first file.
  percent: number;
  onCancel: () => void;
  cancelDisabled: boolean;
}

export function Progress({ label, percent, onCancel, cancelDisabled }: ProgressProps) {
  return (
    <section className="progress">
      <div className="progress-label">
        <span id="progress-text">{label}</span>
        <IconButton
          icon={<X size={14} strokeWidth={1.8} />}
          title="Cancel"
          variant="close"
          disabled={cancelDisabled}
          onClick={onCancel}
        />
      </div>
      <div className="progress-track">
        <div className="progress-bar" style={{ width: `${percent}%` }} />
      </div>
    </section>
  );
}

/// Shown instead of the drop highlight when files are dragged over the window
/// mid-analysis — dropping is refused, and silently ignoring it would look
/// like a bug.
export function DropGuard() {
  return (
    <div className="drop-guard">
      <div className="drop-guard-box">
        <span className="spinner" />
        <p>Analysis in progress — please wait before dropping files</p>
      </div>
    </div>
  );
}

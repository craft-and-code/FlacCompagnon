// The conversion panel: imports audio files/folders independently of the
// main results table and re-encodes them to another format. Mirrors
// TagPanel's shape on purpose (same width/radius/close button, a drop target
// where the cover sits) since the two are meant to read as siblings — one
// panel per side of the results table.

import { X } from "lucide-react";

import type { ConvertFormat, ConvertSource, FlacEffort } from "../types";
import { baseName } from "../format";
import "./ConvertPanel.css";
import { ConvertDropzone } from "./ConvertDropzone";
import { ConvertSettingsFields } from "./ConvertSettingsFields";
import { IconButton } from "./IconButton";

export interface ConvertPanelProps {
  /// Imported files. Only `path` is used here — `base` exists so the batch
  /// can reproduce each file's folder at the destination (see types.ts).
  targets: ConvertSource[];
  selected: Set<string>;
  format: ConvertFormat;
  bitrateKbps: number | null;
  flacEffort: FlacEffort;
  preserveModtime: boolean;
  copyOthers: boolean;
  /// A dropped folder is being expanded — see ConvertDropzone's own prop.
  importing: boolean;
  busy: boolean;
  cancelling: boolean;
  progressLabel: string;
  dragOver: boolean;
  /// How many rows are selected in the *main* results table right now —
  /// entirely separate from `selected` above (this panel's own list), but
  /// surfaced here so "Add selected" has something to show a count for.
  mainSelectionCount: number;
  onClose: () => void;
  onSetFormat: (f: ConvertFormat) => void;
  onSetBitrateKbps: (kbps: number | null) => void;
  onSetFlacEffort: (e: FlacEffort) => void;
  onSetPreserveModtime: (v: boolean) => void;
  onSetCopyOthers: (v: boolean) => void;
  onRemoveTarget: (path: string) => void;
  onClearTargets: () => void;
  onToggleSelected: (path: string) => void;
  /// Imports the main table's current selection into this panel's own list —
  /// the only bridge between the two, and only on explicit click: selecting
  /// rows in the table never imports them by itself, since that table
  /// selection drives other things too (tag editing, playback, delete).
  onAddSelected: () => void;
  onConvert: () => void;
  onCancel: () => void;
}

export function ConvertPanel({
  targets,
  selected,
  format,
  bitrateKbps,
  flacEffort,
  preserveModtime,
  copyOthers,
  importing,
  busy,
  cancelling,
  progressLabel,
  dragOver,
  mainSelectionCount,
  onClose,
  onSetFormat,
  onSetBitrateKbps,
  onSetFlacEffort,
  onSetPreserveModtime,
  onSetCopyOthers,
  onRemoveTarget,
  onClearTargets,
  onToggleSelected,
  onAddSelected,
  onConvert,
  onCancel,
}: ConvertPanelProps) {
  // No count in the label itself — the count is shown once, in the info bar
  // under the drop zone (see .convert-info-bar below), not duplicated here.
  const convertLabel = selected.size > 0 ? "Convert selected" : "Convert";
  const convertDisabled = targets.length === 0;
  const infoText =
    targets.length === 0
      ? "No tracks imported yet"
      : selected.size > 0
        ? `${selected.size} of ${targets.length} selected`
        : `${targets.length} track${targets.length === 1 ? "" : "s"} imported`;

  return (
    <aside className="convert-panel">
      <div className="convert-panel-head">
        <h2>Convert</h2>
        <IconButton
          icon={<X size={14} strokeWidth={1.8} />}
          title="Close"
          variant="close"
          className="convert-panel-close"
          onClick={onClose}
        />
      </div>

      <ConvertDropzone
        dragOver={dragOver}
        importing={importing}
        busy={busy}
        progressLabel={progressLabel}
        itemCount={targets.length}
      />

      {/* Same slot/styling as TagPanel's .tag-cover-info band under the
          cover — carries the imported/selected count that used to live in
          the Convert button's own label, plus the only way to pull the main
          table's selection into this panel without a drag. */}
      <div className="convert-info-bar">
        <span>{infoText}</span>
        {mainSelectionCount > 0 && (
          <button type="button" className="convert-info-action" onClick={onAddSelected}>
            Add {mainSelectionCount} selected
          </button>
        )}
      </div>

      <div className="convert-panel-body">
        <ConvertSettingsFields
          format={format}
          bitrateKbps={bitrateKbps}
          flacEffort={flacEffort}
          preserveModtime={preserveModtime}
          copyOthers={copyOthers}
          busy={busy}
          onSetFormat={onSetFormat}
          onSetBitrateKbps={onSetBitrateKbps}
          onSetFlacEffort={onSetFlacEffort}
          onSetPreserveModtime={onSetPreserveModtime}
          onSetCopyOthers={onSetCopyOthers}
        />

        {targets.length > 0 && (
          <ul className="convert-item-list">
            {targets.map(({ path }) => (
              <li
                key={path}
                className={selected.has(path) ? "convert-item selected" : "convert-item"}
                onClick={() => onToggleSelected(path)}
              >
                <span className="convert-item-name" title={path}>
                  {baseName(path)}
                </span>
                <IconButton
                  icon={<X size={12} strokeWidth={2} />}
                  title="Remove from the list"
                  variant="close"
                  onClick={(ev) => {
                    ev.stopPropagation();
                    onRemoveTarget(path);
                  }}
                />
              </li>
            ))}
          </ul>
        )}

        {/* A link rather than a second button beside Convert: emptying the
            list is a correction, not an action of the same weight as running
            the batch, and pairing them made the footer read as a choice
            between two equals. Same `.link-btn` treatment as the tag panel's
            "Extended tags" on the other side of the table. Below three
            entries it isn't worth the row — removing them one by one with
            each line's own × is quicker than reading a new control. */}
        {targets.length > 2 && (
          <button className="link-btn" type="button" onClick={onClearTargets}>
            Clear the list <span className="link-btn-count">({targets.length})</span>
          </button>
        )}
      </div>

      <div className="convert-panel-actions">
        {busy ? (
          <button className="btn btn-ghost" disabled={cancelling} onClick={onCancel}>
            {cancelling ? "Cancelling…" : "Cancel"}
          </button>
        ) : (
          <button
            className="btn"
            disabled={convertDisabled}
            title={convertDisabled ? "Drop audio files or folders above first" : undefined}
            onClick={onConvert}
          >
            {convertLabel}
          </button>
        )}
      </div>
    </aside>
  );
}

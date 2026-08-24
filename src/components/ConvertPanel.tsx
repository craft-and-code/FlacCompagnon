// The conversion panel: imports audio files/folders independently of the
// main results table and re-encodes them to another format. Mirrors
// TagPanel's shape on purpose (same width/radius/close button, a drop target
// where the cover sits) since the two are meant to read as siblings — one
// panel per side of the results table.

import { X } from "lucide-react";

import type { ConvertFormat } from "../types";
import { baseName } from "../format";
import "./ConvertPanel.css";
import { ConvertDropzone } from "./ConvertDropzone";
import { IconButton } from "./IconButton";

const OPUS_BITRATES = [96, 128, 160, 192, 256];
const MP3_BITRATES = [128, 192, 256, 320];

const FORMAT_LABELS: Record<ConvertFormat, string> = {
  flac: "FLAC (lossless)",
  opus: "Opus (lossy, recommended)",
  mp3: "MP3 (lossy, compatible)",
  wav: "WAV (16-bit PCM)",
};

/// One note per format, shown under the picker — always, not on hover.
///
/// Choosing the format is the only decision in this panel with lasting
/// consequences: it decides whether the copy can ever be turned back into the
/// original. That is exactly the sort of thing a tooltip hides from anyone who
/// doesn't already know to look for it, so it stays on screen.
///
/// Each note answers the same two questions in the same order — *what does
/// this cost me* and *when would I pick it* — so the four can be compared by
/// reading the same position in each, rather than four differently-shaped
/// paragraphs. The FLAC and WAV ones also carry what used to be their only
/// note ("no bitrate to choose"), since the bitrate select disappears for
/// them and its absence otherwise looks like a missing control.
const FORMAT_NOTES: Record<ConvertFormat, string> = {
  flac: "Keeps every bit of the original, at approximately half its size. No bitrate to choose — the size follows the music. Pick it to archive, or whenever the copy may itself become a source.",
  opus: "Discards what you are unlikely to hear, for roughly a fifth of the size. The best quality per megabyte of the four, but younger, so some older players and car stereos don't read it. Pick it for a phone or a music player that supports it.",
  mp3: "Also discards detail, and less efficiently than Opus at the same bitrate — its advantage is that everything made in the last twenty-five years can play it. Pick it when compatibility matters more than size.",
  wav: "No compression at all: the largest files of the four, fixed at 16-bit whatever the source's own depth. Pick it only for a tool that refuses everything else — for keeping the audio intact, FLAC does the same job far smaller.",
};

export interface ConvertPanelProps {
  targets: string[];
  selected: Set<string>;
  format: ConvertFormat;
  bitrateKbps: number | null;
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
  onSetCopyOthers,
  onRemoveTarget,
  onClearTargets,
  onToggleSelected,
  onAddSelected,
  onConvert,
  onCancel,
}: ConvertPanelProps) {
  const bitratePresets = format === "opus" ? OPUS_BITRATES : format === "mp3" ? MP3_BITRATES : null;

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
        <div className="convert-settings">
          <label className="convert-field">
            <span>Format</span>
            <select
              value={format}
              disabled={busy}
              onChange={(ev) => onSetFormat(ev.target.value as ConvertFormat)}
            >
              {(Object.keys(FORMAT_LABELS) as ConvertFormat[]).map((f) => (
                <option value={f} key={f}>
                  {FORMAT_LABELS[f]}
                </option>
              ))}
            </select>
          </label>

          {bitratePresets && (
            <label className="convert-field">
              <span>Bitrate</span>
              <select
                value={bitrateKbps ?? "auto"}
                disabled={busy}
                onChange={(ev) =>
                  onSetBitrateKbps(ev.target.value === "auto" ? null : Number(ev.target.value))
                }
              >
                <option value="auto">Auto (recommended)</option>
                {bitratePresets.map((kbps) => (
                  <option value={kbps} key={kbps}>
                    {kbps} kbps
                  </option>
                ))}
              </select>
            </label>
          )}

          {/* Unconditional: every format has a note now, so the guard that
              used to be here would always pass and only suggest otherwise. */}
          <p className="convert-field-note">{FORMAT_NOTES[format]}</p>

          <label className="convert-checkbox" title="Covers, playlists, spectrograms, and any other non-audio file">
            <input
              type="checkbox"
              checked={copyOthers}
              disabled={busy}
              onChange={(ev) => onSetCopyOthers(ev.target.checked)}
            />
            <span>Also copy other files</span>
          </label>
        </div>

        {targets.length > 0 && (
          <ul className="convert-item-list">
            {targets.map((path) => (
              <li
                key={path}
                className={selected.has(path) ? "convert-item selected" : "convert-item"}
                onClick={() => onToggleSelected(path)}
              >
                <span className="convert-item-name">{baseName(path)}</span>
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

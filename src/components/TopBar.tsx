// The action bar. Purely presentational: every button reports upwards, and
// what's enabled is decided by the app state passed in.

import { useRef } from "react";
import {
  ArrowLeftRight,
  CheckSquare,
  ListMusic,
  FolderSearch,
  ListOrdered,
  RefreshCw,
  Repeat,
  Save,
  Search,
  Square,
  X,
} from "lucide-react";

import { IconButton } from "./IconButton";
import { useTheme } from "./useTheme";
import "./TopBar.css";

export interface TopBarProps {
  busy: boolean;
  hasReport: boolean;
  canGenerateSpectrograms: boolean;
  /// ffmpeg wasn't found — the spectrogram button explains why it's disabled
  /// rather than looking broken.
  ffmpegAvailable: boolean;
  /// Filters which rows the table *renders* — see ResultsTable's
  /// `visiblePaths`. Matches everything a row displays (format, bit depth,
  /// detections, ...), not just the file name — see `fileSearchText`. Never
  /// touches playback, selection, drag order or exports, which all keep
  /// working off the full list.
  searchQuery: string;
  onSearchChange: (query: string) => void;
  /// Size of the current selection — the renumber button only makes sense
  /// (and only enables) once there are at least two tracks to order.
  selectedCount: number;
  /// Selecting, deselecting or inverting can each discard an unsaved tag edit
  /// the same way clicking a different row does, so App.tsx routes all three
  /// through the same confirmation guard rather than calling the selection
  /// hook directly — the tag panel's own close button goes through the same
  /// guard as `onDeselectAll` too.
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onInvertSelection: () => void;
  renumberBusy: boolean;
  /// Opens the confirmation dialog; the actual write happens after the user
  /// confirms (see App.tsx) since it overwrites Track/Track Total tags.
  onRenumberTracks: () => void;
  onPick: () => void;
  onSave: () => void;
  onExportPlaylist: () => void;
  onGenerateSpectrograms: () => void;
  onReset: () => void;
  /// Opens the conversion panel (right-hand side, mirroring the tag panel).
  onOpenConvert: () => void;
  /// Re-checks that every listed file is still on disk. Separated from the
  /// buttons around it by a gap: it acts on the *listing* rather than on the
  /// audio, which is a different kind of action and shouldn't read as one
  /// more export.
  onRefreshPresence: () => void;
  /// True while that check is running.
  checkingPresence: boolean;
  /// How many listed files were not found. Drives whether the relocate button
  /// is offered at all — with nothing missing it has nothing to do.
  missingCount: number;
  /// Asks for a folder and repoints the missing rows at the files found there.
  onRelocateMissing: () => void;
  /// True when the listing came from a `.json` report, which is the only case
  /// where there is a file to write back over.
  canUpdateReport: boolean;
  onUpdateReport: () => void;
}

export function TopBar({
  busy,
  hasReport,
  canGenerateSpectrograms,
  ffmpegAvailable,
  searchQuery,
  onSearchChange,
  selectedCount,
  onSelectAll,
  onDeselectAll,
  onInvertSelection,
  renumberBusy,
  onRenumberTracks,
  onPick,
  onSave,
  onExportPlaylist,
  onGenerateSpectrograms,
  onReset,
  onOpenConvert,
  onRefreshPresence,
  checkingPresence,
  missingCount,
  onRelocateMissing,
  canUpdateReport,
  onUpdateReport,
}: TopBarProps) {
  const theme = useTheme();
  const searchInputRef = useRef<HTMLInputElement>(null);

  return (
    <header className="topbar">
      <div className="topbar-search">
        <Search className="topbar-search-icon" size={14} strokeWidth={1.8} aria-hidden="true" />
        <input
          ref={searchInputRef}
          type="text"
          placeholder="Filter…"
          value={searchQuery}
          disabled={!hasReport}
          onChange={(ev) => onSearchChange(ev.target.value)}
        />
        {/* Only rendered once there's something to clear, and even then only
            revealed on hover (see .topbar-search:hover in TopBar.css) — a
            permanently visible cross here read as a delete affordance, which
            this isn't: it just resets the display filter. Refocusing the
            field afterwards (rather than leaving focus on the now-vanished
            button) means typing a new query doesn't need an extra click. */}
        {searchQuery && (
          <IconButton
            icon={<X size={12} strokeWidth={2} />}
            title="Clear filter"
            variant="close"
            className="topbar-search-clear"
            onClick={() => {
              onSearchChange("");
              searchInputRef.current?.focus();
            }}
          />
        )}
      </div>
      {/* Sized to match .topbar-search's height rather than the standard
          20×20 .icon-btn box (see .topbar-toolbtn in TopBar.css) — the same
          fix the renumber button below already needed: ListOrdered's small
          "1 2 3" glyphs read as noise at the standard size, and once one
          button in this row was bumped up, leaving these two at the smaller
          size would look like an oversight rather than a choice. */}
      <div className="topbar-selection-actions">
        <IconButton
          icon={<CheckSquare size={17} strokeWidth={1.7} />}
          title="Select all tracks"
          className="topbar-toolbtn"
          disabled={busy || !hasReport}
          onClick={onSelectAll}
        />
        <IconButton
          icon={<Square size={17} strokeWidth={1.7} />}
          title="Deselect all"
          className="topbar-toolbtn"
          disabled={busy || selectedCount === 0}
          onClick={onDeselectAll}
        />
        <IconButton
          icon={<ArrowLeftRight size={16} strokeWidth={1.7} />}
          title="Invert selection"
          className="topbar-toolbtn"
          disabled={busy || !hasReport}
          onClick={onInvertSelection}
        />
      </div>
      <IconButton
        icon={<ListOrdered size={20} strokeWidth={1.6} />}
        title={
          selectedCount < 2
            ? "Select at least two tracks to renumber"
            : `Renumber ${selectedCount} selected tracks 1–${selectedCount}`
        }
        className="topbar-toolbtn"
        disabled={busy || renumberBusy || selectedCount < 2}
        onClick={onRenumberTracks}
      />
      <IconButton
        icon={<ListMusic size={19} strokeWidth={1.6} />}
        title="Export playlist…"
        className="topbar-toolbtn"
        disabled={busy || !hasReport}
        onClick={onExportPlaylist}
      />
      <IconButton
        icon={<Repeat size={17} strokeWidth={1.7} />}
        title="Convert files…"
        className="topbar-toolbtn"
        disabled={busy}
        onClick={onOpenConvert}
      />
      <span className="topbar-gap" />
      <IconButton
        icon={
          <RefreshCw
            size={16}
            strokeWidth={1.7}
            className={checkingPresence ? "topbar-spinning" : undefined}
          />
        }
        title="Check that every listed file is still on disk"
        className="topbar-toolbtn"
        disabled={busy || !hasReport || checkingPresence}
        onClick={onRefreshPresence}
      />
      {/* Both only make sense once something is actually wrong or reloadable,
          so they stay out of the way otherwise rather than sitting greyed out
          — the bar is already dense. */}
      {missingCount > 0 && (
        <IconButton
          icon={<FolderSearch size={16} strokeWidth={1.7} />}
          title={`Locate the ${missingCount} missing file${missingCount === 1 ? "" : "s"} in another folder`}
          className="topbar-toolbtn"
          disabled={busy}
          onClick={onRelocateMissing}
        />
      )}
      {canUpdateReport && (
        <IconButton
          icon={<Save size={16} strokeWidth={1.7} />}
          title="Write the current listing back over the report it was loaded from"
          className="topbar-toolbtn"
          disabled={busy}
          onClick={onUpdateReport}
        />
      )}
      <div className="actions">
        <button className="btn" disabled={busy} onClick={onPick}>
          Add titles…
        </button>
        <button className="btn btn-secondary" disabled={busy || !hasReport} onClick={onSave}>
          Save…
        </button>
        <button
          className="btn btn-secondary"
          disabled={busy || !canGenerateSpectrograms || !ffmpegAvailable}
          title={
            ffmpegAvailable
              ? ""
              : "ffmpeg was not found on your system — install it to enable spectrograms"
          }
          onClick={onGenerateSpectrograms}
        >
          Generate spectrograms
        </button>
        {!busy && hasReport && (
          <button className="btn btn-ghost" onClick={onReset}>
            Reset
          </button>
        )}
        <button
          className="btn btn-ghost"
          title="Switch theme (Auto / Light / Dark)"
          onClick={theme.cycle}
        >
          {theme.label}
        </button>
      </div>
    </header>
  );
}

// The action bar. Purely presentational: every button reports upwards, and
// what's enabled is decided by the app state passed in.

import { useTheme } from "./useTheme";

export interface TopBarProps {
  busy: boolean;
  hasReport: boolean;
  canGenerateSpectrograms: boolean;
  /// ffmpeg wasn't found — the spectrogram button explains why it's disabled
  /// rather than looking broken.
  ffmpegAvailable: boolean;
  onPick: () => void;
  onSave: () => void;
  onExportPlaylist: () => void;
  onGenerateSpectrograms: () => void;
  onReset: () => void;
}

export function TopBar({
  busy,
  hasReport,
  canGenerateSpectrograms,
  ffmpegAvailable,
  onPick,
  onSave,
  onExportPlaylist,
  onGenerateSpectrograms,
  onReset,
}: TopBarProps) {
  const theme = useTheme();

  return (
    <header className="topbar">
      <div className="actions">
        <button className="btn" disabled={busy} onClick={onPick}>
          Choose folder…
        </button>
        <button className="btn btn-secondary" disabled={busy || !hasReport} onClick={onSave}>
          Save…
        </button>
        <button
          className="btn btn-secondary"
          disabled={busy || !hasReport}
          onClick={onExportPlaylist}
        >
          Export playlist…
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

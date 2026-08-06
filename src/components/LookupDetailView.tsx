// Lookup pop-in, detail step: the chosen release's cover, metadata and track
// list. Picking a track is only offered for a single-file selection — with
// several files there's no way to know which track goes with which file, so
// only the album-level fields can be applied.

import type { LookupRelease } from "../types";
import { coverDataUrl } from "../format";

export interface LookupDetailViewProps {
  release: LookupRelease;
  selectionCount: number;
  /// The release was reached via a MusicBrainz ID already in the file's tags
  /// rather than a text search — worth saying, since the search step was
  /// skipped entirely.
  matchedByExistingId: boolean;
  selectedTrackIndex: number | null;
  onSelectTrack: (index: number | null) => void;
  onBack: () => void;
  onApply: () => void;
}

export function LookupDetailView({
  release,
  selectionCount,
  matchedByExistingId,
  selectedTrackIndex,
  onSelectTrack,
  onBack,
  onApply,
}: LookupDetailViewProps) {
  const singleFile = selectionCount === 1;
  const coverUrl = release.cover ? coverDataUrl(release.cover) : null;

  return (
    <div className="lookup-view">
      <button className="btn btn-ghost lookup-back-btn" type="button" onClick={onBack}>
        ‹ Back to results
      </button>

      <div className="lookup-detail-body">
        <div className="lookup-detail-head">
          {coverUrl ? (
            <img className="lookup-detail-cover" src={coverUrl} alt="" />
          ) : (
            <div className="lookup-detail-cover" />
          )}
          <div className="lookup-detail-meta">
            <p className="lookup-detail-title">{release.title}</p>
            <p className="lookup-detail-artist">
              {release.artist}
              {release.year ? ` · ${release.year}` : ""}
            </p>
          </div>
        </div>

        {matchedByExistingId && (
          <p className="lookup-note lookup-note-match">
            Matched via the MusicBrainz ID already in this file's tags — no search needed.
          </p>
        )}

        <p className="lookup-note">
          {singleFile
            ? "Click a track to also fill in its title and track number — or just apply the album info below."
            : `${selectionCount} files selected — only album, album artist, year and cover will be applied. Select a single file to also match track titles.`}
        </p>

        {release.tracks.length === 0 ? (
          <p className="lookup-note">No track list available.</p>
        ) : (
          <div className="lookup-tracklist">
            {release.tracks.map((t, i) => (
              <button
                type="button"
                className={selectedTrackIndex === i ? "lookup-track selected" : "lookup-track"}
                key={`${t.position}:${i}`}
                disabled={!singleFile}
                onClick={() => onSelectTrack(selectedTrackIndex === i ? null : i)}
              >
                <span className="lookup-track-pos">{t.position}</span>
                <span className="lookup-track-title">{t.title}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="lookup-detail-actions">
        <button className="btn" type="button" onClick={onApply}>
          Apply to tag panel
        </button>
      </div>
    </div>
  );
}

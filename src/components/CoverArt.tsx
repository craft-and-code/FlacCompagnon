// Cover art at the top of the tag panel: full width at its own aspect ratio
// (never cropped), with a banner underneath carrying the dimensions/format/
// size and a role picker — Mac tag editors like Meta caption artwork below it
// rather than above.
//
// A selection whose files don't share one cover shows chevrons to cycle
// through the distinct ones, auto-advancing every 3s, and disables the role
// picker: `CoverEdit` re-applies one image to the whole batch, so relabeling a
// mixed selection would overwrite every file's artwork, not just its role.

import { useEffect, useState } from "react";
import { ImageDown, Music, Trash2, Upload } from "lucide-react";

import type { CoverArt as CoverArtData } from "../types";
import { PICTURE_TYPE_LABELS, coverDataUrl, pictureTypeLabel } from "../format";
import { IconButton } from "./IconButton";
import { MarqueeText } from "./MarqueeText";

const CAROUSEL_MS = 3000;
// Larger than the results table's 14px thumbnail icon — this box has actual
// room, and a 14px glyph would look lost centered in it.
const PLACEHOLDER_ICON_SIZE = 44;

export interface CoverArtProps {
  covers: CoverArtData[];
  /// Highlighted while a file is being dragged over the box.
  dragOver: boolean;
  /// A dropped image is being read from disk — shows a spinner in place of
  /// the drag-upload icon so the box isn't silently unresponsive in between.
  loading: boolean;
  onOpenLightbox: (cover: CoverArtData) => void;
  onRoleChange: (cover: CoverArtData, pictureType: string) => void;
  /// Removes the cover from every selected file.
  onDelete: () => void;
  /// Writes the currently shown cover next to the audio file(s).
  onExtract: (cover: CoverArtData) => void;
}

function infoLine(cover: CoverArtData, multiple: boolean): string {
  const kb = Math.max(1, Math.round(cover.size_bytes / 1024));
  const kind = cover.mime.replace(/^image\//, "").toUpperCase() || "?";
  return `${cover.width}×${cover.height} · ${kind} · ${kb} KB${multiple ? " · multiple covers" : ""}`;
}

export function CoverArt({
  covers,
  dragOver,
  loading,
  onOpenLightbox,
  onRoleChange,
  onDelete,
  onExtract,
}: CoverArtProps) {
  const [index, setIndex] = useState(0);
  // Restarted from zero on every manual chevron click, so picking a cover
  // doesn't get immediately undone by the timer firing right after.
  const [timerEpoch, setTimerEpoch] = useState(0);
  const multiple = covers.length > 1;

  // The selection changed under us — a stale index would show the wrong cover
  // or none at all.
  useEffect(() => setIndex(0), [covers]);

  useEffect(() => {
    if (!multiple) return;
    const id = window.setInterval(() => setIndex((i) => (i + 1) % covers.length), CAROUSEL_MS);
    return () => window.clearInterval(id);
  }, [multiple, covers.length, timerEpoch]);

  const step = (delta: number) => {
    setIndex((i) => (i + delta + covers.length) % covers.length);
    setTimerEpoch((e) => e + 1);
  };

  const cover = covers[index] ?? null;
  const url = cover ? coverDataUrl(cover) : null;
  const frameClass = dragOver ? "tag-cover-frame drag-over" : "tag-cover-frame";

  const nav = multiple && (
    <>
      <button
        className="tag-cover-nav tag-cover-prev"
        type="button"
        title="Previous cover"
        onClick={() => step(-1)}
      >
        ‹
      </button>
      <button
        className="tag-cover-nav tag-cover-next"
        type="button"
        title="Next cover"
        onClick={() => step(1)}
      >
        ›
      </button>
    </>
  );

  // A role outside the curated list (e.g. lofty's `Undefined(n)`) gets its own
  // synthetic disabled option showing the raw value — rather than silently
  // pre-selecting "Front cover" and letting a change event turn an
  // obscure-but-real role into an actual front-cover overwrite.
  const knownRole = cover ? cover.picture_type in PICTURE_TYPE_LABELS : false;

  // One shared frame markup for both the "has a cover" and "no cover yet"
  // cases — the drag/loading overlay and the drop-target styling apply to
  // the box itself, not to whatever happens to be inside it.
  return (
    <div className="tag-cover">
      <div className={frameClass}>
        {cover && url ? (
          <img
            className="tag-cover-img"
            src={url}
            alt=""
            onClick={() => onOpenLightbox(cover)}
          />
        ) : (
          <span className="tag-cover-placeholder">
            <Music size={PLACEHOLDER_ICON_SIZE} strokeWidth={1.4} />
          </span>
        )}
        {nav}
        {(dragOver || loading) && (
          <div className="tag-cover-overlay">
            {loading ? <span className="spinner" /> : <ImageDown />}
          </div>
        )}
      </div>
      {cover && url && (
        <div className="tag-cover-info">
          <MarqueeText className="tag-cover-info-text" text={infoLine(cover, multiple)} />
          <div className="tag-cover-controls">
            <IconButton
              icon={<Upload size={14} strokeWidth={1.6} />}
              title="Extract this cover next to the audio file"
              onClick={() => onExtract(cover)}
            />
            <IconButton
              icon={<Trash2 size={14} strokeWidth={1.6} />}
              title="Delete this cover"
              variant="danger-persistent"
              onClick={onDelete}
            />
            <select
              className="tag-cover-role"
              value={cover.picture_type}
              disabled={multiple}
              title={
                multiple
                  ? "All selected files must share the exact same cover to change its role"
                  : "Change this cover's role"
              }
              onChange={(ev) => onRoleChange(cover, ev.target.value)}
            >
              {!knownRole && (
                <option value={cover.picture_type} disabled>
                  {pictureTypeLabel(cover.picture_type)}
                </option>
              )}
              {Object.entries(PICTURE_TYPE_LABELS).map(([value, label]) => (
                <option value={value} key={value}>
                  {label}
                </option>
              ))}
            </select>
          </div>
        </div>
      )}
    </div>
  );
}

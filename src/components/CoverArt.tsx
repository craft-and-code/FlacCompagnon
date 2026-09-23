// Cover art at the top of the tag panel: full width in a fixed square frame
// (cropped via `object-fit: cover` when the source isn't square itself — see
// CoverArt.css's `.tag-cover-frame` comment for why fixed-and-square won out
// over showing every cover at its own native ratio), with a banner
// underneath carrying the dimensions/format/size and a picture-type picker — Mac tag
// editors like Meta caption artwork below it rather than above.
//
// A selection whose files don't share one image of the selected type shows
// chevrons to cycle through the distinct ones, auto-advancing every 3s.

import { useEffect, useState } from "react";
import { ChevronLeft, ChevronRight, ImageDown, Music, Trash2, Upload } from "lucide-react";

import { DropHint } from "./DropHint";

import type { CoverArt as CoverArtData } from "../types";
import { PICTURE_TYPE_LABELS, coverDataUrl, pictureTypeLabel } from "../format";
import { dropZone } from "./dropZones";
import "./CoverArt.css";
import { IconButton } from "./IconButton";
import { MarqueeText } from "./MarqueeText";

const CAROUSEL_MS = 3000;

/// What the empty box and the drag overlay say. Kept next to each other
/// because they are two halves of one instruction — the first names what the
/// box takes and what dropping it does, the second confirms the release —
/// and the conversion panel's drop zone words its own pair the same way.
const RELEASE_PROMPT = "Release to import";

export interface CoverArtProps {
  covers: CoverArtData[];
  pictureType: string;
  pictureTypes: string[];
  onTypeChange: (pictureType: string) => void;
  /// Highlighted while a file is being dragged over the box.
  dragOver: boolean;
  /// A dropped image is being read from disk — shows a spinner in place of
  /// the drag-upload icon so the box isn't silently unresponsive in between.
  loading: boolean;
  /// Opens the lightbox on the currently shown cover, but hands over the
  /// whole set and its index — the lightbox has its own chevrons (see
  /// CoverModal) rather than sharing this box's carousel state, since closing
  /// it shouldn't leave this box's mini-carousel wherever the lightbox ended.
  onOpenLightbox: (covers: CoverArtData[], index: number) => void;
  /// Removes pictures of the selected type from every selected file.
  onDelete: () => void;
  /// Extracts every distinct picture in `covers`, not just the one currently
  /// shown — a selection whose files don't all share the exact same image
  /// needs all of them written, or clicking through the carousel and
  /// extracting each in turn would silently overwrite the last one with the
  /// next (see extractCoverArt's numbered-filename fix for this).
  onExtract: (covers: CoverArtData[]) => void;
}

function infoLine(cover: CoverArtData, multiple: boolean): string {
  const kb = Math.max(1, Math.round(cover.size_bytes / 1024));
  const kind = cover.mime.replace(/^image\//, "").toUpperCase() || "?";
  return `${cover.width}×${cover.height} · ${kind} · ${kb} KB${multiple ? " · multiple covers" : ""}`;
}

export function CoverArt({
  covers,
  pictureType,
  pictureTypes,
  onTypeChange,
  dragOver,
  loading,
  onOpenLightbox,
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
  // No `drag-over` modifier on the frame: the whole drag feedback is the
  // overlay below, which covers it edge to edge (see CoverArt.css).

  // Plain <button>s rather than IconButton: `.icon-btn`'s family is a flat,
  // muted-at-rest control meant for a toolbar or a table row, but these float
  // as a circular, semi-transparent overlay directly on top of the artwork —
  // a different visual language IconButton isn't meant to cover, not a
  // variant it's missing.
  const nav = multiple && (
    <>
      <button
        className="tag-cover-nav tag-cover-prev"
        type="button"
        title="Previous cover"
        onClick={() => step(-1)}
      >
        <ChevronLeft size={16} strokeWidth={2.2} />
      </button>
      <button
        className="tag-cover-nav tag-cover-next"
        type="button"
        title="Next cover"
        onClick={() => step(1)}
      >
        <ChevronRight size={16} strokeWidth={2.2} />
      </button>
    </>
  );

  // The overlay covers the frame edge to edge, but its veil is translucent —
  // artwork is meant to stay visible underneath, which is the point. The empty
  // box's own prompt is not: it would show through the release prompt, two
  // lines of text on top of each other. So the placeholder steps aside while
  // the overlay is up, giving the same one-prompt-at-a-time reading as the
  // conversion panel, which swaps its single hint rather than stacking two.
  const overlay = dragOver || loading;

  // One shared frame markup for both the "has a cover" and "no cover yet"
  // cases — the drag/loading overlay and the drop-target styling apply to
  // the box itself, not to whatever happens to be inside it.
  return (
    <div className="tag-cover">
      {/* The drop target is this square alone, not the info band below it —
          see dropZones.ts. */}
      <div className="tag-cover-frame" {...dropZone("cover")}>
        {cover && url ? (
          <img
            className="tag-cover-img"
            src={url}
            alt=""
            onClick={() => onOpenLightbox(covers, index)}
          />
        ) : (
          !overlay && (
            <span className="tag-cover-placeholder">
              <DropHint
                icon={Music}
                label={`Drop an image to set the ${pictureTypeLabel(pictureType).toLowerCase()} on the selected tracks`}
              />
            </span>
          )
        )}
        {nav}
        {overlay && (
          <div className="tag-cover-overlay">
            {loading ? (
              <span className="spinner" />
            ) : (
              <DropHint icon={ImageDown} label={RELEASE_PROMPT} tone="over" />
            )}
          </div>
        )}
      </div>
      <div className="tag-cover-info">
        {cover && url ? (
          <MarqueeText className="tag-cover-info-text" text={infoLine(cover, multiple)} />
        ) : (
          <span className="tag-cover-info-text" />
        )}
        <div className="tag-cover-controls">
          {cover && url && (
            <>
              <IconButton
                icon={<Upload size={14} strokeWidth={1.6} />}
                title={
                  multiple
                    ? `Extract all ${covers.length} ${pictureTypeLabel(pictureType)} images next to the audio files`
                    : `Extract this ${pictureTypeLabel(pictureType)} image next to the audio file`
                }
                onClick={() => onExtract(covers)}
              />
              <IconButton
                icon={<Trash2 size={14} strokeWidth={1.6} />}
                title={`Delete ${pictureTypeLabel(pictureType)} images`}
                variant="danger-persistent"
                onClick={onDelete}
              />
            </>
          )}
          <select
            className="tag-cover-role"
            value={pictureType}
            title="Select image type"
            aria-label="Image type"
            onChange={(ev) => onTypeChange(ev.target.value)}
          >
            {pictureTypes.map((value) => (
              <option value={value} key={value}>
                {PICTURE_TYPE_LABELS[value] ?? value}
              </option>
            ))}
          </select>
        </div>
      </div>
    </div>
  );
}

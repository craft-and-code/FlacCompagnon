// The conversion panel's drop target — sits where the tag panel's cover art
// does, full-bleed at the top of the panel, but takes audio files/folders
// instead of a single image. Purely presentational: drag state, the imported
// count and the busy animation are all handed in.

// `FileAudio`, not `Upload` — `Upload` is already CoverArt.tsx's icon for
// *extracting* a cover to disk; reusing it here for the opposite direction
// (dropping audio files in) reads as the wrong action once both are visible
// side by side.
import { ArrowDownToLine, FileAudio } from "lucide-react";

import { DropHint } from "./DropHint";
import { dropZone } from "./dropZones";
import "./ConvertDropzone.css";

export interface ConvertDropzoneProps {
  /// A file is being dragged over this box specifically (not just the
  /// window) — see useNativeDrop's `overConvert`.
  dragOver: boolean;
  /// A dropped folder is being walked for the audio files it holds. Short for
  /// an album, long enough to notice for a deep tree — and until it finishes
  /// the list is unchanged, which without this reads as a drop that missed.
  importing: boolean;
  /// A conversion batch is running — swaps the prompt for a spinner and
  /// per-file progress text, the "animation" requested for this state.
  busy: boolean;
  progressLabel: string;
  itemCount: number;
}

export function ConvertDropzone({
  dragOver,
  importing,
  busy,
  progressLabel,
  itemCount,
}: ConvertDropzoneProps) {
  const frameClass = dragOver ? "convert-drop-frame drag-over" : "convert-drop-frame";

  return (
    <div className={frameClass} {...dropZone("convert")}>
      {/* Checked before `busy`: the two can't overlap (the app is frozen for
          the length of a conversion, so nothing can be dropped then), and
          importing is the shorter-lived of the two. */}
      {importing ? (
        <>
          <span className="spinner" />
          <p className="convert-drop-text">Looking for audio files…</p>
        </>
      ) : busy ? (
        <>
          <span className="spinner" />
          <p className="convert-drop-text">{progressLabel}</p>
        </>
      ) : (
        /* Same icon swap as the main dropzone: at rest it names what the box
           takes, under a drag it says "release here". Rendered through the
           shared `DropHint` so the cover box on the other side of the table
           stays identical in size, colour and spacing. */
        <DropHint
          icon={dragOver ? ArrowDownToLine : FileAudio}
          tone={dragOver ? "over" : "rest"}
          label={
            dragOver
              ? "Release to import"
              : itemCount === 0
                ? "Drop audio files or folders to convert"
                : `${itemCount} track${itemCount === 1 ? "" : "s"} imported — drop more anytime`
          }
        />
      )}
    </div>
  );
}

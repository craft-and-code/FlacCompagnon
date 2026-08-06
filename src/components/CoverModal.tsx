// Cover lightbox: the tag panel's artwork at full size, with its dimensions,
// format, weight and picture role underneath.

import type { CoverArt } from "../types";
import { coverDataUrl, pictureTypeLabel } from "../format";
import { Modal } from "./Modal";

export interface CoverModalProps {
  /// The cover to show; `null` closes the lightbox.
  cover: CoverArt | null;
  onClose: () => void;
}

function caption(cover: CoverArt): string {
  const kb = Math.max(1, Math.round(cover.size_bytes / 1024));
  const kind = cover.mime.replace(/^image\//, "").toUpperCase() || "?";
  return `${cover.width}×${cover.height} · ${kind} · ${kb} KB · ${pictureTypeLabel(cover.picture_type)}`;
}

export function CoverModal({ cover, onClose }: CoverModalProps) {
  const url = cover ? coverDataUrl(cover) : null;
  return (
    <Modal open={cover != null && url != null} onClose={onClose}>
      {cover && url && (
        <>
          <img className="cover-modal-img" src={url} alt="" />
          <p className="cover-modal-info">{caption(cover)}</p>
        </>
      )}
    </Modal>
  );
}

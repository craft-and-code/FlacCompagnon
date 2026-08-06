// Shared pop-in shell: backdrop, centered panel, close button, Escape.
//
// Every modal in the app has the same three behaviours — click the backdrop to
// dismiss, clicks inside the panel must not bubble to that backdrop, and
// Escape closes it — so they live here once rather than being re-implemented
// (and re-forgotten) per modal.

import { useEffect, type ReactNode } from "react";
import { X } from "lucide-react";

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  /// Extra class on the inner panel, for per-modal sizing (e.g. "lookup-inner").
  innerClassName?: string;
  /// Heading shown above the content. Omitted for the cover lightbox, which is
  /// just the artwork.
  title?: string;
  children: ReactNode;
}

export function Modal({ open, onClose, innerClassName, title, children }: ModalProps) {
  // Bound only while open, so a closed modal can't swallow an Escape meant for
  // whichever one is actually on screen.
  useEffect(() => {
    if (!open) return;
    const onKeyDown = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="cover-modal" onClick={onClose}>
      <div
        className={innerClassName ? `cover-modal-inner ${innerClassName}` : "cover-modal-inner"}
        onClick={(ev) => ev.stopPropagation()}
      >
        <button className="cover-modal-close" title="Close" onClick={onClose}>
          <X size={15} strokeWidth={1.8} />
        </button>
        {title && <h3 className="extended-tags-title">{title}</h3>}
        {children}
      </div>
    </div>
  );
}

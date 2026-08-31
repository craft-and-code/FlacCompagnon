// The centred "icon over a one-line prompt" shown inside a full-bleed drop
// target: the tag panel's cover box on the left, the conversion panel's drop
// zone on the right.
//
// Extracted because there are two of them. They sit in mirrored slots on
// either side of the results table and are read as a pair, so an icon a few
// pixels bigger on one side, or a prompt on one and none on the other, reads
// as an inconsistency rather than as a distinction — which is exactly what it
// was. One declaration site means the two cannot drift again.
//
// What differs between the two is only the icon and the wording, so those are
// the props. The colour is not: it follows `tone`, because the two states
// (idle over the panel background, or over the drag veil) each have one right
// answer and neither caller should be picking it.

import type { LucideIcon } from "lucide-react";

import "./DropHint.css";

/// Both boxes are the same width, so both icons are the same size. 32px is
/// what the conversion side already used; the cover placeholder's 44px was
/// the odd one out.
const ICON_SIZE = 32;

export interface DropHintProps {
  icon: LucideIcon;
  label: string;
  /// `"rest"` — sitting on the panel background, accent icon over muted text.
  /// `"over"` — on top of the `--drop-veil` wash, everything white.
  tone?: "rest" | "over";
}

export function DropHint({ icon: Icon, label, tone = "rest" }: DropHintProps) {
  return (
    <span className={`drop-hint drop-hint-${tone}`}>
      <Icon size={ICON_SIZE} strokeWidth={1.4} />
      <span className="drop-hint-label">{label}</span>
    </span>
  );
}

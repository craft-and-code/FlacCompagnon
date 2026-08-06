// Every icon-only button in the app — the row's reveal/delete, the tag
// panel's close cross, the cover's extract/delete, the progress bar's
// cancel — used to each hand-roll its own <button className="...-btn">
// with its own size/radius/hover colors, which is exactly how they'd drifted
// out of sync (different border-radius, different hover background, one
// trash icon red only on hover while another was always red). One
// component, one CSS family (`.icon-btn` + a variant class) fixes a whole
// class of these at once, and keeps the next one from drifting too.

import type { MouseEvent, ReactNode } from "react";

export interface IconButtonProps {
  icon: ReactNode;
  title: string;
  onClick: (ev: MouseEvent<HTMLButtonElement>) => void;
  /// Table rows listen for mousedown to start a drag/selection; a button
  /// inside one has to swallow it first or clicking it would also drag or
  /// select the row underneath.
  onMouseDown?: (ev: MouseEvent<HTMLButtonElement>) => void;
  /// - "neutral" (default): muted at rest, turns accent blue on hover —
  ///   reveal-in-Finder, extract-cover, anything that just *does* something.
  /// - "close": muted at rest, turns red on hover with no background tint —
  ///   dismisses or stops something (close panel, cancel a task), which
  ///   isn't destructive enough to warrant a permanent red warning.
  /// - "danger": muted at rest, turns red (icon + tinted background) on
  ///   hover — deletes something, but sits in a repeated list (a table row)
  ///   where a permanently red icon on every row would be noise.
  /// - "danger-persistent": same red hover as "danger", but red even at
  ///   rest — for a delete button that stands alone rather than repeating
  ///   down a list, where the extra permanent warning earns its keep.
  variant?: "neutral" | "close" | "danger" | "danger-persistent";
  disabled?: boolean;
  className?: string;
}

export function IconButton({
  icon,
  title,
  onClick,
  onMouseDown,
  variant = "neutral",
  disabled,
  className,
}: IconButtonProps) {
  const variantClass = variant === "neutral" ? "" : `icon-btn-${variant}`;
  const cls = ["icon-btn", variantClass, className].filter(Boolean).join(" ");
  return (
    <button
      type="button"
      className={cls}
      title={title}
      disabled={disabled}
      onMouseDown={onMouseDown}
      onClick={onClick}
    >
      {icon}
    </button>
  );
}

// Keyboard entry points must obey the page's availability gate as well as
// the focus/modal guards; a pointer-blocking conversion overlay cannot stop keys.

export interface GlobalShortcutOptions {
  /// False while the owning view is inactive or unavailable.
  enabled?: boolean;
  /// A modal's own shortcuts can opt out of the page's modal guard.
  insideModal?: boolean;
}

export function dispatchGlobalShortcut(
  event: KeyboardEvent,
  onKey: (event: KeyboardEvent) => void,
  { enabled = true, insideModal = false }: GlobalShortcutOptions,
  hasModal: () => boolean,
): void {
  if (!event.target || !enabled) return;
  const target = event.target as HTMLElement;
  if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)
    return;
  if (!insideModal && hasModal()) return;
  onKey(event);
}

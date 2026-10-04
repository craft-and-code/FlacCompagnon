// Native menu events use the same availability gate as the toolbar. The OS
// menu can still emit an action while the corresponding webview control is disabled.

import type { SpectrogramSize } from "../types";

export interface MenuActions {
  exportM3u: () => void;
  exportM3uExtended: () => void;
  exportCsv: () => void;
  exportJson: () => void;
  reset: () => void;
  setSpectrogramSize: (size: SpectrogramSize) => void;
}

export function dispatchMenuAction(action: string, actions: MenuActions, enabled: boolean): void {
  if (!enabled) return;
  switch (action) {
    case "export_m3u":
      actions.exportM3u();
      break;
    case "export_m3u_extended":
      actions.exportM3uExtended();
      break;
    case "export_csv":
      actions.exportCsv();
      break;
    case "export_json":
      actions.exportJson();
      break;
    case "reset":
      actions.reset();
      break;
    case "spectrogram_size_small":
      actions.setSpectrogramSize("half");
      break;
    case "spectrogram_size_full":
      actions.setSpectrogramSize("full");
      break;
  }
}

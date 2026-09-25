// The preferred spectrogram dimensions, shared by the native menu and the
// generation button. Like the theme and column preferences, this survives a
// restart and tolerates a webview where storage is unavailable.

import { useEffect, useState } from "react";

import * as api from "../api";
import type { SpectrogramSize } from "../types";

export const SPECTROGRAM_SIZE_STORAGE_KEY = "spectrogramSize";

/// Interpret the only two stored values this preference supports. Keeping the
/// fallback here means an old, corrupt or manually edited preference cannot
/// make spectrogram generation send an invalid size to the Rust command.
export function readSpectrogramSize(value: string | null): SpectrogramSize {
  return value === "half" || value === "full" ? value : "half";
}

/// Write through the same tiny storage boundary as the column preferences.
/// The hook owns the unavailable-storage fallback because it is UI state.
export function writeSpectrogramSize(
  storage: Pick<Storage, "setItem">,
  size: SpectrogramSize,
) {
  storage.setItem(SPECTROGRAM_SIZE_STORAGE_KEY, size);
}

function stored(): SpectrogramSize {
  try {
    return readSpectrogramSize(localStorage.getItem(SPECTROGRAM_SIZE_STORAGE_KEY));
  } catch {
    /* ignore */
  }
  return "half";
}

export function useSpectrogramSize() {
  const [size, setSize] = useState<SpectrogramSize>(stored);

  useEffect(() => {
    try {
      writeSpectrogramSize(localStorage, size);
    } catch {
      /* ignore */
    }
    void api.syncSpectrogramSize(size).catch(() => {
      // The preference still remains local even if the native menu is not
      // available during an unusual webview startup.
    });
  }, [size]);

  return { size, setSize };
}

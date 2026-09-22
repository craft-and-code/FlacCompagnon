// The preferred spectrogram dimensions, shared by the native menu and the
// generation button. Like the theme and column preferences, this survives a
// restart and tolerates a webview where storage is unavailable.

import { useEffect, useState } from "react";

import * as api from "../api";
import type { SpectrogramSize } from "../types";

const KEY = "spectrogramSize";

function stored(): SpectrogramSize {
  try {
    const value = localStorage.getItem(KEY);
    if (value === "half" || value === "full") return value;
  } catch {
    /* ignore */
  }
  return "half";
}

export function useSpectrogramSize() {
  const [size, setSize] = useState<SpectrogramSize>(stored);

  useEffect(() => {
    try {
      localStorage.setItem(KEY, size);
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

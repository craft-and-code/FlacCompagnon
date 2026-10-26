// Persisted native-menu choices share one storage/synchronization boundary.
import { useEffect, useState } from "react";
import * as api from "../api";
import type { SpectrogramSize, SpectrogramColor } from "../types";

export const SPECTROGRAM_SIZE_STORAGE_KEY = "spectrogramSize";
export const SPECTROGRAM_COLOR_STORAGE_KEY = "spectrogramColor";

export function readSpectrogramSize(value: string | null): SpectrogramSize {
  return value === "full" ? "full" : "half";
}

export function readSpectrogramColor(value: string | null): SpectrogramColor {
  return value === "blue" || value === "amber" || value === "violet" ? value : "green";
}

export function writeSpectrogramSize(storage: Pick<Storage, "setItem">, value: SpectrogramSize) {
  storage.setItem(SPECTROGRAM_SIZE_STORAGE_KEY, value);
}

function usePreference<T extends string>(
  key: string,
  read: (value: string | null) => T,
  synchronize: (value: T) => Promise<void>,
) {
  const [value, setValue] = useState<T>(() => {
    try {
      return read(localStorage.getItem(key));
    } catch {
      return read(null);
    }
  });
  useEffect(() => {
    try {
      localStorage.setItem(key, value);
    } catch {
      /* Storage may be unavailable. */
    }
    void synchronize(value).catch(() => {
      /* The local preference remains usable. */
    });
  }, [key, value, synchronize]);
  return [value, setValue] as const;
}

export function useSpectrogramSettings() {
  const [size, setSize] = usePreference(
    SPECTROGRAM_SIZE_STORAGE_KEY,
    readSpectrogramSize,
    api.syncSpectrogramSize,
  );
  const [color, setColor] = usePreference(
    SPECTROGRAM_COLOR_STORAGE_KEY,
    readSpectrogramColor,
    api.syncSpectrogramColor,
  );
  return { size, setSize, color, setColor };
}

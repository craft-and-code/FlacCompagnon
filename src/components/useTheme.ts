// Theme (Auto / Light / Dark), defaulting to the OS preference.
//
// "Auto" is the absence of a `data-theme` attribute, letting the stylesheet's
// own `prefers-color-scheme` media query decide — so following the OS needs no
// listener here. localStorage access is wrapped because a webview with storage
// disabled would otherwise throw on startup.

import { useCallback, useEffect, useState } from "react";

import type { Theme } from "../types";

const KEY = "theme";
const ORDER: Theme[] = ["auto", "light", "dark"];

const LABELS: Record<Theme, string> = {
  auto: "◐ Auto",
  light: "☀ Light",
  dark: "☾ Dark",
};

function stored(): Theme {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "light" || saved === "dark" || saved === "auto") return saved;
  } catch {
    /* ignore */
  }
  return "auto";
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(stored);

  useEffect(() => {
    const root = document.documentElement;
    if (theme === "auto") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", theme);
    try {
      localStorage.setItem(KEY, theme);
    } catch {
      /* ignore */
    }
  }, [theme]);

  const cycle = useCallback(() => {
    setTheme((t) => ORDER[(ORDER.indexOf(t) + 1) % ORDER.length]);
  }, []);

  return { theme, label: LABELS[theme], cycle };
}

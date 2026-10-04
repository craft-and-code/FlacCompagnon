// Backend events and window lifecycle — the parts of the app that aren't
// driven by a click anywhere in the React tree.

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import type { Progress } from "../types";
import { useLatest } from "./useLatest";
import { dispatchMenuAction, type MenuActions } from "./menuActions";

export type { MenuActions } from "./menuActions";

/// Native menu bar (built in src-tauri/src/menu.rs). Each item only emits its
/// id; routing it here means every action keeps exactly one implementation —
/// the same function the equivalent toolbar button calls.
export function useMenuActions(actions: MenuActions, enabled = true) {
  const latest = useLatest({ actions, enabled });
  useEffect(() => {
    const unlisten = listen<string>("menu://action", (e) => {
      const { actions: current, enabled: available } = latest.current;
      dispatchMenuAction(e.payload, current, available);
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [latest]);
}

export function useProgressEvents(onProgress: (p: Progress, verb: string) => void) {
  const latest = useLatest(onProgress);
  useEffect(() => {
    const a = listen<Progress>("analyze://progress", (e) =>
      latest.current(e.payload, "Analyzing"),
    );
    const s = listen<Progress>("spectro://progress", (e) =>
      latest.current(e.payload, "Rendering"),
    );
    return () => {
      void a.then((f) => f());
      void s.then((f) => f());
    };
  }, [latest]);
}

// Conversion progress, kept apart from `useProgressEvents` above: that one
// drives the shared bottom progress bar shown while the whole app is idle
// between imports, but the conversion panel animates its own dropzone while
// it runs instead (see ConvertPanel) — a separate listener, not a third verb
// on the same one.
export function useConvertProgress(onProgress: (p: Progress) => void) {
  const latest = useLatest(onProgress);
  useEffect(() => {
    const c = listen<Progress>("convert://progress", (e) => latest.current(e.payload));
    return () => {
      void c.then((f) => f());
    };
  }, [latest]);
}

// The window starts hidden (tauri.conf.json: visible=false) and is revealed
// once the page is fully loaded — not merely once this module has run.
//
// Revealing immediately looks safe because the stylesheet is imported at the
// top of the entry module, but that only orders *script* execution, not
// *rendering*: the bundled CSS is a <link> the browser fetches independently,
// and script execution doesn't wait for it. On a cold start the webview could
// still be painting its first unstyled frame — the "menus without CSS" flash.
// Waiting for `load` removes the race rather than narrowing it.
//
// Do NOT wrap this in requestAnimationFrame: WebKit suspends rendering
// callbacks for hidden windows, so it would never fire and the window would
// stay invisible forever.
export function useRevealWindow() {
  useEffect(() => {
    let done = false;
    const reveal = () => {
      if (done) return;
      done = true;
      const w = getCurrentWindow();
      w.show()
        .then(() => w.setFocus())
        .catch(() => {});
    };

    if (document.readyState === "complete") {
      reveal();
    } else {
      window.addEventListener("load", reveal, { once: true });
    }
    // Defensive fallback: a stalled asset shouldn't leave the user staring at
    // nothing, and this also covers dev-server reloads where `load` never
    // fires again.
    const timer = window.setTimeout(reveal, 2000);
    return () => {
      window.removeEventListener("load", reveal);
      window.clearTimeout(timer);
    };
  }, []);
}

// This is a native desktop app, not a browser tab. The webview's own
// right-click menu (Reload, Inspect Element, …) isn't suppressed by Tauri in
// development *or* production, and nothing in this UI has its own right-click
// behaviour, so it's disabled outright rather than left showing browser chrome
// that doesn't belong here.
export function useSuppressContextMenu() {
  useEffect(() => {
    const onContextMenu = (ev: MouseEvent) => ev.preventDefault();
    document.addEventListener("contextmenu", onContextMenu);
    return () => document.removeEventListener("contextmenu", onContextMenu);
  }, []);
}

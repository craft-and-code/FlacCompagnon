// Native OS file drop.
//
// This is a Tauri window event, not an HTML5 one: `dragDropEnabled` hands drops
// to the OS layer, which is why the webview's own drag-and-drop API is dead
// here (and why row reordering uses raw mouse events — see useRowDrag).
//
// Positions arrive in physical pixels while DOM rects are logical (CSS) ones,
// so everything is divided by the device pixel ratio before being compared to
// anything measured from the page.

import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import type { CoverArt } from "../types";
import * as api from "../api";
import type { TagPanelHandle } from "./TagPanel";
import { useLatest } from "./useLatest";

const COVER_IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

export interface UseNativeDropArgs {
  busy: boolean;
  tagPanelRef: React.RefObject<TagPanelHandle | null>;
  onAnalyze: (paths: string[]) => void;
  onLoadReport: (path: string) => void;
  onToast: (msg: string, kind?: "info" | "error") => void;
}

export interface NativeDropState {
  /// Files are hovering over the window and would be accepted.
  overWindow: boolean;
  /// ...specifically over the tag panel's cover box.
  overCover: boolean;
  /// A drop was attempted mid-analysis and refused.
  blocked: boolean;
}

export function useNativeDrop({
  busy,
  tagPanelRef,
  onAnalyze,
  onLoadReport,
  onToast,
}: UseNativeDropArgs): NativeDropState {
  const [state, setState] = useState<NativeDropState>({
    overWindow: false,
    overCover: false,
    blocked: false,
  });

  // Everything the handler needs, read live — so this subscribes exactly once
  // for the lifetime of the app (see useLatest for why that matters).
  const latest = useLatest({ busy, tagPanelRef, onAnalyze, onLoadReport, onToast });

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      const { busy, tagPanelRef, onAnalyze, onLoadReport, onToast } = latest.current;

      if (busy) {
        // Dropping is refused during analysis; say so rather than silently
        // ignoring it.
        setState({
          overWindow: false,
          overCover: false,
          blocked: p.type === "enter" || p.type === "over",
        });
        return;
      }

      if (p.type === "enter" || p.type === "over") {
        const scale = window.devicePixelRatio || 1;
        const overCover =
          tagPanelRef.current?.containsPoint(p.position.x / scale, p.position.y / scale) ?? false;
        setState({ overWindow: !overCover, overCover, blocked: false });
        return;
      }

      if (p.type === "drop") {
        const scale = window.devicePixelRatio || 1;
        const onCover =
          tagPanelRef.current?.containsPoint(p.position.x / scale, p.position.y / scale) ?? false;
        setState({ overWindow: false, overCover: false, blocked: false });

        void (async () => {
          // A single image dropped on the cover box replaces the artwork of
          // every selected file rather than being analyzed as audio.
          if (onCover && p.paths.length === 1) {
            const ext = p.paths[0].split(/[\\/]/).pop()?.split(".").pop()?.toLowerCase();
            if (ext && COVER_IMAGE_EXTS.includes(ext)) {
              tagPanelRef.current?.setCoverLoading(true);
              try {
                const cover: CoverArt = await api.readCoverImage(p.paths[0]);
                tagPanelRef.current?.stageCover(cover);
              } catch (e) {
                onToast(String(e), "error");
              } finally {
                tagPanelRef.current?.setCoverLoading(false);
              }
              return;
            }
          }
          // A single previously-saved .json report reloads the table instead
          // of being analyzed — there's no button for this, just the drop.
          if (p.paths.length === 1 && p.paths[0].toLowerCase().endsWith(".json")) {
            onLoadReport(p.paths[0]);
          } else if (p.paths.length > 0) {
            onAnalyze(p.paths);
          }
        })();
        return;
      }

      setState({ overWindow: false, overCover: false, blocked: false });
    });

    return () => {
      void unlisten.then((f) => f());
    };
  }, [latest]);

  return state;
}

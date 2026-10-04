// Native OS file drop.
//
// This is a Tauri window event, not an HTML5 one: `dragDropEnabled` hands drops
// to the OS layer, which is why the webview's own drag-and-drop API is dead
// here (and why row reordering uses raw mouse events — see useRowDrag).
//
// Where a drop landed is resolved by `dropZones.ts` — see that file for why
// the three targets declare themselves instead of being measured from here.
//
// What's left in this file is the one thing that can't be delegated: deciding
// what unit an incoming position is in. Tauri *types* them as physical pixels,
// but that isn't reliable — on macOS they arrive already in logical (CSS)
// ones, and dividing by the device pixel ratio there pushes every drop into
// the top-left quadrant. (Related, still open:
// https://github.com/tauri-apps/tauri/issues/10744.)
//
// There's no flag to ask, so the value is tested rather than assumed: a
// physical coordinate on a scaled display is, by construction, larger than the
// CSS viewport it came from. If the raw value still lands inside the viewport
// it was already logical; only when it can't be a CSS coordinate at all is it
// divided. An earlier attempt calibrated against `webview.size()` instead,
// which failed for an instructive reason — that call reports genuine physical
// pixels even where the drop positions don't, so the ratio came back as the
// device pixel ratio and nothing changed. The two conventions have to be
// compared against the *page*, not against each other.

import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import * as api from "../api";
import { IMAGE_EXT_LIST, isAudioPath, isImagePath } from "../format";
import { dropZoneAt, isInsideViewport, type DropZoneKind } from "./dropZones";
import type { TagPanelHandle } from "./TagPanel";
import { useLatest } from "./useLatest";

/// A drop position in whatever unit Tauri gave us, in CSS pixels.
function toCssPoint(pos: { x: number; y: number }): { x: number; y: number } {
  if (isInsideViewport(pos.x, pos.y)) return { x: pos.x, y: pos.y };
  const dpr = window.devicePixelRatio || 1;
  return { x: pos.x / dpr, y: pos.y / dpr };
}

export interface UseNativeDropArgs {
  busy: boolean;
  /// A conversion batch is running — refuses drops the same way `busy` does,
  /// kept as a separate flag since the two features have separate busy
  /// states (see useConvertPanel).
  converting: boolean;
  /// Only for staging a dropped cover image — the tag panel no longer takes
  /// part in hit-testing (see dropZones.ts).
  tagPanelRef: React.RefObject<TagPanelHandle | null>;
  onAnalyze: (paths: string[]) => void;
  onLoadReport: (path: string) => void;
  /// Files/folders dropped on the conversion panel's own drop target — kept
  /// entirely separate from `onAnalyze`, since importing a track there never
  /// adds it to the main results table.
  onImportForConvert: (paths: string[]) => void;
  onToast: (msg: string, kind?: "info" | "error") => void;
}

export interface NativeDropState {
  /// Files are hovering over the results list / empty-state dropzone, the
  /// only place a drop imports them for analysis.
  overList: boolean;
  /// ...over the tag panel's cover box.
  overCover: boolean;
  /// ...over the conversion panel's drop target.
  overConvert: boolean;
  /// A drop was attempted while busy (analyzing or converting) and refused.
  blocked: boolean;
}

export function useNativeDrop({
  busy,
  converting,
  tagPanelRef,
  onAnalyze,
  onLoadReport,
  onImportForConvert,
  onToast,
}: UseNativeDropArgs): NativeDropState {
  const [state, setState] = useState<NativeDropState>({
    overList: false,
    overCover: false,
    overConvert: false,
    blocked: false,
  });

  // Everything the handler needs, read live — so this subscribes exactly once
  // for the lifetime of the app (see useLatest for why that matters).
  const latest = useLatest({
    busy,
    converting,
    tagPanelRef,
    onAnalyze,
    onLoadReport,
    onImportForConvert,
    onToast,
  });

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      const { busy, converting, tagPanelRef, onAnalyze, onLoadReport, onImportForConvert, onToast } =
        latest.current;

      if (busy || converting) {
        // Dropping is refused while either batch runs; say so rather than
        // silently ignoring it.
        setState({
          overList: false,
          overCover: false,
          overConvert: false,
          blocked: p.type === "enter" || p.type === "over",
        });
        return;
      }

      if (p.type === "enter" || p.type === "over") {
        const { x, y } = toCssPoint(p.position);
        const zone: DropZoneKind | null = dropZoneAt(x, y);
        setState({
          overList: zone === "list",
          overCover: zone === "cover",
          overConvert: zone === "convert",
          blocked: false,
        });
        return;
      }

      if (p.type === "drop") {
        const { x, y } = toCssPoint(p.position);
        const zone = dropZoneAt(x, y);
        setState({ overList: false, overCover: false, overConvert: false, blocked: false });

        const paths = p.paths;

        if (paths.length === 0) return;

        void (async () => {
          // The cover box takes exactly one image and nothing else. Anything
          // else dropped there is an error rather than a fall-through to
          // analysis: the user aimed at the artwork, so silently importing
          // their file as a track instead would be the wrong repair.
          if (zone === "cover") {
            if (paths.length > 1 || !isImagePath(paths[0])) {
              onToast(
                paths.length > 1
                  ? "Drop a single image on the cover, not several files."
                  : `The cover only accepts images (${IMAGE_EXT_LIST}).`,
                "error",
              );
              return;
            }
            const panel = tagPanelRef.current;
            if (!panel) return;
            try {
              await panel.importCover(api.readCoverImage(paths[0]));
            } catch (e) {
              onToast(String(e), "error");
            }
            return;
          }
          // Dropped on the conversion panel's own drop target: import for
          // conversion, never analyzed and never touching the results table.
          if (zone === "convert") {
            // Refused here rather than silently imported and failed later by
            // the backend, so a mis-aimed drop (an image meant for the cover
            // is the likely one) says so straight away — same contract as the
            // cover target above, which refuses audio.
            if (!paths.every(isAudioPath)) {
              onToast("The conversion panel only accepts audio files or folders.", "error");
              return;
            }
            onImportForConvert(paths);
            return;
          }
          // Anywhere else is not a drop target at all. Saying so beats both
          // importing by accident (what the whole window used to do) and
          // going silent, which reads as the app having missed the drop.
          if (zone !== "list") {
            onToast("Drop audio files on the list to add them.");
            return;
          }
          // A single previously-saved .json report reloads the table instead
          // of being analyzed — there's no button for this, just the drop.
          if (paths.length === 1 && paths[0].toLowerCase().endsWith(".json")) {
            onLoadReport(paths[0]);
          } else {
            onAnalyze(paths);
          }
        })();
        return;
      }

      setState({ overList: false, overCover: false, overConvert: false, blocked: false });
    });

    return () => {
      void unlisten.then((f) => f());
    };
  }, [latest]);

  return state;
}

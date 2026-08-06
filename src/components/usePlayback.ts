// Single-track preview playback.
//
// The audio itself lives in the Rust backend (a `cpal` stream owned by the
// audio thread), not here — which is why this module stops playback once on
// mount: a webview reload restarts the JS with no idea a track is playing,
// while the Rust side keeps going. Without that call the UI would look idle
// with audio still coming out and nothing left to stop it.
//
// `requestId` pairs each play with the backend's `playback://finished` event,
// so a stale notification from a track that was already superseded can't
// trigger auto-advance on the wrong row.

import { useCallback, useEffect, useRef, useState } from "react";

import type { PlaybackFinished } from "../types";
import * as api from "../api";
import { listen } from "@tauri-apps/api/event";

export interface NowPlaying {
  path: string;
  requestId: number;
}

export function usePlayback(
  orderedPaths: string[],
  onToast: (msg: string, kind?: "info" | "error") => void,
) {
  const [nowPlaying, setNowPlaying] = useState<NowPlaying | null>(null);
  // Mirrors `nowPlaying` for the "finished" listener, which must not read it
  // through a state updater: React may invoke an updater more than once, and
  // auto-advance has to fire exactly once per finished track.
  const current = useRef<NowPlaying | null>(null);
  const orderRef = useRef(orderedPaths);
  orderRef.current = orderedPaths;

  useEffect(() => {
    current.current = nowPlaying;
  }, [nowPlaying]);

  useEffect(() => {
    api.stopPlayback().catch(() => {});
  }, []);

  const stop = useCallback(() => {
    setNowPlaying(null);
    api.stopPlayback().catch(() => {});
  }, []);

  const play = useCallback(
    async (path: string) => {
      try {
        const requestId = await api.playTrack(path);
        setNowPlaying({ path, requestId });
      } catch (e) {
        setNowPlaying(null);
        onToast(String(e), "error");
      }
    },
    [onToast],
  );

  const togglePlay = useCallback(
    (path: string) => {
      if (nowPlaying?.path === path) {
        stop();
        return;
      }
      void play(path);
    },
    [nowPlaying, play, stop],
  );

  /// Stop only if `path` is what's currently playing — used when that row is
  /// about to leave the table.
  const stopIfPlaying = useCallback(
    (path: string) => {
      if (nowPlaying?.path === path) stop();
    },
    [nowPlaying, stop],
  );

  // Auto-advance to the next row in display order when a track ends on its
  // own; stop at the end of the list. Subscribed once — the handler reads the
  // current track and order from refs, so re-ordering the table mid-playback
  // doesn't tear down and rebuild the listener.
  useEffect(() => {
    const unlisten = listen<PlaybackFinished>("playback://finished", (e) => {
      const playing = current.current;
      // A stale notification from a track already superseded by a newer play.
      if (!playing || e.payload.request_id !== playing.requestId) return;
      const order = orderRef.current;
      const idx = order.indexOf(playing.path);
      const next = idx === -1 ? undefined : order[idx + 1];
      if (next) {
        void play(next);
      } else {
        current.current = null;
        setNowPlaying(null);
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [play]);

  return { nowPlaying, togglePlay, stopIfPlaying, stop };
}

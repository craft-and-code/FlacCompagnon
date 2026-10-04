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
//
// Playback rules (see `activeQueue`): pressing Play with no selection starts
// at the top of the table and plays on to the end; with one row selected, it
// starts there and still plays on to the end; with several rows selected, it
// plays just that selection, in table order, and stops once it's exhausted.
// Whichever of those applies is decided once, when playback starts, and is
// deliberately not re-decided as the selection changes afterward — the user
// asked for a selection made *during* playback to have no effect on a
// session already under way, only on the next one.

import { useCallback, useEffect, useRef, useState } from "react";

import type { PlaybackFinished, PlaybackPosition } from "../types";
import * as api from "../api";
import { listen } from "@tauri-apps/api/event";
import { effectiveQueue } from "./playbackQueue";
import { LatestRequest } from "./latestRequest";

export interface NowPlaying {
  path: string;
  requestId: number;
}

export function usePlayback(
  orderedPaths: string[],
  selectedPaths: string[],
  onToast: (msg: string, kind?: "info" | "error") => void,
) {
  const [nowPlaying, setNowPlaying] = useState<NowPlaying | null>(null);
  // A footer-only concept layered on top of `nowPlaying` — the results
  // table's own row buttons (`togglePlay`) never look at this and keep
  // meaning "play this row / stop whatever's playing", exactly as before.
  // Only the footer's transport button tells a real pause/resume apart from
  // a full stop.
  const [paused, setPaused] = useState(false);
  // Seconds into the current track — from `playback://position` while
  // playing, or set optimistically the instant the footer's seek bar is
  // dragged (that event is throttled and stops arriving entirely while
  // paused, so waiting for it would make the bar lag behind the mouse).
  const [position, setPosition] = useState(0);
  const [volume, setVolumeState] = useState(1);
  const [muted, setMuted] = useState(false);
  // The list of tracks the current playback session walks through —
  // `effectiveQueue` computed once, the moment playback starts (see `play`),
  // from the order/selection at that instant. `null` while nothing is
  // playing. Deliberately *not* recomputed as the selection changes while a
  // track is already playing — see the module doc comment above — so
  // auto-advance and Previous/Next both read this instead of calling
  // `effectiveQueue` fresh each time.
  const [activeQueue, setActiveQueueState] = useState<string[] | null>(null);

  // Mirrors `nowPlaying`/`activeQueue` for the "finished"/"position"
  // listeners, which must not read them through a state updater: React may
  // invoke an updater more than once, and auto-advance has to fire exactly
  // once per finished track.
  const current = useRef<NowPlaying | null>(null);
  const pendingPath = useRef<string | null>(null);
  const requests = useRef(new LatestRequest());
  const activeQueueRef = useRef<string[] | null>(null);
  const orderRef = useRef(orderedPaths);
  orderRef.current = orderedPaths;
  const selectedRef = useRef(selectedPaths);
  selectedRef.current = selectedPaths;

  const setActiveQueue = useCallback((q: string[] | null) => {
    activeQueueRef.current = q;
    setActiveQueueState(q);
  }, []);

  useEffect(() => {
    api.stopPlayback().catch(() => {});
  }, []);

  const stop = useCallback(() => {
    requests.current.cancel();
    pendingPath.current = null;
    current.current = null;
    setNowPlaying(null);
    setPaused(false);
    setPosition(0);
    setActiveQueue(null);
    api.stopPlayback().catch(() => {});
  }, [setActiveQueue]);

  // Starts `path` without touching `activeQueue` — the continuation half of
  // playback, used only by auto-advance and Previous/Next (via `stepQueue`),
  // both of which must keep walking the queue already frozen for this
  // session rather than deriving a new one. Every *external* "play this"
  // action goes through `play`, below, instead.
  const startTrack = useCallback(
    async (path: string) => {
      // Retire the previous track immediately: its final event may arrive
      // while the next file is still opening, before React renders again.
      current.current = null;
      pendingPath.current = path;
      const result = await requests.current.run(() => api.playTrack(path));
      if (result.status === "superseded" || !result.isCurrent()) return;
      pendingPath.current = null;
      if (result.status === "success") {
        const playing = { path, requestId: result.value };
        current.current = playing;
        setNowPlaying(playing);
        setPaused(false);
        setPosition(0);
      } else {
        setNowPlaying(null);
        setActiveQueue(null);
        onToast(String(result.error), "error");
      }
    },
    [onToast, setActiveQueue],
  );

  // Every deliberate "play this track" action — the footer's Play button
  // with nothing loaded, a row's own inline Play icon — freezes a new
  // `activeQueue` from the current order/selection before starting `path`.
  // That's what makes this a session boundary: anything that happens to the
  // selection after this point, until the session ends, is ignored by
  // auto-advance/Previous/Next (see the module doc comment).
  const play = useCallback(
    async (path: string) => {
      setActiveQueue(effectiveQueue(orderRef.current, selectedRef.current));
      await startTrack(path);
    },
    [setActiveQueue, startTrack],
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
      if (current.current?.path === path || pendingPath.current === path) stop();
    },
    [stop],
  );

  // A real pause in place, resumed in place — see `usePlaybackQueue`'s
  // `onPlayPause`, which is what the footer's button actually calls: this
  // half only makes sense once something is already loaded, so it's a no-op
  // otherwise rather than the thing that decides what to start playing.
  const togglePause = useCallback(() => {
    if (!nowPlaying) return;
    const next = !paused;
    setPaused(next);
    const call = next ? api.pausePlayback() : api.resumePlayback();
    call.catch((e) => {
      if (current.current?.requestId !== nowPlaying.requestId) return;
      setPaused(!next); // the backend didn't actually change state — undo
      onToast(String(e), "error");
    });
  }, [nowPlaying, paused, onToast]);

  const seek = useCallback(
    (seconds: number) => {
      if (!nowPlaying) return;
      setPosition(seconds); // optimistic — see `position`'s doc comment
      api.seekPlayback(seconds).catch((e) => onToast(String(e), "error"));
    },
    [nowPlaying, onToast],
  );

  const setVolume = useCallback((v: number) => {
    const clamped = Math.min(1, Math.max(0, v));
    setVolumeState(clamped);
    api.setVolume(clamped).catch(() => {});
  }, []);

  const toggleMute = useCallback(() => {
    // Flipping the `muted` flag at volume 0 would be inaudible either way —
    // the button would look like it does nothing. Jump straight to full
    // volume instead: an earlier version tried to restore whatever level the
    // slider was at just before it reached 0, but that made the button's
    // effect unpredictable in practice (a single click straight to 0 leaves
    // "the level before" at whatever it happened to already be, which isn't
    // something the user can see or anticipate) — full volume is at least
    // always the same, known result.
    // Rounded, not a bare `=== 0`: repeated keyboard nudges can leave a
    // floating-point residue like 5e-17 after clamping toward 0 (see
    // `volumeIcon` in PlaybackSeekBar.tsx for the same issue).
    if (Math.round(volume * 100) === 0) {
      setVolumeState(1);
      api.setVolume(1).catch(() => {});
      if (muted) {
        setMuted(false);
        api.setMuted(false).catch(() => {});
      }
      return;
    }
    const next = !muted;
    setMuted(next);
    api.setMuted(next).catch(() => {});
  }, [volume, muted]);

  // Steps to the previous/next track within `activeQueue` — the footer's
  // Previous/Next buttons. Reads the queue frozen at session start (via the
  // ref, for the same "no stale closure" reason the listeners below do), not
  // a fresh `effectiveQueue` call, so a selection change mid-playback can't
  // redirect where these buttons go either.
  const stepQueue = useCallback(
    (delta: 1 | -1) => {
      const queue = activeQueueRef.current;
      const playing = current.current;
      if (!queue || !playing) return;
      const idx = queue.indexOf(playing.path);
      if (idx === -1) return;
      const target = queue[idx + delta];
      if (target) void startTrack(target);
    },
    [startTrack],
  );

  // Auto-advance to the next row in `activeQueue` when a track ends on its
  // own; stop once that queue is exhausted. Subscribed once — the handler
  // reads the current track and queue from refs, so re-ordering the table or
  // changing the selection mid-playback doesn't tear down and rebuild the
  // listener (and, per the module doc comment, must not change what it does
  // either).
  useEffect(() => {
    const unlisten = listen<PlaybackFinished>("playback://finished", (e) => {
      const playing = current.current;
      // A stale notification from a track already superseded by a newer play.
      if (!playing || e.payload.request_id !== playing.requestId) return;
      const queue = activeQueueRef.current;
      const idx = queue ? queue.indexOf(playing.path) : -1;
      const next = idx === -1 ? undefined : queue?.[idx + 1];
      if (next) {
        void startTrack(next);
      } else {
        current.current = null;
        setNowPlaying(null);
        setPaused(false);
        setPosition(0);
        setActiveQueue(null);
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [startTrack, setActiveQueue]);

  // Drives the footer's seek bar while a track plays — throttled on the
  // backend (same cadence as the equalizer bars' level events), and simply
  // stops arriving while paused, which is exactly right: the bar should hold
  // still, not creep toward 0.
  useEffect(() => {
    const unlisten = listen<PlaybackPosition>("playback://position", (e) => {
      const playing = current.current;
      if (!playing || e.payload.request_id !== playing.requestId) return;
      setPosition(e.payload.position_secs);
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, []);

  return {
    nowPlaying,
    paused,
    position,
    volume,
    muted,
    activeQueue,
    play,
    togglePlay,
    stopIfPlaying,
    stop,
    togglePause,
    seek,
    setVolume,
    toggleMute,
    stepQueue,
  };
}

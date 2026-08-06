// Audio-reactive equalizer bars for the currently playing row — Qobuz-style,
// but driven by the track's actual loudness rather than a canned animation.
//
// The level itself comes from the Rust audio callback: an RMS of the samples
// about to reach the speakers, computed right where they're already in hand,
// throttled to ~16 times/second and sent as the `playback://level` event (see
// `src-tauri/src/playback.rs`). `requestId` is filtered the same way
// `playback://finished` already is elsewhere — so a level meant for a track
// that was superseded by a newer `play` can't drive the wrong row's bars.
//
// This is one overall loudness value, not a real per-frequency spectrum (that
// would need an FFT running in the audio callback, real-time budget it hasn't
// earned yet) — the four bars fake independent movement by smoothing that
// same value at four deliberately spread-out rates, rather than showing
// four identical bars ticking in lockstep.

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import type { PlaybackLevel } from "../types";

// Each is how much of the gap to the new level that bar closes per ~60ms
// update — low means it lags well behind, high means it's almost instant.
// Spread wide on purpose: values bunched together (as an earlier version of
// this had) barely read as different bars at a glance.
const BAR_SMOOTHING = [0.14, 0.45, 0.24, 0.56];
const MIN_HEIGHT_PX = 3;
const MAX_HEIGHT_PX = 12;

function flat(): number[] {
  return BAR_SMOOTHING.map(() => MIN_HEIGHT_PX);
}

export function LiveEqualizerBars({ requestId }: { requestId: number }) {
  const [heights, setHeights] = useState<number[]>(flat);

  useEffect(() => {
    setHeights(flat());
    // 0..1 smoothed level per bar, kept outside React state so each event can
    // ease toward the new target without waiting on a render round-trip.
    let smoothed = BAR_SMOOTHING.map(() => 0);

    const unlisten = listen<PlaybackLevel>("playback://level", (e) => {
      if (e.payload.request_id !== requestId) return; // a superseded track
      const target = Math.min(1, Math.max(0, e.payload.level));
      smoothed = smoothed.map((v, i) => v + (target - v) * BAR_SMOOTHING[i]);
      setHeights(smoothed.map((v) => MIN_HEIGHT_PX + v * (MAX_HEIGHT_PX - MIN_HEIGHT_PX)));
    });

    return () => {
      void unlisten.then((f) => f());
    };
  }, [requestId]);

  return (
    <span className="eq-bars">
      {heights.map((h, i) => (
        <span key={i} style={{ height: `${h}px` }} />
      ))}
    </span>
  );
}

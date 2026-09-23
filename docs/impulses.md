# Suspected impulse detection

The Impulses column counts short isolated pulse candidates in decoded PCM and stores a limited set of locations to audition. It is a restoration cue, not a corruption verdict: percussion, edits, intentionally generated pulses and codec artefacts can satisfy a sample-domain click rule.

## Signal tested

Each decoded channel is examined separately. A candidate is an isolated pulse lasting at most 0.5 ms whose entry and exit edges largely cancel. Both edges must be at least 0.05 full scale and at least eight times the RMS of first differences in the real 5 ms context before and after the pulse. The mismatch between the opposing edges must be no more than 25% of the larger edge.

After a candidate, edges occurring within 1 ms are grouped into the same cue. This avoids turning a short multi-sample disturbance into a sequence of displayed click locations.

## Result and limits

The app shows the count only: `0`, a positive integer, or `—` when unavailable. Hovering supplies timestamps, channel numbers and durations. The detector retains only the first 32 locations of this kind for reports, ordered by time and channel, while the count itself continues beyond that cap. Simultaneous candidates on two channels count separately.

No result is published for invalid streams, streams shorter than about 11 ms, unsupported 1–32 channel/sample-rate bounds (8–768 kHz), or DSD. DSD's decoded PCM representation and conversion filter alter precisely the pulse shape the rule tests. No artificial silence is padded at file boundaries, so events without real context are not assessed.

Quiet, wide, masked or filtered clicks can be missed. A zero count does not prove that a recording has no audible discontinuity. Inspect the listed position in an editor before repair.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)+if(eq(n\,12000)\,0.6\,0)|0.1*sin(2*PI*317*t):s=48000:d=1' -c:a pcm_s16le click-left.wav
```

Import `click-left.wav`. It should report one impulse on channel 1 around 0.250 s with a duration near 0.021 ms. Open it in Audacity and zoom to that sample to inspect the injected pulse.

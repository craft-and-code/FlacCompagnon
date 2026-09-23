# Suspected clicks and digital dropouts

These measurements provide locations to audition, not a corruption verdict. They do not change the lossless-authenticity badge. Intentional short pulses, percussion and hard edits can resemble defects; no candidates does not prove that a recording is free of clicks or missing audio.

## Scope and interpretation

The streaming detector processes each decoded channel separately, without a mono downmix. The table shows **Clicks?** and **Dropouts?** counts. Hover to read locations, channel numbers and durations. Two simultaneous defects on two channels count as two channel events. Counts continue beyond the storage limit; only the first 32 locations per kind are retained, sorted by time and channel. JSON and CSV exports preserve this distinction.

- **Clicks:** isolated pulses up to 0.5 ms, with opposing edges that largely cancel. Both edges must exceed 0.05 full scale and eight times the RMS of first differences in the surrounding 5 ms before and after the pulse. The mismatch between opposing edges is limited to 25% of the larger edge. Edges within 1 ms after a detected pulse are grouped into that cue.
- **Dropouts:** exact-zero runs lasting 2–250 ms, with jumps of at least 0.02 full scale at both boundaries. The 5 ms context on each side must exceed −60 dBFS RMS; the resumed RMS must be within approximately 12 dB of the preceding RMS, and at least 90% of the resumed context must be nonzero.

These are conservative **project thresholds**, not a standardized test or a calibrated confidence score. Quiet, longer, filtered or heavily masked clicks can be missed. Near-silence with dither is not an exact-zero dropout. Fades, long pauses and leading/trailing silence are excluded. Events without enough real context at file boundaries are not assessed; no silence is padded in.

The detector accepts 1–32 channels at 8–768 kHz. Invalid samples and streams shorter than approximately 11 ms yield no reading. DSD yields no reading: conversion filtering changes the pulse and exact-zero shapes these rules measure. Lossy decoding can similarly mask sample-domain discontinuities. Old saved reports show a dash until the files are analyzed again.

## Implementation and tests

`core/src/analysis/discontinuities/` has separate click and dropout detectors, with shared result types and stream validation in `mod.rs`. Memory use is bounded by short context buffers, channel count and the location cap.

Tests live outside production code. They inject known sample-domain pulses and exact-zero intervals, then check counts, channels, timestamps and widths. Controls cover clean low/high tones, square waves, seeded noise, damped attacks, silence and fades. Additional checks cover processing boundaries, final partial buffers, report limits, malformed samples, multiple sample rates, end-to-end WAV decoding, exports and frontend rendering/sorting/search. These synthetic controls are not a validation corpus of real music.

```sh
cargo test -p flaccompagnon-core --lib analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/*.test.mjs
```

## Manual fixtures

Run in a scratch directory with FFmpeg installed, then import the WAV files. Commands refuse to overwrite existing files. These signals test this detector; their intentionally narrow spectrum is unsuitable for testing authenticity.

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)|0.1*sin(2*PI*317*t):s=48000:d=1' -c:a pcm_s16le clean.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)+if(eq(n\,12000)\,0.6\,0)|0.1*sin(2*PI*317*t):s=48000:d=1' -c:a pcm_s16le click-left.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)|if(between(n\,26400\,27839)\,0\,0.1*sin(2*PI*317*t)):s=48000:d=1' -c:a pcm_s16le dropout-right.wav
```

| File | Clicks? | Dropouts? | Expected location |
| --- | --: | --: | --- |
| `clean.wav` | 0 | 0 | None |
| `click-left.wav` | 1 | 0 | Channel 1 at 0.250 s, duration ≈0.021 ms |
| `dropout-right.wav` | 0 | 1 | Channel 2 at 0.550 s, duration 30 ms |

Open these in Audacity and zoom around the stated time to inspect the injected sample or silence. CSV exports contain `suspected_clicks`, `suspected_dropouts`, `click_locations` and `dropout_locations`. Locations use `ch2@0.550000s/0.030000s` (channel, start and duration), separated by semicolons. Blank counts mean unavailable; zero counts mean measured with no candidates.

Background on interpreting impulses and musical transients: [iZotope audio repair guide](https://downloads.izotope.com/guides/iZotope-AudioRepairAndEnhancement.pdf). The detector here is an independent heuristic, not that product's algorithm.

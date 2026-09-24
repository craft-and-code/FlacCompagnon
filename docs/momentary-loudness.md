# Maximum momentary loudness (LUFS-M max)

The **LUFS-M max** column follows LUFS and reports the loudest complete **400 ms** window in a file. Its cell contains only the value, to one decimal place; the header identifies the unit. Hovering shows the window's start and end times. This is useful for locating brief loud passages within a programme whose integrated level may be much lower.

## Calculation

The measurement reuses the [integrated loudness](integrated-loudness.md) meter's K-weighted power. For each decoded frame, it sums the latest `round(sample_rate × 0.4)` powers and divides by that window length. Mono and stereo channels have unit weights and their powers are added. The complete-window loudness is:

```text
M = −0.691 + 10 × log10(mean K-weighted channel-summed power)
```

The largest power sum determines the maximum and its location. A larger value is louder: −10 LUFS exceeds −20 LUFS. An exact power tie retains the first location. The start time is `(decoded_frames − window_frames) / sample_rate`, with decoded frames counted at the end of the candidate window.

The window length and ungated measurement follow [EBU Tech 3341, sections 2.1–2.2](https://tech.ebu.ch/docs/tech/tech3341.pdf). No absolute or relative silence gate is applied to M. Quiet finite audio below −70 LUFS can therefore have a reading even when integrated LUFS is unavailable.

Every complete sample-aligned window is examined, including one ending on the last real frame between the integrated meter's 100 ms updates. No partial startup window or synthetic trailing silence contributes. The 1.5 seconds appended internally for LRA are excluded by capturing the maxima first.

## Availability and interpretation

- Mono or stereo at **8–768 kHz** is supported. Decoded DSD PCM uses the same path. Other layouts remain unavailable until reliable speaker positions and channel weights are supplied.
- Audio shorter than one complete 400 ms window has no M maximum. Window lengths are rounded to whole frames at unusual sample rates.
- Digital silence has zero power and mathematically −∞ loudness. The app displays a dash instead of inventing a finite LUFS value. Decode failures, invalid samples, skipped decoding and older reports also have no reading.
- The displayed value is the **maximum over the file**, not the current playback position. A sustained tone, a brief accent and the complete programme have different measurement scopes. Use [short-term loudness](short-term-loudness.md) for a longer local average.
- A momentary loudness maximum measures weighted average power, not sample peak or true peak. It does not by itself identify clipping or a defect.
- The window location identifies the measured interval, not the exact onset of an event. K-weighting has filter history; its initial state is zero at file start. Exporting a short selection and measuring it separately can change edge behaviour.

## Code, reports and table behaviour

`core/src/analysis/loudness_peaks.rs` holds M/S maxima and locations; `loudness.rs` supplies the shared weighted power and existing windows. Tracking maxima adds fixed-size state and no second filtering pass or additional sample buffer.

JSON stores `loudness_peaks.momentary` as `{ "lufs": number, "start_secs": number }`, or `null` when unavailable. The outer `loudness_peaks` field is optional for older reports. CSV exports `max_momentary_lufs` and `momentary_max_start_s` immediately after `integrated_lufs`. Values retain their stored precision; a missing value is blank and a measured zero remains zero.

The default order is **LUFS → LUFS-M max → LUFS-S max → LRA**. Existing saved column preferences insert these new columns after LUFS on first appearance. Subsequent user moves and visibility choices remain persistent. Sorting uses raw LUFS, with missing readings last in either direction; search includes the maximum value and its name.

## Verification

```sh
cargo test -p flaccompagnon-core loudness_peaks
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

Tests are separate from production code. They reproduce the independently specified levels and shifted-file cases from EBU Tech 3341, including all 20 offsets in case 13, which expose maxima missed by measuring only on a 100 ms grid. Additional tests cover quiet ungated signals, mono/stereo power, exact minimum duration, malformed tails, final windows and exclusion of LRA's artificial tail. A WAV integration test checks decoding, positions, CSV/JSON and older-report loading.

The optional reference test compares mixed-frequency signals with FFmpeg at 44.1, 48 and 96 kHz, in mono and stereo. [FFmpeg's ebur128 filter](https://ffmpeg.org/ffmpeg-filters.html#ebur128) logs M/S values at 10 Hz; the comparison uses sustained levels whose maxima are represented on that grid. For short transients, taking the maximum of those logs can under-read an every-frame maximum.

## Manual fixture

Generate a 400 ms stereo 1 kHz burst at −23 dBFS per channel, beginning off the 100 ms grid:

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=if(between(t\,2.04\,2.44)\,0.0707945784\,0)*sin(2*PI*1000*t)|if(between(t\,2.04\,2.44)\,0.0707945784\,0)*sin(2*PI*1000*t):s=48000:d=6' -c:a pcm_s24le momentary.wav
ffmpeg -hide_banner -nostats -i momentary.wav -af ebur128 -f null -
```

Expect **LUFS-M max ≈ −23.0**, with a maximum window near 2.04–2.44 s. **LUFS-S max** should be approximately **−31.8**, because 400 ms of tone occupies only part of a 3-second window: `−23 + 10 log10(0.4 / 3) ≈ −31.75`. FFmpeg's individual M log entries may be lower because of their update grid. In Audacity, create a stereo tone with a linear amplitude of `0.0707945784`, retain 400 ms and add silence around it to reproduce this comparison.

# Integrated loudness (LUFS)

The LUFS column measures programme loudness according to the gated, K-weighted integrated-loudness method of ITU-R BS.1770 and EBU R 128. It is the measurement to use for broad level comparison between programmes, rather than peak or RMS alone. The following columns, [LUFS-M max](momentary-loudness.md) and [LUFS-S max](short-term-loudness.md), report the loudest 400 ms and 3-second windows separately.

## Calculation

FlacCompagnon streams each mono or stereo channel through the BS.1770 K-weighting filters: a high-frequency shelf followed by a high-pass filter. The published 48 kHz filter coefficients are retained exactly at that rate. At other rates, the meter recovers the analogue pole frequency from each reference denominator and uses a frequency-prewarped bilinear transformation. The RLB high-pass numerator remains the published unit second difference `[1, −2, 1]`; only its poles are retuned. This avoids the shelf shift and unintended RLB gain change caused by a plain sample-rate ratio at low rates. The −0.691 loudness offset is unchanged; no empirical calibration factor is added.

The meter forms complete 400 ms blocks with nominal 100 ms hops (75% overlap). Window lengths round to the nearest sample; hops round down to whole samples, giving at least 10 updates per second at unusual rates such as 11025 Hz. A trailing incomplete block is discarded. It then applies two gates:

| Gate     | Rule                                                                           |
| -------- | ------------------------------------------------------------------------------ |
| Absolute | Keep blocks above −70 LUFS                                                     |
| Relative | From those blocks, keep values above the absolute-gated power mean minus 10 LU |

Both means are calculated in linear power, not by averaging decibels. Gate comparisons are strict: a block exactly at a threshold is excluded.

The final result is `−0.691 + 10 × log10(mean gated power)`. The −0.691 offset and both gates come from BS.1770. A file has no result when it is silent, shorter than one complete 400 ms window, malformed during decoding, outside 8–768 kHz, or has a layout other than mono or stereo.

## Interpretation

LUFS is programme loudness after frequency weighting and gating. A more negative value is quieter. It is not a peak measurement and does not identify clipping, dynamic range or source quality. Compare values measured with the same standard and scope; a short excerpt and an entire album track are different programmes.

The result currently gives each mono or stereo channel unit weight. Mono is measured as one channel; there is no implicit dual-mono playback compensation, so duplicating it into identical stereo raises loudness by approximately 3.01 LU. Multichannel layouts are intentionally unavailable until the decoder can supply reliable speaker positions and LFE roles, rather than silently applying incorrect weighting.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::loudness
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
```

The second command needs FFmpeg on `PATH`; it compares mixed-frequency signals at 8, 11.025, 16, 32, 44.1, 48 and 96 kHz in mono and stereo against FFmpeg's EBU R128 filter with defined tolerances.

Unit tests also check the published EBU Tech 3341 integrated cases 1–5, absolute/relative gate boundaries, frequency responses against fixed FFmpeg metadata readings, stable poles across 8–768 kHz, and recovery of quiet windows after a large finite float-PCM passage. These synthetic checks do not certify every programme or sample rate.

Create a nominal −23 LUFS stereo sine fixture:

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.0707945784*sin(2*PI*1000*t)|0.0707945784*sin(2*PI*1000*t):s=48000:d=60' -c:a pcm_s24le loudness.wav
ffmpeg -i loudness.wav -af ebur128 -f null -
```

Import `loudness.wav`; FlacCompagnon should report approximately −23.0 LUFS. The FFmpeg command is an independent comparison, but small differences in reporting and rounding do not constitute formal certification.

References: [ITU-R BS.1770-5](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf), [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf), [FFmpeg ebur128 filter](https://ffmpeg.org/ffmpeg-filters.html#ebur128).

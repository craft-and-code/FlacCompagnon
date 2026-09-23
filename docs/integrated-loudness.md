# Integrated loudness (LUFS)

The LUFS column measures programme loudness according to the gated, K-weighted integrated-loudness method of ITU-R BS.1770 and EBU R 128. It is the measurement to use for broad level comparison between programmes, rather than peak or RMS alone.

## Calculation

FlacCompagnon streams each mono or stereo channel through the BS.1770 K-weighting filters: a high-frequency shelf followed by a high-pass filter. The published 48 kHz filter coefficients are retuned from their analogue prototype for the decoded sample rate, preserving the reference response at 48 kHz and tracking it at other supported rates.

The meter forms 400 ms blocks every 100 ms, which is 75% overlap. It then applies two gates:

| Gate     | Rule                                                              |
| -------- | ----------------------------------------------------------------- |
| Absolute | Keep blocks above −70 LUFS                                        |
| Relative | From those blocks, keep values above the ungated mean minus 10 LU |

The final result is `−0.691 + 10 × log10(mean gated power)`. The −0.691 offset and both gates come from BS.1770. A file has no result when it is silent, shorter than one complete 400 ms window, malformed during decoding, outside 8–768 kHz, or has a layout other than mono or stereo.

## Interpretation

LUFS is programme loudness after frequency weighting and gating. A more negative value is quieter. It is not a peak measurement and does not identify clipping, dynamic range or source quality. Compare values measured with the same standard and scope; a short excerpt and an entire album track are different programmes.

The result currently gives each mono or stereo channel unit weight. Multichannel layouts are intentionally unavailable until the decoder can supply reliable speaker positions and LFE roles, rather than silently applying incorrect weighting.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::loudness
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
```

The second command needs FFmpeg on `PATH`; it compares mixed-frequency signals at 44.1, 48 and 96 kHz in mono and stereo against FFmpeg's EBU R128 filter with defined tolerances.

Create a nominal −23 LUFS stereo sine fixture:

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.0707945784*sin(2*PI*1000*t)|0.0707945784*sin(2*PI*1000*t):s=48000:d=60' -c:a pcm_s24le loudness.wav
ffmpeg -i loudness.wav -af ebur128 -f null -
```

Import `loudness.wav`; FlacCompagnon should report approximately −23.0 LUFS. The FFmpeg command is an independent comparison, but small differences in reporting and rounding do not constitute formal certification.

References: [ITU-R BS.1770-5](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf), [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf), [FFmpeg ebur128 filter](https://ffmpeg.org/ffmpeg-filters.html#ebur128).

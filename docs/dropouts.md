# Suspected dropout detection

The Dropouts column counts short exact-zero gaps bounded by active decoded PCM. It is designed to find a digital-silence shape that can indicate missing audio. It is not a verdict about the cause: edits, deliberate mutes and synthetic material can contain the same shape.

## Signal tested

Each channel is tested independently for an exact-zero run lasting 2–250 ms. Both run boundaries must jump by at least 0.02 full scale. The real 5 ms contexts on both sides must have RMS above −60 dBFS. To avoid treating a legitimate transition into silence as a dropout, the resumed RMS must lie within about 12 dB of the preceding RMS and at least 90% of the resumed context must be nonzero.

Leading and trailing silence, fades and contexts that cannot supply enough real samples are excluded. The analyzer does not pad audio with zeros to manufacture a context.

## Result and limits

The app shows the count only: `0`, a positive integer, or `—` when unavailable. Hovering supplies channel, start time and duration. At most 32 locations are retained in reports, but the count remains complete. Simultaneous left and right gaps are two channel events.

The detector supports 1–32 channels at 8–768 kHz. Invalid samples, very short streams and DSD yield no result. A gap filled with dither or low-level noise is not exact zero and will not be caught. Longer missing passages, gradual fades and dropouts obscured by lossy decoding can also be missed.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)|if(between(n\,26400\,27839)\,0\,0.1*sin(2*PI*317*t)):s=48000:d=1' -c:a pcm_s16le dropout-right.wav
```

Import `dropout-right.wav`. It should report one dropout on channel 2 around 0.550 s, lasting 30 ms. Zoom around that time in Audacity to inspect the exact-zero interval.

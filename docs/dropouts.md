# Suspected dropout detection

The Dropouts column counts short exact-zero gaps bounded by active decoded PCM. It is designed to find a digital-silence shape that can indicate missing audio. It is not a verdict about the cause: edits, deliberate mutes and synthetic material can contain the same shape.

## Signal tested

Each channel is tested independently for an exact-zero run lasting 2–250 ms, including both limits. The minimum is rounded up and the maximum down to whole samples. Both run boundaries must jump by at least 0.02 full scale. The real contexts on both sides contain `ceil(sample_rate / 200)` samples, or at least 5 ms, and must have RMS at least −60 dBFS. The resumed-to-preceding mean-square ratio must lie between `1/16` and `16`, including both limits (about ±12.04 dB), and at least 90% of the resumed context must be nonzero.

Leading and trailing silence and contexts that cannot supply enough real samples are excluded. Smooth fades with subthreshold edges are rejected, but a sufficiently fast, loud fade around silence can pass. The analyzer does not pad audio with zeros to manufacture a context.

## Result and limits

The app shows the count only: `0`, a positive integer, or `—` when unavailable. Hovering supplies channel, start time and duration. At most 32 locations are retained in reports, but the count remains complete. Simultaneous left and right gaps are two channel events.

The detector supports 1–32 channels at 8–768 kHz. Invalid samples, very short streams and DSD yield no result. A gap filled with dither, DC bias or low-level noise is not exact zero and will not be caught. Longer missing passages, gradual fades and dropouts obscured by lossy decoding can also be missed. Finite float samples outside ±1 are tested without clamping; the absolute edge threshold therefore depends on gain.

This analyzes the decoded waveform, without observing playback underruns or the recording's transport history. Missing packets need not become zeros: [Opus specifies packet-loss concealment](https://www.rfc-editor.org/rfc/rfc6716.html#section-4.4), for example. The count alone cannot distinguish a defect from an intentional mute, or establish audibility.

## Accuracy, cost and alternatives

The rolling context energy uses a [compensated sum](https://doi.org/10.1002/zamm.19740540106), with separate additions and removals. This retains ordinary-level context energy after an extreme finite float sample leaves the window. Work is linear in decoded samples; the rolling history has `ceil(sample_rate / 200)` double-precision values per channel and does not grow with file duration. The combined buffers of both discontinuity detectors are described in [Impulses](impulses.md).

A noise-tolerant gap detector would need a different definition and a measured noise floor; it would also classify some quiet musical passages as gaps. Predictive methods discussed in [Impulses](impulses.md) could test discontinuities without requiring exact zeros, but would add model assumptions and cost. Neither alternative is implemented. Threshold tuning needs labelled recordings with deliberate mutes, fades, quiet passages and real faults, plus listening tests; synthetic fixtures establish the implemented boundaries rather than a universal detection rate.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)|if(between(n\,26400\,27839)\,0\,0.1*sin(2*PI*317*t)):s=48000:d=1' -c:a pcm_s16le dropout-right.wav
```

Tests cover both duration limits at several sample rates, channel independence, context edges, RMS-ratio and nonzero-percentage limits, noise-filled gaps, fast deliberate fades and large float samples through WAV decoding.

Import `dropout-right.wav`. It should report one dropout on channel 2 around 0.550 s, lasting 30 ms. Zoom around that time in Audacity to inspect the exact-zero interval.

# Suspected impulse detection

The Impulses column counts short isolated pulse candidates in decoded PCM and stores a limited set of locations to audition. It is a restoration cue, not a corruption verdict: percussion, edits, intentionally generated pulses and codec artefacts can satisfy a sample-domain click rule.

## Signal tested

Each decoded channel is examined separately. A candidate is an isolated pulse lasting at most 0.5 ms whose entry and exit edges largely cancel. Both edges must be at least 0.05 full scale and at least eight times the RMS of first differences in the real 5 ms context before and after the pulse. The mismatch between the opposing edges must be no more than 25% of the larger edge.

These are project thresholds, not an audio standard or an audibility model. At sample rate `r`, each context has `ceil(r / 200)` first differences and the maximum pulse width is `floor(r / 2000)` samples. Limits follow the original sample grid without resampling. Entry/exit differences are excluded from the background RMS; the actual candidate width determines its required right context.

After a candidate, edges occurring within 1 ms are grouped into the same cue. This avoids turning a short multi-sample disturbance into a sequence of displayed click locations.

## Result and limits

The app shows the count only: `0`, a positive integer, or `—` when unavailable. Hovering supplies timestamps, channel numbers and durations. The detector retains only the first 32 locations of this kind for reports, ordered by time and channel, while the count itself continues beyond that cap. Simultaneous candidates on two channels count separately.

No result is published for invalid streams, streams shorter than `2 × ceil(r / 200) + 3` frames (483 frames, or 10.0625 ms, at 48 kHz), unsupported 1–32 channel/sample-rate bounds (8–768 kHz), or DSD. DSD's decoded PCM representation and conversion filter alter precisely the pulse shape the rule tests. No artificial silence is padded at file boundaries, so events without real context are not assessed. Even a one-sample pulse needs both edges and the full real context.

Quiet, wide, masked or filtered clicks can be missed. A zero count does not prove that a recording has no audible discontinuity. Inspect the listed position in an editor before repair.

Changing polarity or adding a constant offset preserves the first differences. Gain preserves the relative edge/context ratio, but a quieter event can fall below the absolute 0.05 floor. Finite float samples above nominal full scale are not clamped. Nearby pulses can raise each other's context RMS and escape the rule. The 1 ms grouping counts listening cues, not every changed sample.

## Accuracy, cost and alternatives

Compensated energy prefixes preserve quiet context after a very large float sample. They are calculated only when a batch contains an eligible entry edge, with buffers reused across the 10 ms processing batches. Memory depends on sample rate and channel count, not recording length. With both discontinuity checks enabled, the main numeric buffers occupy about 50 KiB for stereo at 48 kHz and under 13 MiB at 768 kHz / 32 channels, excluding decoder and allocator overhead. Buffer scans and movement are linear in frame count, with up to `floor(r / 2000)` exit candidates for each sufficiently large entry edge. This is a resource bound, not a measured whole-app throughput claim.

Median/MAD detectors such as [Hampel](https://www.mathworks.com/help/signal/ref/hampel.html) reject isolated outliers robustly, but small windows can also identify waveform extrema. [Median-based audio restoration](https://www.dafx.de/paper-archive/2013/papers/06.dafx2013_submission_46.pdf) likewise needs careful transient discrimination. [Sparse linear prediction](https://ftp.esat.kuleuven.be/pub/stadius/vanwaterschoot/reports/dufera2019.pdf) models tonal context but adds fitting and iterative optimization. These are possible future comparators, not implemented replacements.

[Listening-test research](https://link.springer.com/article/10.1186/s13636-024-00389-9) compares auditory and wavelet models using perceptible clicks. Its results are not an accuracy estimate for this detector. [Music-context research](https://archives.ismir.net/ismir2021/paper/000095.pdf) also treats intentional clicks as a classification challenge. A labelled music corpus and listening evaluation are needed before selecting a replacement or reporting false-positive rates.

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

Automated fixtures specify the injected samples, times and channel independently. They cover exact edge/context and width limits, polarity/DC controls, 8–768 kHz, 32 channels, shortest valid clips, file and batch boundaries, extreme float prefixes and the 32-location cap. Clean tones, square waves, noise and damped attacks are controls; they do not represent every musical transient. Integration tests also exercise WAV decoding and selected-analysis reporting.

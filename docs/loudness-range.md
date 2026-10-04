# Loudness range (LRA)

The LRA column measures how much short-term loudness varies across a programme. It follows EBU Tech 3342 and is expressed in LU. It is complementary to integrated LUFS: two tracks can have the same integrated loudness and very different loudness variation. [LUFS-S max](short-term-loudness.md) exposes the highest ungated short-term reading separately, before LRA's artificial tail is added.

## Calculation

LRA reuses the same K-weighted channel power as [Integrated loudness](integrated-loudness.md), but creates 3-second short-term windows every 100 ms. In file mode, the meter follows Tech 3342’s reference-file convention by appending at least 1.5 seconds of analysis-only silence, rounded upward to whole frames. The final window’s centre lies within one update of the real end; an arbitrary file ending need not align exactly with the update grid. The filter processes that silence, so its existing tail is retained.

The calculation keeps short-term windows at or above −70 LUFS, computes their mean in linear power, then keeps values at or above that mean minus 20 LU. From the remaining values it selects the 10th and 95th percentiles and returns their difference in LU. For `n` retained readings, the ascending zero-based ranks are `round((n − 1) × 0.10)` and `round((n − 1) × 0.95)`; there is no interpolation between readings. LRA includes equality at either gate, unlike integrated LUFS.

| Property          | Setting                                 |
| ----------------- | --------------------------------------- |
| Short-term window | 3 seconds                               |
| Update interval   | 100 ms or more often at supported rates |
| Absolute gate     | −70 LUFS                                |
| Relative gate     | Mean minus 20 LU                        |
| Range             | 95th percentile minus 10th percentile   |

The same availability rules apply as LUFS: only valid mono or stereo streams at 8–768 kHz are measured. A stream shorter than one full 3-second window, silence, invalid samples or unrepresentable internal power produces no reading.

The histories discard readings that can never pass the absolute gate. LRA reuses its retained vector and selects two ranks without a full sort or a second programme-sized allocation. The shared 400 ms/3 s rings occupy about 11.1 MiB at 768 kHz; qualifying gate histories grow by approximately 160 bytes per programme second (before vector capacity overhead). M/S maxima add fixed-size state. Compensated rolling sums preserve quiet-window power after a large finite float passage; this does not provide arbitrary precision.

## Interpretation and limits

A larger LRA means greater variation among gated short-term loudness values. It is not a peak-to-RMS ratio, so it is not the `DR` column. It does not decide whether compression is artistically appropriate or whether a track will feel loud in every listening context.

EBU guidance treats programmes shorter than 60 seconds as an approximation for LRA. The app marks such readings accordingly. A constant five-second tone is therefore a poor fixture for demanding exactly zero LRA: the end windows and their gates can materially affect a very short programme.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::loudness
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
```

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=if(lt(t\,20)\,0.1\,0.0316227766)*sin(2*PI*1000*t)|if(lt(t\,20)\,0.1\,0.0316227766)*sin(2*PI*1000*t):s=48000:d=40' -c:a pcm_s24le range.wav
ffmpeg -i range.wav -af ebur128 -f null -
```

The unit suite checks the four published EBU Tech 3342 level distributions, power-mean gating, equality at the absolute gate, percentile exclusion, repeated observations, silence and odd-rate trailing padding. The optional FFmpeg test uses three-decimal metadata at several rates. FFmpeg and this meter can differ in percentile sampling and tail conventions; the LRA comparison uses Tech 3342’s ±1 LU acceptance tolerance.

`range.wav` should have an LRA of about 10 LU, with the under-60-second approximation mark. Compare it with FFmpeg as a reference rather than expecting bit-for-bit identical display rounding.

References: [EBU Tech 3342](https://tech.ebu.ch/docs/tech/tech3342.pdf), [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf), [EBU Tech 3343 short-programme guidance](https://tech.ebu.ch/docs/tech/tech3343.pdf).

# High-frequency stereo narrowing

The `HF Stereo` column measures how much Side energy remains in the high-frequency band compared with Mid energy. It reports a conservative cue when a stereo programme is wide in the upper-mid reference band but remains much narrower above 6 kHz. This experimental pattern can result from intensity stereo or deliberate mixing, but it does not identify a codec and does not prove an audible defect.

## Principle

For each stereo frame, FlacCompagnon derives:

```text
Mid  = (Left + Right) / 2
Side = (Left − Right) / 2
```

A high Side/Mid ratio indicates a wide stereo difference; a more negative ratio indicates a narrower image. The meter filters Mid and Side through matching filters, using eighth-order Butterworth high-pass and low-pass sections at each band corner, then measures their energies in non-overlapping 500 ms blocks. The high band ends at 20 kHz; when Nyquist is at or below 20 kHz, no upper filter is needed. Filter transitions are gradual, so these are nominal bands, not exact spectral cutouts. The coefficients follow the LPF/HPF equations in the [W3C Audio EQ Cookbook](https://www.w3.org/TR/audio-eq-cookbook/).

| Band      | Role                                                                  |
| --------- | --------------------------------------------------------------------- |
| 1.5–5 kHz | Reference: establishes that the programme has meaningful stereo width |
| 6–20 kHz  | High band, limited by Nyquist at lower sample rates                   |

A block is eligible only when Mid RMS exceeds −60 dBFS in both bands. A block is called narrowed when high-band Side/Mid is at or below −20 dB, the reference Side/Mid is at or above −12 dB, and the difference between them is at least 12 dB. The file receives the narrowing cue only when at least 70% of three or more eligible blocks satisfy the rule **and** the ratios of summed eligible energies satisfy the same three thresholds. Blocks need not be consecutive; the fraction describes eligible audio, not a continuous duration.

The displayed number is `10 log10(sum(Side energy) / sum(Mid energy))` across eligible complete blocks, in dB. Quiet blocks and the trailing partial block are excluded. Zero or extremely weak Side is reported at a relative floor of −120 dB, which is a display convention, not a measured noise floor. Hovering also shows the reference ratio and the fraction of qualifying blocks.

All thresholds are project heuristics, not values prescribed by MPEG, ITU or EBU. Synthetic tests validate their implementation; sensitivity and false-positive rates on a labelled corpus of music have not been established.

## Interpretation and limits

The ITU overview explains that intensity stereo reduces directional information while retaining energy envelopes. M/S coding is a separate sum/difference representation and does not inherently remove Side energy. [ITU HSTP-MCTA, §§7.4 and 7.5.2](https://www.itu.int/dms_pub/itu-t/opb/tut/T-TUT-IPTV-2009-MCTA-PDF-E.pdf)

A low Side/Mid ratio is only an indirect cue. Unequal channel gains or polarity can leave substantial Side energy even when the high-band channels share one signal. A negative result cannot rule out intensity stereo; identifying a codec tool requires evidence beyond this measurement.

Naturally centre-heavy mixes, deliberately narrow high-frequency material, mono ambience and later mastering can have the same shape. The reference-band requirement prevents a uniformly narrow recording from being labelled, but cannot eliminate every artistic case. A negative result means only that this defined persistent pattern was not measured.

The measurement is available only for valid two-channel streams at 24–768 kHz with at least three qualifying half-second blocks. Mono, multichannel, silent, very short, invalid and old saved reports show `—`.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::intensity_stereo
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

The tests also cover bass leakage into the reference, ultrasonic contamination, mono ratio invariance, persistence, invalid tails, seven sample rates from 24 to 768 kHz, and decoding a WAV through report export. The Rust tests construct Mid and Side tones independently: a 3 kHz reference with equal Mid and Side energy, paired with either a 9 kHz Side signal 40 dB below Mid or a wide 9 kHz control. This gives ground truth without using the meter’s own result as its oracle.

For a manual fixture, create a wide upper-mid reference and a strongly narrowed high band, then compare it with a wide-band control in the app:

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*3000*t)+0.1*cos(2*PI*3000*t)+0.101*sin(2*PI*9000*t)|0.1*sin(2*PI*3000*t)-0.1*cos(2*PI*3000*t)+0.099*sin(2*PI*9000*t):s=48000:d=4' -c:a pcm_s24le hf-narrow.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*3000*t)+0.1*cos(2*PI*3000*t)+0.1*sin(2*PI*9000*t)+0.1*cos(2*PI*9000*t)|0.1*sin(2*PI*3000*t)-0.1*cos(2*PI*3000*t)+0.1*sin(2*PI*9000*t)-0.1*cos(2*PI*9000*t):s=48000:d=4' -c:a pcm_s24le hf-wide.wav
```

`hf-narrow.wav` is a controlled signal for the column and should show a markedly more negative `HF Stereo` value and the narrowing cue. It is not a realistic music corpus or a reference for codec quality.

# Stereo balance measurement

The balance column reports the whole-track RMS level difference between the two decoded stereo channels. It is descriptive: panning, microphone placement, mastering decisions and DC offset can all create a non-zero result without a recording defect.

## Calculation

The analyzer accumulates unweighted energy for both channels. Because RMS is the square root of energy divided by the same frame count, the difference is calculated as:

```text
right minus left = 10 × log10(E_R / E_L) dB
```

This follows the [ordinary RMS definition](https://www.mathworks.com/help/dsp/ref/rms.html). In code, the two logarithms are subtracted instead of dividing the energies first, so very unequal finite energies cannot overflow the ratio. Common gain or reversing either channel's polarity leaves balance unchanged; swapping channels reverses its sign. There is no minimum duration or absolute-level gate, so even a short, quiet, nonzero float signal has a measurement.

A positive result means the right channel is louder; a negative result means the left channel is louder. The display uses the more readable form `R +x.x dB` or `L +x.x dB`. If every sample in one channel is zero, the result is `L silent` or `R silent`; an infinite dB value is never exported. Both channels silent, invalid values and layouts other than exactly two channels have no reading.

CSV keeps the unambiguous signed form in `balance_right_minus_left_db`, plus `balance_silent_channel` for the two silent states.

## Limits

This is a broad-band, unweighted, whole-track RMS measurement. It does not model human loudness, does not establish channel assignment, and does not distinguish an intentional asymmetric mix from a wiring problem. A short loud event can dominate energy, while a frequency-specific imbalance may not be obvious in the single number.

DC is deliberately included: `mean square = variance + mean²`. Equal AC energy with different constant channel biases can therefore give a nonzero balance. That is raw signal balance, not an AC-only level comparison. Consult [DC offset](dc-offset.md) separately; removing DC here would silently change the quantity reported. Likewise, replacing it with a loudness difference would require a separate definition of channel weighting, duration and gating, as distinct from [ITU-R BS.1770-5 programme loudness](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf).

Equal RMS does not imply identical channels, equal perceived loudness, or mono cancellation. Those questions need the waveform relationship, frequency content and the [phase measurements](local-phase.md). This column has no good/bad balance threshold. Accumulation requires constant memory and one energy sum per channel in the existing decode pass; the two logarithms are evaluated once at the end.

## Tests and manual fixtures

```sh
cargo test -p flaccompagnon-core analysis::stereo
cargo test -p flaccompagnon-core --test stereo_balance --test stereo_validity
```

Analytical unit tests cover the amplitude/energy conversion, common gain, huge energy ratios, polarity-independent energy, raw DC and exact silence. Float WAV integration fixtures cover sixteen-frame excerpts from very quiet to above full scale, known constant biases and silent-channel states. Malformed-frame injection verifies that a valid prefix is withheld after invalid stereo input.

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.05*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le balance-left.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0:s=48000:d=5' -c:a pcm_s24le right-silent.wav
```

`balance-left.wav` should show `L +6.0 dB` (the exact amplitude-ratio result is 6.0206 dB). `right-silent.wav` should show `R silent` and no L/R correlation. For an independent numerical check, run `ffmpeg -i balance-left.wav -af astats -f null -` and compare the two channel RMS levels.

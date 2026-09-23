# Stereo balance measurement

The balance column reports the whole-track RMS level difference between the two decoded stereo channels. It is descriptive: panning, microphone placement, mastering decisions and DC offset can all create a non-zero result without a recording defect.

## Calculation

The analyzer accumulates unweighted energy for both channels. Because RMS is the square root of energy divided by the same frame count, the difference is calculated as:

```text
right minus left = 10 × log10(E_R / E_L) dB
```

A positive result means the right channel is louder; a negative result means the left channel is louder. The display uses the more readable form `R +x.x dB` or `L +x.x dB`. If every sample in one channel is zero, the result is `L silent` or `R silent`; an infinite dB value is never exported. Both channels silent, invalid values and layouts other than exactly two channels have no reading.

CSV keeps the unambiguous signed form in `balance_right_minus_left_db`, plus `balance_silent_channel` for the two silent states.

## Limits

This is a broad-band, unweighted, whole-track RMS measurement. It does not model human loudness, does not establish channel assignment, and does not distinguish an intentional asymmetric mix from a wiring problem. A short loud event can dominate energy, while a frequency-specific imbalance may not be obvious in the single number.

## Tests and manual fixtures

```sh
cargo test -p flaccompagnon-core analysis::stereo
```

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.05*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le balance-left.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0:s=48000:d=5' -c:a pcm_s24le right-silent.wav
```

`balance-left.wav` should show `L +6.0 dB` (the exact amplitude-ratio result is 6.0206 dB). `right-silent.wav` should show `R silent` and no L/R correlation. For an independent numerical check, run `ffmpeg -i balance-left.wav -af astats -f null -` and compare the two channel RMS levels.

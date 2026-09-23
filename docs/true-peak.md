# True peak measurement

True peak estimates the maximum of the waveform reconstructed between stored PCM samples. A sample peak below 0 dBFS can still reconstruct above full scale, which is called an inter-sample over. The result is reported in dBTP, where 0 dBTP is full scale.

## Calculation

Each decoded channel is oversampled by a factor of four using a 48-tap, Blackman-windowed sinc low-pass FIR. The streaming polyphase implementation has 12 taps per phase and evaluates four outputs for each input sample. The maximum absolute oversampled value over every channel becomes the true peak:

```text
dBTP = 20 × log10(oversampled peak)
```

Silence has no finite logarithmic peak and is represented internally by negative infinity. The measurement is deliberately separate from the clipping counter, which uses only stored samples.

## Display interpretation

The app colours the result as a delivery-headroom cue:

| dBTP               | Display meaning    |
| ------------------ | ------------------ |
| At or below −1     | Green headroom cue |
| Above −1 through 0 | Neutral            |
| Above 0 through +1 | Warning cue        |
| Above +1           | High-over cue      |

These colours help compare files; they are not a statement that every platform or codec has identical headroom requirements.

## Limits

The meter uses the BS.1770-style 4× oversampling approach and its own fixed 48-tap filter. It is not presented as formal conformance testing for every BS.1770 implementation, codec or DAC reconstruction filter. Different filters, sample-rate handling and end-of-stream treatment can differ slightly. True peak is a level estimate, not a clipping diagnosis or an audible-distortion score.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::truepeak
```

The unit suite includes a quarter-rate sine sampled away from its crest: its stored peak is below 0.70 while the 4× meter recovers a peak above 0.95. For a practical comparison, import a loud clipped fixture from [Clipping](clipping.md) and compare the clip event count with its dBTP value.

Reference: [ITU-R BS.1770-5](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf).

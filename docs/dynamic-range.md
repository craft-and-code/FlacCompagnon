# Dynamic range estimate

The `DR` column is a DR-meter-style crest-factor estimate: the file's sample peak compared with the RMS of its loudest sustained passages. It describes level dynamics, independently of losslessness, provenance and loudness normalisation.

## Calculation

The decoder accumulates the mean square of each frame into blocks of 131,072 frames, roughly three seconds at 44.1 kHz. A trailing partial block is included only when it contains at least one quarter of that size. At the end, blocks are sorted by energy; the loudest 20%, rounded up to at least one block, are averaged. The output is:

```text
DR = 20 × log10(sample peak / RMS of loudest blocks)
```

This follows the basic idea of comparing peak level with loud-passage RMS. It deliberately uses a fixed frame count, so the block duration varies with sample rate; at 96 kHz it is about 1.37 seconds rather than three.

## Interpretation

Higher values mean more crest factor in the selected loud passages. The interface uses a green cue at 12 dB and above and a warning cue below 8 dB. Values between these boundaries are descriptive. A high DR value does not prove that a master is preferable, and a low value does not prove clipping or poor audio quality.

No reading is produced for silence, a zero sample peak or an invalid loud-passage RMS. The ordinary sample peak is used, so this value should not be confused with the [true peak](true-peak.md) result.

## Limits

This is not the official output of a particular DR Meter release and not an EBU loudness-range measurement. Different block sizes, weighting, gating and peak definitions can produce different numbers. Short tracks have few blocks, and a loud isolated event can influence both peak and the loudest-block selection.

For perceived programme loudness use [Integrated loudness](integrated-loudness.md); for variation in loudness over time use [Loudness range](loudness-range.md).

## Tests

```sh
cargo test -p flaccompagnon-core analysis::analyzer
```

A useful manual comparison is a quiet sine and the same sine with a short high-level burst. The burst should increase sample peak much more than the loudest-block RMS, increasing the displayed DR. This demonstrates the defined calculation rather than an external standard.

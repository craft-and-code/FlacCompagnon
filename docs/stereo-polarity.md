# Stereo polarity analysis

The polarity indication compares the two decoded stereo channels over the whole file. It flags a likely L/R polarity inversion when they are strongly opposed. It cannot determine absolute polarity, and it cannot decide whether an intentional artistic phase relationship is a defect.

## Calculation

For exactly two channels, FlacCompagnon accumulates left energy `E_L`, right energy `E_R` and cross energy `C = Σ(L×R)`. The reported correlation is:

```text
correlation = C / sqrt(E_L × E_R)
```

It is clamped to the interval −1 through +1. A value at or below −0.95 produces the polarity indication. The threshold accepts relationships such as `R = −kL`, where different channel gains prevent complete cancellation in a mono fold-down.

No correlation is reported when either channel is silent, either energy is invalid, or the file is not exactly stereo.

## Interpretation

A value near +1 means the channels move together, a value near 0 means little whole-track linear correlation, and a value near −1 means they move in opposite directions. The indication is deliberately conservative: values above −0.95 are still useful information for mono compatibility, but are not labelled as an inversion by the app.

The result is whole-track average behaviour. A brief inverted section can be diluted by the rest of a track, while intentional spatial effects, stereo ambience and phase processing can make correlation a poor quality judgement. The separate [local and frequency-band phase analysis](local-phase.md) measures short windows and four frequency ranges to reveal opposition hidden by this average. Listen in mono and inspect the source before changing audio.

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::stereo
```

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|-0.1*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le polarity.wav
```

Import `polarity.wav`: the correlation should be approximately −1 and the polarity indication should appear. Open it in Audacity, invert one channel, then export a comparison file to verify that the indication disappears.

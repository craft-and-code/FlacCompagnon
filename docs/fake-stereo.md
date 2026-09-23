# Fake-stereo check

The fake-stereo indication identifies a two-channel file whose left and right channels are effectively the same signal. It is a channel-relationship measurement, not an authenticity verdict: dual mono can be intentional for mono masters, spoken-word releases, archival transfers and some production choices.

## Calculation

The analyzer accumulates left energy, right energy and the energy of the L−R difference during the decoded pass. It returns a fake-stereo indication for a stream with at least two channels when either condition holds:

| Condition                                      | Meaning                                                 |
| ---------------------------------------------- | ------------------------------------------------------- |
| Every L/R frame is bit-identical               | Exact dual mono                                         |
| L−R energy is more than 60 dB below L+R energy | The stereo difference is negligible over the whole file |

A fully silent stream is not called fake stereo. For files with more than two channels, the decision still uses the first two decoded channels; it does not make a claim about a surround layout.

## Interpretation and limits

The indication means the channel relationship is effectively mono according to the thresholds above. It does not say that the source was fraudulently made stereo, that a mono release is defective, or that a small stereo difference is perceptually meaningful. Correlated stereo, centre-heavy mixes and low-level ambience can move the result around the 60 dB engineering threshold.

For an unequal but sign-reversed relationship, use [Stereo polarity](stereo-polarity.md). For a finite gain difference, use [Stereo balance](stereo-balance.md).

## Tests and manual fixture

```sh
cargo test -p flaccompagnon-core analysis::stereo
```

Create an exact dual-mono file and an obviously different stereo file, then import both:

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.1*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le dual-mono.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.1*sin(2*PI*1700*t):s=48000:d=5' -c:a pcm_s24le stereo.wav
```

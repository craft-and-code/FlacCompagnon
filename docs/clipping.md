# Sample-domain clipping

The clipping analysis finds runs of stored PCM samples at or very near full scale. It describes clipping in the file's sample values. It is separate from [true peak](true-peak.md), which estimates overshoot between samples after reconstruction.

## Calculation

Every decoded sample in every channel is tested against an absolute normalized threshold of `0.9997`. The analyzer counts all samples at or above that magnitude. A run becomes one clip event only when it reaches three consecutive threshold-crossing samples; longer runs remain one event. This prevents a single legitimate peak sample from being labelled as a clipped plateau.

The result contains a boolean clipping indication, the number of events, the number of clipped samples, and the ordinary sample peak. The event stream follows the decoded interleaved sample order, so simultaneous full-scale runs in separate channels are represented by the actual stored sequence rather than guessed as one acoustic event.

## Interpretation and limits

A clip event is strong evidence of a sustained full-scale plateau in decoded PCM. It does not say whether the plateau is audible, unwanted, created during recording or deliberately used as an effect. The threshold is an engineering tolerance for normalized decoding, not a universal mastering rule. Brief clipping shorter than three samples is counted as near-full-scale samples but does not create an event.

Conversely, a file with no sample-domain clipping can still have a positive inter-sample true peak. A file with clipped samples can have a true peak whose size depends on reconstruction filtering. Read both values when assessing headroom.

## Tests and manual fixtures

```sh
cargo test -p flaccompagnon-core analysis::clipping
```

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.5*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le clean-level.wav
ffmpeg -n -f lavfi -i 'aevalsrc=clip(1.5*sin(2*PI*1000*t)\,-1\,1):s=48000:d=5' -c:a pcm_s24le clipped-level.wav
```

Import both files. The second should report clipping events and many clipped samples. Zoom into its waveform in Audacity to see the flat peaks.

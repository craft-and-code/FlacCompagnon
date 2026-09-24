# DC offset

The `DC (%)` column measures the average displacement of decoded audio from zero. It shows the largest absolute channel mean as a percentage of full scale, with three decimal places. Hovering shows each channel's signed mean, in decoded channel order. For example, channel means of +1% and −2% produce a cell value of `2.000`.

## Calculation

For each channel, using every decoded sample in the file:

```text
mean[channel] = sum(samples[channel]) / frame_count
max_abs       = max(abs(mean[channel]))
display       = 100 × max_abs
```

Samples use normalized amplitude: `+1.0` is positive full scale, so `0.01` is 1%. The calculation includes silence and the final frame. There is no frequency weighting, gate, resampling, filtering or minimum duration. Each channel has its own compensated double-precision sum to preserve small contributions when large positive and negative samples cancel. Opposite offsets in different channels never cancel through a mono downmix.

This follows the arithmetic-mean interpretation of DC offset described in the [Audacity manual](https://manual.audacityteam.org/man/dc_offset.html) and [FFmpeg's DC correction documentation](https://ffmpeg.org/ffmpeg-filters.html#dynaudnorm). A persistent displacement consumes headroom on one side of the waveform.

## Availability and interpretation

- Mono, stereo and multichannel streams with 1–32 decoded channels are supported, independently of sample rate.
- Digital silence measures zero; an empty stream has no reading. Constant nonzero samples are measured even without an audible alternating signal.
- Any malformed frame or non-finite sample withholds the entire reading. Failed or skipped decoding and older reports show `—`.
- For DSD decoded with FFmpeg, the value describes the resulting PCM stream and may depend on the conversion filters. Header-only DSD analysis has no DC reading.
- Very short excerpts, incomplete low-frequency periods and asymmetric edits can have a nonzero mean. Unequal positive and negative peaks alone do not imply a nonzero mean.
- A whole-file mean does not locate changes over time. Equal positive and negative bias in different parts of a track can cancel; silence can dilute a localized offset.
- `0.000` is rounded display precision, not proof of exact zero. Signed tooltip values suppress negative zero after rounding.
- Float audio outside the nominal ±1 range is measured without clipping or clamping, so a result can exceed 100%.

This is a descriptive measurement with no automatic warning threshold. It does not establish hardware failure, audibility, or lossless provenance. It does not change the three authenticity verdicts or modify audio files.

## Reports

JSON stores `dc_offset.channel_means` and `dc_offset.max_abs` in normalized amplitude, without display rounding. Older JSON files without `dc_offset` still load and need reanalysis to obtain the measurement.

CSV exports `dc_offset_max_abs` and `dc_offset_channel_means`, also in normalized amplitude rather than percent. Channel means are separated by semicolons in decoded order, for example `0.01;-0.02`. Both fields are blank when unavailable; a measured zero is exported as `0`.

## Automated verification

```sh
cargo test -p flaccompagnon-core dc_offset
cargo test -p flaccompagnon-core --test dc_offset -- --ignored
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

The ordinary tests inject exact integer or binary-fraction offsets into balanced signals. They check channel independence, silence, asymmetric zero-mean waves, partial periods, invalid tails, large float samples, mono/stereo/multichannel WAV decoding, JSON compatibility and CSV units. UI tests cover signed tooltips, percentage conversion, numerical sorting, search and missing values.

The optional reference test uses the installed FFmpeg binary to compare per-channel `astats` means and independently encode FLAC. It requires FFmpeg on `PATH`; no third-party implementation source is used. The comparison allows the half-unit rounding error of FFmpeg's six-decimal output. FLAC decoding must preserve the same channel means as the original integer WAV.

## Manual test

Create a five-second stereo tone with +1% on the left and −2% on the right:

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*1000*t)+0.01|0.2*sin(2*PI*1000*t)-0.02:s=48000:d=5' -c:a pcm_f32le dc-offset.wav
ffmpeg -hide_banner -nostats -i dc-offset.wav -af astats=reset=0 -f null -
```

Import `dc-offset.wav`: the column should show `2.000`, with `Ch 1: +1.000%` and `Ch 2: -2.000%` in the tooltip. FFmpeg should report channel DC offsets close to `0.010000` and `-0.020000`. Compare individual channel entries, not FFmpeg's separate overall statistic. See the [astats documentation](https://ffmpeg.org/ffmpeg-filters.html#astats).

For a centred control, repeat the generator without `+0.01` and `-0.02`, using another output filename; the column should then show `0.000`. In Audacity, inspect the shifted waveform and use DC offset removal on a copy; re-exporting and analysing that copy should produce a value close to zero.

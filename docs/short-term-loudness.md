# Maximum short-term loudness (LUFS-S max)

The **LUFS-S max** column follows LUFS-M max and reports the loudest complete **3-second** window in a file. The numeric value has one decimal place; hovering gives the maximum window's start and end times. It describes a sustained local loudness maximum, complementary to whole-programme LUFS and the faster 400 ms momentary measurement.

## Calculation

The measurement uses the same K-weighted channel powers as [integrated loudness](integrated-loudness.md) and the same 3-second rolling history as [LRA](loudness-range.md). For each decoded frame after the first three seconds:

```text
S = −0.691 + 10 × log10(mean K-weighted channel-summed power over 3 s)
```

Following [EBU Tech 3341, section 2.2](https://tech.ebu.ch/docs/tech/tech3341.pdf), S uses a rectangular window with no gate or additional attack/release smoothing. The file maximum is evaluated at every complete sample-aligned window, rather than only when an LRA sample is collected. The largest power sum determines the location; exact ties keep the first.

The maximum is captured before the LRA calculation appends its analysis-only tail. There are no padded startup windows or measurements beyond the final real frame. The [momentary loudness page](momentary-loudness.md) describes the shared implementation, sample timing and column preferences.

## Availability and limits

- Supported audio is **mono or stereo at 8–768 kHz**, including decoded DSD PCM. Multichannel audio has no reading because reliable channel-position weights are not yet supplied.
- A file **shorter than three seconds** has no S maximum, even if it has valid integrated or momentary loudness. Exactly three seconds permits one complete S window.
- Silence, invalid input, failed/skipped decoding and older reports have no reading. Quiet nonzero audio below the integrated loudness gate remains measurable.
- The shared 3-second history stores powers as 32-bit floats to bound memory. Extreme finite float samples whose weighted power cannot fit invalidate S and LRA; M can remain available because its history uses 64-bit powers. Powers too tiny for the compact representation cannot produce a finite S reading.
- A maximum is not a typical loudness or a quality grade. Short accents are diluted over three seconds. A longer window can also average across pauses or transitions; audition the interval shown in the tooltip.
- K-weighting filter history affects boundaries. A maximum from an excerpt need not equal the maximum calculated over that same passage within the full file.
- This is not LRA. LRA describes the gated distribution of many S readings; this column retains the highest ungated S reading. Its value is in **LUFS**, whereas LRA is in **LU**.

## Saved reports

JSON stores `loudness_peaks.short_term` as `{ "lufs": number, "start_secs": number }`, or `null` independently of the momentary reading. If neither reading is available, `loudness_peaks` is `null`. Older reports without this field remain readable.

CSV exports `max_short_term_lufs` and `short_term_max_start_s` after the momentary fields, before `loudness_range_lu`. Times are seconds from the first decoded frame. Missing readings stay blank; finite values retain their stored precision. The table uses the unrounded number for sorting and always puts unavailable readings last.

## Verification

```sh
cargo test -p flaccompagnon-core loudness_peaks
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
```

The unit suite includes all 20 shifted-file cases from EBU Tech 3341 case 10, the known stereo calibration level, the approximately 3.01 LU mono/stereo difference, audio below the integrated gate, exact three-second availability and invalid input. It also verifies that LRA's synthetic filter tail cannot increase exported M/S maxima. A separate WAV test checks a known level step, maximum positions, report fields and backward compatibility. The optional FFmpeg comparison covers mixed frequencies, several rates and both supported layouts.

## Manual fixtures

### A sustained three-second burst

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=if(between(t\,2.15\,5.15)\,0.0707945784\,0)*sin(2*PI*1000*t)|if(between(t\,2.15\,5.15)\,0.0707945784\,0)*sin(2*PI*1000*t):s=48000:d=8' -c:a pcm_s24le short-term.wav
ffmpeg -hide_banner -nostats -i short-term.wav -af ebur128 -f null -
```

Expect **LUFS-S max ≈ −23.0** with its window near **2.15–5.15 s**. LUFS-M max should also be about −23.0. Small differences in timing are possible because of the K-filter response. In Audacity, a three-second stereo 1 kHz tone at linear amplitude `0.0707945784`, with surrounding silence, produces the equivalent check.

### Too short for an S reading

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.0707945784*sin(2*PI*1000*t)|0.0707945784*sin(2*PI*1000*t):s=48000:d=2' -c:a pcm_s24le too-short-for-s.wav
```

Expect integrated LUFS and LUFS-M max near −23.0, and **a dash for LUFS-S max**. To see how a brief accent is averaged differently, use the 400 ms burst on the [momentary loudness page](momentary-loudness.md).

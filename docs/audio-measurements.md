# Stereo and loudness checks

For clicks and short digital dropouts, see [discontinuity checks](discontinuities.md).

The stereo module owns polarity/correlation and channel balance because both use the channel energies accumulated during decoding. The loudness module owns integrated LUFS and LRA because they share K-weighting. Unit tests live under `core/tests/unit/analysis/`, separate from production code.

## Interpretation

- **Polarity:** strongly negative whole-track L/R correlation suggests opposing channel polarity. It cannot establish absolute polarity or artistic intent.
- **LUFS:** integrated, K-weighted loudness with absolute and relative gates.
- **LRA:** variation of 3-second loudness windows after gating. Readings under 60 seconds carry an approximation mark, following EBU Tech 3341 section 2.4. A 5-second constant tone is unsuitable for expecting exactly zero LRA: the analysis windows at its end can materially affect the result.
- **Balance:** unweighted whole-track RMS difference, for two-channel audio. `L +6.0 dB` means the left channel is 6 dB louder; `R silent` means every right-channel sample is zero. Both silent, unsupported layouts or invalid samples give no reading. Panning and DC offset affect this measurement; it does not by itself establish a recording defect.

LUFS/LRA currently support mono and stereo only, at 8–768 kHz. Balance is available on decoded PCM and on DSD's decoded PCM representation. Old saved reports remain readable but must be re-analyzed to obtain new measurements. CSV exports use `balance_right_minus_left_db` (positive = right louder) and `balance_silent_channel` (`left`/`right`, blank for a finite difference).

## Manual fixtures

With FFmpeg installed, run these in a new scratch directory. Commands refuse to overwrite existing fixtures. Import the resulting WAV files into the app. These artificial tones validate the named measurements only; other detectors may legitimately react to their deliberately narrow spectrum or dual mono.

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|-0.1*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le polarity.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.0707945784*sin(2*PI*1000*t)|0.0707945784*sin(2*PI*1000*t):s=48000:d=60' -c:a pcm_s24le loudness.wav
ffmpeg -n -f lavfi -i 'aevalsrc=if(lt(t\,20)\,0.1\,0.0316227766)*sin(2*PI*1000*t)|if(lt(t\,20)\,0.1\,0.0316227766)*sin(2*PI*1000*t):s=48000:d=40' -c:a pcm_s24le range.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.05*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le balance-left.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0:s=48000:d=5' -c:a pcm_s24le right-silent.wav
```

| Fixture | Expected result |
| --- | --- |
| `polarity.wav` | `polarity?`, correlation approximately −1; balance 0.0 dB |
| `loudness.wav` | −23.0 ±0.1 LUFS; LRA approximately 0 LU |
| `range.wav` | LRA 10 ±1 LU, with the under-60-second approximation mark |
| `balance-left.wav` | `L +6.0 dB` (exact amplitude-ratio result: 6.0206 dB) |
| `right-silent.wav` | `R silent`; no measurable L/R phase correlation |

Independent checks:

```sh
ffmpeg -i loudness.wav -af ebur128 -f null -
ffmpeg -i range.wav -af ebur128 -f null -
ffmpeg -i balance-left.wav -af astats -f null -
```

For `astats`, compare each channel's **RMS level dB**, not its peak. Window alignment and histogram resolution can cause small LRA differences between meters. None of these checks constitutes full EBU certification.

## Automated checks

```sh
cargo test
node --test tests/*.test.mjs
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
```

The last test needs FFmpeg on PATH; it compares six 60-second mixed-frequency signals at 44.1/48/96 kHz in mono/stereo against the installed executable, with tolerances of 0.1 LUFS and 1 LU. It is explicitly excluded from the ordinary suite so FFmpeg is not a mandatory development dependency.

References: [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf), [EBU Tech 3342](https://tech.ebu.ch/docs/tech/tech3342.pdf), [FFmpeg filter documentation](https://ffmpeg.org/ffmpeg-filters.html).

# Spectral cutoff measurement

The cutoff column is a descriptive measurement of the highest frequency that still carries appreciable averaged spectral content. It is not, by itself, an authenticity verdict. A low cutoff may be musical, archival or deliberate; a high cutoff may come from noise rather than useful programme content.

## Calculation

Decoded frames are mixed to mono for this measurement. The analyzer applies a Hann-windowed 8192-point FFT to consecutive windows, accumulates power per bin and converts the averaged magnitude spectrum to dB relative to its strongest bin. At 44.1 kHz the nominal bin spacing is about 5.4 Hz.

A five-bin moving average suppresses isolated bins. The cutoff is the highest smoothed bin above −90 dB relative to the spectral peak. The app also measures approximately 3 kHz below and above that boundary:

| Value  | Meaning                                                                           |
| ------ | --------------------------------------------------------------------------------- |
| Cutoff | Highest frequency with content above the floor                                    |
| Cliff  | Mean level below the cutoff minus mean level above it; a positive value is a drop |
| Above  | Mean level in the band above the cutoff, relative to the spectral peak            |

Short files still receive a spectrum: the final partial input is zero-padded to an FFT window. This is useful for display but gives the usual reduced frequency certainty of a short observation.

## Interpretation

The result reports the averaged response of the decoded material. It can help direct visual inspection in the spectrogram and explain a bandwidth observation. FlacCompagnon keeps it separate from `Transcoded`; transcoding uses codec re-quantization evidence instead. Upsampling uses a separate time-local MDCT dead-zone test; see [Upsampling](upsampling.md).

A cutoff near Nyquist says only that the measured spectrum has bins above the content floor. A cutoff well below Nyquist says only that the averaged spectrum did not. Neither statement proves how a file was recorded, converted or encoded.

## Limits

The threshold is relative to the strongest spectral bin, so a small amount of high-frequency noise can extend the reported cutoff. Conversely, a genuine recording with natural roll-off, filtering or no ultrasonic instruments can read low. Mono downmixing can cancel some stereo-only content, and fixed windowing cannot resolve frequency changes that happen inside the average.

## Tests

The helper functions have synthetic spectral unit tests. Run:

```sh
cargo test -p flaccompagnon-core analysis::spectrum
```

For a manual visual check, compare a 48 kHz tone with and without a low-pass filter in the app's spectrum and spectrogram:

```sh
ffmpeg -n -f lavfi -i 'sine=frequency=1000:sample_rate=48000:duration=10' -c:a pcm_s24le full-band-fixture.wav
ffmpeg -n -i full-band-fixture.wav -af 'lowpass=f=8000' -c:a pcm_s24le lowpass-fixture.wav
```

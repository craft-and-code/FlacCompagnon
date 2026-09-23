# Upsampling detection

The `Upsampled` finding identifies a high-rate PCM container whose decoded content has a persistent high-frequency dead zone compatible with a lower-rate source. It is an upsampling heuristic, never a proof of the file's history: a genuinely recorded or deliberately filtered low-bandwidth master can produce identical PCM samples.

## Signal tested

The detector runs only when the declared sample rate is above 48 kHz. It repeatedly analyses AAC-sized MDCT windows from the decoded stream, rather than using the displayed FFT cutoff. The MDCT view is useful here because it can test whether a high-frequency region stays empty over many time-local frames instead of treating a single faint spectral bin as meaningful content.

A result is flagged when all of these project thresholds hold:

| Requirement                  | Threshold                                     |
| ---------------------------- | --------------------------------------------- |
| High-rate container          | Sample rate above 48 kHz                      |
| Persistent dead zone         | At least 70% of analysed MDCT frames          |
| Mean MDCT cutoff             | Below 90% of Nyquist and at or below 30 kHz   |
| Mean level above that cutoff | At or below −75 dB relative to the frame peak |

The analyzer samples one out of four overlapping MDCT hops and caps its work at 240 frames. This bounds analysis time for long files, so the result is a measurement of the sampled programme rather than every possible transform frame.

## Result and interpretation

`Upsampled` means that the conditions above were observed. The detail text states the estimated content boundary and the container rate, and explicitly says that limited bandwidth is not proof of resampling. A `Clean` result means this test did not establish the signature; ultrasonic noise, later processing, or a source with a different resampling signature can prevent a finding.

The displayed spectral cutoff is intentionally not the verdict input. The full-track FFT is useful to inspect a recording, but it can read near Nyquist because of isolated noise even when most MDCT frames have a dead zone, and it can read low for a native dark recording. Keeping the measurement and verdict separate avoids silently turning a descriptive spectrum column into an accusation.

## Limits

No spectral rule can distinguish two files with identical samples but different provenance. Native acoustic, archival, analogue-tape and low-pass-filtered recordings can have little ultrasonic energy. Conversely, injected ultrasonic noise can obscure the signature. The numerical thresholds are project thresholds, calibrated engineering choices rather than a published classification standard or a probability of upsampling.

The detection is about PCM bandwidth. DSD has its own source check because its shaped ultrasonic noise does not follow the same model; see [DSD authenticity](dsd-authenticity.md).

## Tests

The Rust unit and integration suites create controlled spectra and exercise the classification boundary:

```sh
cargo test -p flaccompagnon-core analysis::detections
cargo test -p flaccompagnon-core analysis::analyzer
```

For a manual check, create a known low-pass source, resample it to 96 kHz, then import it. This validates the plumbing and displayed result, not a universal accuracy claim:

```sh
ffmpeg -n -f lavfi -i 'sine=frequency=1000:sample_rate=44100:duration=30' -af 'lowpass=f=20000,aresample=96000' -c:a pcm_s24le upsampled-fixture.wav
```

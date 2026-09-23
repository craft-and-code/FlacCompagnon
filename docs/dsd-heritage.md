# DSD heritage in high-resolution PCM

This analysis recognises a spectral shape associated with a DSD master converted to high-resolution PCM. It is used as evidence for the descriptive `Hi-Res (DSD source)` badge; it does not create an authenticity warning and does not prove that the PCM file came from DSD.

## Calculation

The check runs only on PCM at 96 kHz or above, using the averaged spectrum from the normal streaming analyzer. It measures a valley from 22–30 kHz and an ultrasonic ramp from 36 kHz to the lower of 75 kHz or 92% of Nyquist. The detector returns a result when both conditions hold:

| Requirement         | Threshold                                  |
| ------------------- | ------------------------------------------ |
| Ultrasonic rise     | Ramp minus valley at least 15 dB           |
| Absolute ramp level | Above −75 dB relative to the spectral peak |

Both bands must span enough finite FFT bins. The wide-band average is intentional: it avoids turning a few isolated high-frequency bins into a source claim.

## Interpretation and limits

DSD's sigma-delta process can create an ultrasonic noise ramp that remains when a DSD master is converted to a sufficiently high-rate PCM file. The test looks for that shape. It cannot distinguish a real DSD transfer from another process that produces the same spectral valley and rise, and it can miss a DSD transfer after low-pass filtering, noise reduction or later processing.

A missing result is not evidence that a high-resolution PCM master is native PCM. It says only that the defined ramp was not established. The corresponding DSD-file check is separate: see [DSD PCM-source detection](dsd-pcm-source.md).

## Tests

```sh
cargo test -p flaccompagnon-core dsd::spectral
cargo test -p flaccompagnon-core pipeline
```

For a practical check, import a documented high-rate PCM transfer of DSD material and inspect its spectrum alongside the result. Do not use the badge as a provenance certificate.

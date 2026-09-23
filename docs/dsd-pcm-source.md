# DSD PCM-source detection

This analysis looks for a sharp PCM Nyquist boundary in decoded DSF or DFF content. Native DSD normally blends into rising sigma-delta noise above the audible range, while DSD created from 44.1 or 48 kHz PCM can preserve a digital brick wall at the PCM source boundary. A finding raises the app's DSD `Upsampled` detection.

## Calculation

The DSD stream is decoded by FFmpeg to float PCM and measured at that decoded rate. FlacCompagnon uses the averaged spectrum already produced by the streaming analyzer, then compares bands around both common PCM Nyquist boundaries:

| Boundary  | Band below      | Band above      |
| --------- | --------------- | --------------- |
| 22.05 kHz | 20.05–21.85 kHz | 22.35–24.05 kHz |
| 24 kHz    | 22.0–23.8 kHz   | 24.3–26.0 kHz   |

The band below must contain real content, above −80 dB relative to the spectral peak. The detector subtracts the mean level above the boundary from the mean level below it. A drop of at least 30 dB qualifies; when both boundaries qualify, the larger drop is reported.

The 30 dB threshold is a project calibration. Synthetic delta-sigma ground truth measured about 3 dB for native-band content and about 50 dB for content limited to 44.1 kHz before DSD conversion.

## Interpretation and limits

A finding means the decoded spectrum contains this PCM-like signature. It does not prove every production step or rule out all native DSD processing. Conversion filters, source material without energy near the tested boundary, noise shaping, later filtering and added noise can hide or imitate a cliff. Two files with identical samples cannot be distinguished by their histories.

No content result is available when FFmpeg is unavailable or cannot decode the DSD stream. A valid DSF/DFF header still reports container information in that case.

## Tests

```sh
cargo test -p flaccompagnon-core dsd::spectral
cargo test -p flaccompagnon-core pipeline::dsd
```

Use known DSD material with FFmpeg installed for a manual inspection. A synthetic PCM tone converted to DSD checks the analysis path, but does not provide representative native-DSD ground truth.

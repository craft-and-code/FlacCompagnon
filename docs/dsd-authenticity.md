# DSD authenticity index

This page is retained as the DSD entry point. DSD has a container check and two separate spectral analyses, each documented independently:

- [DSD PCM-source detection](dsd-pcm-source.md): detects a PCM-like brick wall in decoded DSF/DFF content and can raise the `Upsampled` finding.
- [DSD heritage in high-resolution PCM](dsd-heritage.md): detects sigma-delta noise shaping in PCM at 96 kHz or above and can support the `Hi-Res (DSD source)` badge.

FlacCompagnon parses DSF and DFF headers natively, validating format markers, channel count and plausible DSD sample rate. It derives DSD64, DSD128, DSD256 and related labels from the one-bit rate. A valid header establishes container structure only; it does not establish audio provenance.

Content analysis requires FFmpeg because the app converts the one-bit stream to float PCM. The analysis rate is normally the DSD rate divided by eight (352.8 kHz for DSD64), not the one-bit container rate. Without FFmpeg, header information remains available but content analysis is unavailable. DST-compressed DFF can also be header-only if FFmpeg cannot decode it.

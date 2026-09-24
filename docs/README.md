# Analysis guide

This directory documents every measurement and detection exposed by FlacCompagnon. Each page states what is measured, how the value or verdict is calculated, what the result means, what it cannot establish, and how to reproduce the check. A positive detection is evidence for the described property; a negative detection means only that this particular test did not establish it.

The app decodes a file once and feeds the decoded frames to streaming measurements. Measurements can therefore share the same file read without sharing a conclusion. The three authenticity verdicts are deliberately independent: a file can be flagged for upscaling, upsampling, transcoding, several of these, or none.

| Kind                   | Analysis                      | Result in the app                            | Documentation                                     |
| ---------------------- | ----------------------------- | -------------------------------------------- | ------------------------------------------------- |
| Authenticity detection | Integer precision / upscaling | `Upscaled`, real bit depth                   | [Integer bit depth](upscaling.md)                 |
| Authenticity detection | Bandwidth / upsampling        | `Upsampled`                                  | [Upsampling](upsampling.md)                       |
| Authenticity detection | Codec re-quantization         | `Transcoded`, lattice score                  | [Transcoding](transcoding.md)                     |
| Descriptive spectrum   | Content cutoff                | Cutoff frequency, cliff and upper-band level | [Spectral cutoff](spectral-cutoff.md)             |
| Channel relationship   | Dual mono                     | Fake-stereo indication                       | [Fake stereo](fake-stereo.md)                     |
| Channel relationship   | L/R polarity                  | Correlation and polarity indication          | [Stereo polarity](stereo-polarity.md)             |
| Channel relationship   | Local and band correlation    | Local phase, Band phase                      | [Local phase](local-phase.md)                     |
| Channel relationship   | L/R level                     | Balance                                      | [Stereo balance](stereo-balance.md)               |
| Channel relationship   | High-frequency stereo width   | HF Stereo                                    | [High-frequency stereo](high-frequency-stereo.md) |
| Level                  | Mean displacement from zero   | DC (%)                                       | [DC offset](dc-offset.md)                         |
| Level                  | Full-scale clipping           | Clip events and clipped samples              | [Clipping](clipping.md)                           |
| Level                  | Inter-sample peak             | True peak in dBTP                            | [True peak](true-peak.md)                         |
| Dynamics               | Loud-passage crest factor     | DR                                           | [Dynamic range](dynamic-range.md)                 |
| Loudness               | Programme loudness            | Integrated LUFS                              | [Integrated loudness](integrated-loudness.md)     |
| Loudness               | Loudness variation            | LRA in LU                                    | [Loudness range](loudness-range.md)               |
| Restoration cue        | Short discontinuities         | Impulses count and locations                 | [Impulses](impulses.md)                           |
| Restoration cue        | Exact-zero gaps               | Dropouts count and locations                 | [Dropouts](dropouts.md)                           |
| DSD authenticity       | PCM-source boundary in DSD    | DSD `Upsampled` finding                      | [DSD PCM-source](dsd-pcm-source.md)               |
| DSD provenance cue     | DSD heritage in PCM           | `Hi-Res (DSD source)` badge evidence         | [DSD heritage](dsd-heritage.md)                   |
| Integrity check        | FLAC STREAMINFO signature     | FLAC MD5 status                              | [FLAC MD5](flac-md5.md)                           |

The [audio measurements index](audio-measurements.md) and [discontinuity checks index](discontinuities.md) remain as stable entry points for existing links.

## Verification commands

The complete project checks are run from the repository root:

```sh
npx tsc --noEmit
npm run build
npm run check:markdown
cargo test
cargo clippy
```

Some reference comparisons require FFmpeg. Those tests are marked ignored so FFmpeg remains optional for ordinary development. Individual pages name the narrowest useful command and their manual audio fixtures.

# flaccompagnon-spectrogram

Reusable streaming spectrogram computation and lossless PNG rendering in Rust.
MPL-2.0, like FlacCompagnon's analysis core. No Tauri, audio decoder, filesystem,
Python, system font or external executable is required by this crate.

`flaccompagnon-core::spectrogram` re-exports this API and adds a read-only file
adapter. Applications already using the analysis core, including Aède, inherit
the same renderer from the same tagged source. They need no sibling checkout.

## Data and rendering

1. Construct `SpectrogramBuilder` with the native sample rate/channel count,
   exact decoded frame count and `AnalysisOptions`.
2. Feed complete interleaved `f32` PCM packets with `push_interleaved` or its
   cancellable variant. A ring buffer retains one analysis window, not a track.
3. Call `finish` to analyze the right boundary and obtain a `Spectrogram`.
4. Call `render_png` with `ImageLabels` and `RenderOptions`. The returned bytes
   are a lossless RGB PNG. The host validates/publishes its own destination.

Render a measured result again with different colors, labels or markers without
repeating decoding or FFTs. Larger plots require sufficiently detailed source
measurements; scaling a PNG cannot create new spectral detail.

## Options

| Group     | Configuration                                                                         |
| --------- | ------------------------------------------------------------------------------------- |
| Signal    | Actual sample rate, channel count and exact frame count                               |
| Analysis  | Plot width/height, FFT/hop lengths, Hann/Blackman-Harris/rectangular window           |
| Channels  | Mean power, strongest channel, or one selected channel                                |
| Frequency | Linear/log axis, minimum and maximum frequency within Nyquist                         |
| Pixels    | Peak or mean linear-power reduction before conversion to decibels                     |
| Palette   | Green/blue/amber/violet presets, arbitrary accent or 2–256 custom ordered color stops |
| Levels    | Explicit lower/upper dBFS/bin bounds, −120…0 by default                               |
| Frame     | Background/text/muted/border colors, four margins, text size, optional font           |
| Labels    | Generator, title, artist, caller-supplied actual audio description                    |
| Overlays  | Axes, grid, color bar, optional physical frequency/time markers                       |
| Runtime   | Cancellation between FFT windows, checked input and allocation limits                 |

`PlotSize::Standard` is **1200×627**, rounded from the original 900:470 aspect
ratio. `PlotSize::Large` is **1800×940**. These are plotted dimensions. Default
margins produce PNGs of **1448×903** and **2048×1216**, respectively.

## Measurement contract

- The default automatic FFT is the nearest power of two to 8192 samples at
  44.1 kHz (8192 at 44.1/48 kHz; 16384 at 96 kHz), capped at 65536.
- A periodic Hann window and a hop of FFT/8 are defaults, not fixed API limits.
- Each channel is transformed separately using `f64` FFT arithmetic. Mean power
  preserves opposite-polarity stereo content without doubling identical mono.
- Coherent-gain-corrected one-sided **peak amplitude** sets a bin-centered
  full-scale sine to 0 dBFS/bin. DC/Nyquist do not get a doubled amplitude.
  This is not a power spectral density in dBFS/Hz.
- Each default pixel retains its strongest contributing bin/window. This
  preserves brief or narrow features; it is not mean energy over that pixel.
- Both boundaries receive centered zero padding. Silence remains zero power.
- If there are more rows/columns than independent observations, nearest
  observations fill them; this adds no physical frequency/time resolution.
- PNG compression preserves rendered pixels. The colored image remains a
  finite-resolution visualization, not an archive of the PCM or raw FFT bins.
- PNG metadata retain transform, channel, pooling, level and source settings.
  Physical cutoff/time annotations are caller-provided evidence, not verdicts.

The core file adapter uses two bounded decode passes to count actual frames and
map windows to the exact timeline. Invalid/truncated streams, changed lengths
and cancellation return errors without publishing an incomplete image.

## Fonts and verification

`assets/NotoSans-Variable.ttf` is redistributed under the accompanying SIL OFL
1.1 license. It covers Latin, Greek and Cyrillic; unsupported scripts need a
caller-supplied font. Drawing uses `fontdue`, with no installed font lookup.
Default-font renders retain the full OFL notice in PNG metadata and in the
generating binary. Hosts supplying a custom font retain its own license notices.

```sh
cargo test -p flaccompagnon-spectrogram
cargo test -p flaccompagnon-core spectrogram
cargo clippy -p flaccompagnon-spectrogram --all-targets -- -D warnings
```

Tests use analytically derived sine/DC/Nyquist levels and frequency locations,
opposed stereo, packet independence, malformed samples/options, cancellation,
custom palettes/fonts, Unicode labels and exact PNG pixel round trips.

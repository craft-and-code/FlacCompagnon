# Native PNG spectrograms

Select **Generate spectrograms** to create one PNG per track in a `spectrograms/`
folder beside the audio. The source files are opened read-only. Successful PNGs
replace previous images atomically; errors and cancellation preserve them.

The complete calculation, image composition, text and PNG encoding run in Rust.
There is no Python process or external renderer. **FFmpeg is only needed to
decode DSD (`.dsf`/`.dff`) content**, including for spectrograms; ordinary PCM and
supported compressed formats use the native decoders already used by analysis.

## Size and color

The **Spectrograms** menu contains **Size** and **Color** submenus:

- **Standard (1200 × 627)** is the default. The height rounds 1200 × 470 / 900.
- **Large (1800 × 940)** retains the existing detailed plot.
- **Green** is the shared default for FlacCompagnon and Aède. **Blue**, **Amber**
  and **Violet** are other monochrome ramps with the same fixed level scale.

Choices survive restarts. Selecting a menu item changes the preference; it does
not generate or overwrite images until **Generate spectrograms** is selected.
Existing `half` preferences now select Standard; `full` still selects Large.

The dimensions above describe the spectral area. With the default frame the
complete PNG measures 1448 × 903 or 2048 × 1216. PNG encoding is lossless.

## Reading the image

Time runs horizontally, frequency vertically up to the decoded Nyquist limit.
The right-hand color bar maps −120…0 dBFS/bin from dark to light. Colors describe
level; they do not represent detection categories or channels. No automatic
per-track gain changes that mapping.

The application signature appears at the top right, title/artist at the bottom
left, and actual decoded sample rate, available bit depth and Mono/Stereo/channel
count at the bottom right. Missing titles use the file stem. Unknown bit depth
is not guessed. DSD is explicitly labeled as a decoded PCM view.

Each channel is transformed before powers are combined, avoiding cancellation
of opposite-polarity content. A Hann-windowed FFT with 87.5% overlap is the
default. Pixel reduction retains peaks, including brief events. The footer and
PNG metadata record settings. Read the [crate contract](https://github.com/craft-and-code/FlacCompagnon/tree/main/spectrogram#readme)
for amplitude normalization and the time/frequency resolution limits.

A spectral boundary can come from filtering, recording conditions or an
encoder. It is not a proof of lossy provenance. The renderer supports optional
measured cutoff/time markers; the app does not draw a guessed cutoff line.

## Reuse and verification

`flaccompagnon-spectrogram` has no app, Tauri, decoder or filesystem dependency.
`flaccompagnon-core::spectrogram` re-exports it and supplies the read-only file
adapter. Aède uses this same tagged crate through its existing core dependency.
The host chooses the signature, labels, colors, dimensions and annotations.

For a reproducible render through the desktop application's Rust service:

```sh
cargo run -p flaccompagnon-services --example spectrogram -- input.flac output.png green
cargo run -p flaccompagnon-services --example spectrogram -- input.flac output.png blue 1800 940
```

DSD examples additionally need `FLACCOMPAGNON_FFMPEG` set to the decoder path.
Native application generation resolves it automatically for DSD only.

```sh
cargo test -p flaccompagnon-spectrogram
cargo test -p flaccompagnon-core spectrogram
cargo test -p flaccompagnon --lib spectrogram
```

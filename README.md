# FlacCompagnon

[![CI](https://github.com/craft-and-code/FlacCompagnon/actions/workflows/ci.yml/badge.svg)](https://github.com/craft-and-code/FlacCompagnon/actions/workflows/ci.yml)
[![Release](https://github.com/craft-and-code/FlacCompagnon/actions/workflows/release.yml/badge.svg)](https://github.com/craft-and-code/FlacCompagnon/actions/workflows/release.yml)
[![Docs](https://github.com/craft-and-code/FlacCompagnon/actions/workflows/docs.yml/badge.svg)](https://github.com/craft-and-code/FlacCompagnon/actions/workflows/docs.yml)

**A cross-platform desktop tool that checks whether your "lossless" audio is actually lossless.**

FlacCompagnon is a from-scratch, open-source successor to the discontinued _Lossless Audio Checker_. Drop a folder **or a single audio file** onto the window and it runs the same three independent detections as the original — **Upscaling**, **Upsampling**, and **Transcoding** (including the **AAC re-quantization** test, which catches AAC sources at every bitrate) — verifies **FLAC MD5** signatures, flags **fake stereo** files, detects **clipping**, and can render a **spectrogram** for each track.

Built with **Rust** and **Tauri v2**, it compiles to a small native app for **Linux, Windows, and macOS**.

---

## What it does

### 1. Authenticity detections (Lossless Audio Checker model)

FlacCompagnon runs the same three **independent** detections as the original Lossless Audio Checker. A file can trip none, one, or several; if none fire it is reported **Clean**. The **Detections** column shows a coloured tag per finding, and hovering it explains the reasoning.

| Detection       | Meaning                                                                                                                                                                                                                                                                          |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Upscaling**   | Fake resolution: a ≤16-bit signal stored at 24-bit (the low bits carry no real information).                                                                                                                                                                                     |
| **Upsampling**  | Fake sample rate: a high-rate container (e.g. 96 kHz) whose content stops sharply around the CD range (~22 kHz).                                                                                                                                                                 |
| **Transcoding** | Lossy source re-wrapped as lossless. Three signatures, strongest first: the **AAC re-quantization grid** (coefficients snap onto AAC's quantization grid at a synchronized MDCT alignment — near-conclusive, catches every bitrate), an MDCT-domain high-frequency dead zone, and a brick-wall spectral cut-off. Shown as _Transcoded_ (detected) or _Transcoded?_ (a gentle early roll-off that is ambiguous). |

See [Detection algorithms](#detection-algorithms) below for how each works, and its limitations. Like the original, these are informed heuristics, not cryptographic proof — the spectrogram is the final arbiter.

### 2. FLAC MD5 verification

Every FLAC file stores an MD5 hash of its decoded audio in the STREAMINFO block. FlacCompagnon reads it natively (no external `flac` binary required) and, by fully decoding the file, recomputes the hash to confirm the audio is intact — the same integrity check as `flac -t`.

The **MD5** column only appears when the analysis actually includes FLAC files, and reports one of:

- **OK** — signature present and the audio matches it.
- **Mismatch** — signature present but the audio does **not** match (corruption or a non-conforming encoder).
- **No signature** — the file was encoded without an MD5 (nothing to verify against).

### 3. Spectrogram generation

Click **Generate spectrograms** to render a high-resolution spectrogram image for every track using **ffmpeg** installed on your system (resolved automatically at runtime — see prerequisites). For each folder that contains audio, a `spectres/` sub-folder is created next to the files, and one PNG is written per track. The image includes a labelled **frequency axis** (its top equals Nyquist = sample-rate ÷ 2) and a caption spelling out the **sample rate**, bit depth, channel count, and format — so the cutoff and the sampling are visible at a glance.

### 4. Extra integrity checks

- **Fake stereo** — detects "stereo" files that are really dual-mono (both channels identical).
- **Clipping** — counts full-scale sample runs (each _event_ = ≥3 consecutive samples at 0 dBFS) and reports the peak level in dBFS. This flags an over-loud master; it is independent of whether the file is lossless.

### 5. CSV export (on demand)

Analysis never writes anything by itself. When you want to keep the results, click **Save CSV…** and choose where to store a spreadsheet-friendly `FlacCompagnon.csv` (all columns: status, upscaling, upsampling, transcoding, cutoff, bit depth, clipping, MD5, …). You decide the name and location — nothing is dropped into your music folders unless you ask for it.

---

## Supported formats

FLAC, WAV, AIFF, ALAC/MP4 (`.m4a`), CAF, OGG/Vorbis, MP3, and AAC. (MP3/AAC are decoded so you can compare them, though they are lossy by definition.)

---

## How it works

```
                    ┌──────────────────────────── Tauri (Rust) ────────────────────────────────┐
  drop a folder ──▶ │  list files ─▶ decode (symphonia) ─▶ streaming analyzer                  │
                    │                                       ├─ FFT spectrum ─▶ cut-off         │
   TypeScript UI    │                                       ├─ MDCT ─▶ AAC requant + dead zone │
   (results table,  │                                       ├─ clipping / fake-stereo          │
    progress, ◀──── │                                       └─ effective bit depth ─▶ 3 checks │
    spectrograms)   │  FLAC ─▶ fused decode: analysis + MD5 in one pass (claxon)              │
                    │  CSV export (on demand)     spectrograms ─▶ system ffmpeg ─▶ spectres/   │
                    └──────────────────────────────────────────────────────────────────────────┘
```

The project is a Cargo workspace with two crates:

- **`core/`** — a pure-Rust library (`flaccompagnon-core`) containing all the analysis. It has no UI dependency and is fully unit-tested.
- **`src-tauri/`** — the Tauri desktop app that wraps the core and exposes it to the web frontend.

---

## Detection algorithms

These mirror the three tests described by the authors of the original Lossless Audio Checker, Julien Lacroix & Yann Prime, in their AES papers (see [references](#references)). FlacCompagnon is an independent re-implementation of the _principles_ — the original engine is closed-source and the papers are paywalled, so exact thresholds differ and are tunable in `core/`.

**Upscaling (fake resolution).** Every integer sample is OR-ed together; the number of low bits that are _always_ zero is the padding. If a file declares 24-bit but its effective depth is ≤16 bits, it is a 16-bit signal padded to 24-bit. This works for WAV/AIFF (raw bytes) and, because the check is done on the decoded samples, for FLAC/ALAC too. Shown green in the **Real bits** column when it matches the declared depth, red when it does not.

**Upsampling (fake sample rate).** The decoded signal is transformed by a Hann-windowed FFT (8192-point), averaged over the whole track. The **cut-off frequency** is the highest frequency still carrying content (above a floor set relative to the spectral peak). If the sample rate is "hi-res" (> 48 kHz) but the content stops sharply around the CD range (~22 kHz), the extra bandwidth is empty — the file was up-sampled from a lower rate.

**Transcoding (lossy source).** Three signatures, from strongest to weakest:

1. _AAC re-quantization grid (the LAC method)_ — an AAC encoder quantizes MDCT coefficients per scale-factor band on the grid `|X| = n^(4/3)·Δ`, and decoding to PCM preserves that structure. FlacCompagnon re-analyzes the audio with AAC's own transform (2048-sample MDCT, both sine and KBD window shapes), sweeping **all 1024 possible frame alignments at one-sample resolution**: only the encoder's exact alignment makes the coefficients snap back onto the quantization grid, and a single sample of misalignment destroys the effect. The fraction of on-grid bands at the best alignment is near-conclusive evidence. Measured on real AAC→FLAC transcodes (16-bit chain) it reaches **0.70–0.97 at every bitrate, including 256 kbps**, while genuine music never exceeded **0.014** at any of the 1024 alignments. This is the only signature able to catch high-bitrate AAC, which keeps the full audio bandwidth. Runs at 44.1/48 kHz (the rates covered by the AAC scale-factor band table, per the LAC paper).
2. _AAC dead zone (MDCT domain)_ — at low-to-mid bitrates the encoder zeroes whole high-frequency coefficient bands, leaving a flat, sharply-bounded dead zone in the MDCT domain that survives the decode. Catches ~128–192 kbps AAC cheaply.
3. _Spectral brick-wall_ — a sharp cut-off well below Nyquist that drops into a flat, low "dead zone" is characteristic of an MP3/AAC low-pass (≈16 kHz at 128 kbps, ≈19 kHz at 192, ≈20 kHz at 320). A gentle roll-off with no cliff is reported only as _Transcoded?_ (suspected), because it can also be natural.

The re-quantization hit-rate is exported in the CSV as the `aac_grid` column (empty when the check did not run).

### Known limitation: naturally "dark" recordings

All cut-off-based detection — LAC included — assumes genuine music has energy up near Nyquist. Acoustic, classical, and older (ADD / analog-tape) recordings often have almost nothing above ~16–18 kHz _by nature_, so their spectrum rolls off early and can read as **Upsampled** or **Transcoded?** even though they are perfectly lossless. FlacCompagnon mitigates this by only calling a hard _Transcoded_ when there is a genuine sharp cliff into a dead zone (a codec signature), leaving gentle roll-offs as the softer _Transcoded?_. When in doubt, look at the spectrogram.

---

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) (stable) and Cargo.
- [Node.js](https://nodejs.org/) 18+ and npm.
- Tauri v2 system dependencies for your OS — see
  <https://v2.tauri.app/start/prerequisites/> (on Linux: `webkit2gtk`, `libayatana-appindicator`, etc.).
- **ffmpeg** — only needed for the spectrogram feature. Install it with your package manager:
    - macOS: `brew install ffmpeg`
    - Debian/Ubuntu: `sudo apt install ffmpeg`
    - Windows: `choco install ffmpeg` (or download from ffmpeg.org and add it to `PATH`)

`ffmpeg` is located automatically at runtime (it checks `PATH` plus common install locations such as Homebrew's `/opt/homebrew/bin`). If it lives somewhere unusual, point the app at it with the `FLACCOMPAGNON_FFMPEG` environment variable. Analysis, MD5 verification, and reports do **not** require ffmpeg — only spectrogram rendering does.

### 1. Install dependencies

```bash
npm install
```

### 2. Run in development

```bash
npm run tauri dev
```

### 3. Build a release bundle

```bash
npm run tauri build
```

The installer/app bundle is written to `src-tauri/target/release/bundle/`.

> **Cross-platform note:** native desktop apps are normally built **on** their target OS. Build the Windows app on Windows, the macOS app on macOS, and the Linux app on Linux. The easiest way to produce all three from one place is a CI matrix (e.g. GitHub Actions) that runs `npm run tauri build` on `windows-latest`, `macos-latest`, and `ubuntu-latest`.

### App icons

The app icon (a spectrum-bars-under-a-magnifier mark on a blue-to-purple gradient) ships in `src-tauri/icons/`. To replace it with your own artwork:

```bash
npm run tauri icon path/to/your-1024x1024.png
```

---

## Continuous integration & releases

Two GitHub Actions workflows are included:

- **CI** (`.github/workflows/ci.yml`) runs on every push and pull request: it runs the `core` test suite, type-checks and bundles the frontend, and compiles the whole Rust workspace on Linux. The badges at the top of this README reflect its status.
- **Release** (`.github/workflows/release.yml`) builds installers for **macOS (Apple Silicon + Intel), Windows and Linux** and publishes them to a GitHub Release. It runs when you push a version tag:

  ```bash
  git tag v0.1.0
  git push origin v0.1.0
  ```

(or from the Actions tab via "Run workflow"). The release is created as a **draft** — review the attached installers, then publish it. Your downloads then live on the repository's **Releases** page. ffmpeg is not bundled; users install it themselves for the spectrogram feature.

## Testing

All analysis logic lives in the `core` crate and is covered by unit and integration tests (the integration tests synthesize WAV files with known spectral properties and assert the detections; `mdct` has its own correctness tests):

```bash
cargo test -p flaccompagnon-core
```

---

## Documentation (rustdoc)

The whole `core` crate is documented with Rust doc comments (crate-, module- and function-level), so you can browse the full API — every analysis routine, its inputs and its heuristics — as a generated HTML site. Build it locally with:

```bash
cargo doc -p flaccompagnon-core --no-deps --open
```

On every push to the main branch the `Docs` workflow (`.github/workflows/docs.yml`) builds this documentation and publishes it into the `doc/` sub-folder of the `gh-pages` branch, so it can live alongside a static presentation site served from the root of the same branch (neither overwrites the other). Enable it once under **Settings → Pages → Source: Deploy from a branch → `gh-pages` / (root)**; the API docs are then served at <https://craft-and-code.github.io/FlacCompagnon/doc/>.

---

## Output layout

Analysis alone writes **nothing**. The only files FlacCompagnon can create are the spectrogram PNGs (when you click _Generate spectrograms_) and a CSV (when you click _Save CSV…_ and pick a location). For a dropped folder:

```
My Album/
├── 01 - Track.flac
├── 02 - Track.flac
└── spectres/              ← only created when you generate spectrograms
    ├── 01 - Track.png
    └── 02 - Track.png
```

Sub-folders that contain audio each get their own `spectres/` folder next to their files.

### Your audio is never modified

FlacCompagnon opens every track **read-only** — it decodes samples to analyze them and never writes back to an audio file in any way. The MD5 check only reads the FLAC and recomputes a hash in memory; it does not alter the file.

---

## Limitations & notes

- The spectral detections are **heuristics** (as in the original). See [Detection algorithms](#detection-algorithms) — in particular, naturally dark/acoustic recordings can read as _Upsampled_ or _Transcoded?_; always sanity-check with the spectrogram. The AAC re-quantization detection, in contrast, is close to a proof: it requires the audio to snap onto AAC's exact quantization grid at a synchronized frame alignment, which genuine audio essentially never does.
- **AAC transcode detection covers all bitrates at 44.1/48 kHz** (validated on real 128/192/256 kbps AAC→FLAC transcodes against their originals). **MP3 sources** are still only caught through the spectral brick-wall signature, so high-bitrate MP3 (320 kbps) can pass — MP3 uses a different filterbank (hybrid PQMF + 576-point MDCT) and would need its own re-quantization detector.
- Effective bit-depth reconstruction is exact for ≤ 24-bit integer sources.
- FLAC files are decoded **once**: a fused pass feeds the analysis and hashes the MD5 from the same raw integer samples (bit-identical to `flac -t`), so MD5 verification adds only a negligible hashing cost on top of the analysis.

## Roadmap ideas

Easy future additions (the analyzer is modular): per-channel spectral analysis, DR (dynamic range) measurement, true-peak/inter-sample clipping, joint-stereo artifact detection, ReplayGain scanning, and a re-importable JSON report.

## References

- J. Lacroix & Y. Prime, _Lossless Audio Checker: A Software for the Detection of Upscaling, Upsampling, and Transcoding in Lossless Musical Tracks_, AES Convention e-Brief (AES e-Library #17972).
- J. Lacroix & Y. Prime, _Detection of Genuine Lossless Audio Files: Application to the MPEG-AAC Codec_.
- Original project (discontinued): losslessaudiochecker.com; GUI source: <https://github.com/emps/Lossless-Audio-Checker-GUI> (GPL-2.0).

## License

MIT — see [LICENSE](LICENSE). Bundled ffmpeg builds carry their own licenses; review them before redistribution.

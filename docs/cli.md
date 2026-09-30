# Using the command line

The **CLI** is FlacCompagnon without the graphical window: type a command in Terminal or PowerShell, give it a file or folder, and read the results there. It uses the same Rust analysis engine and the same JSON report schema as the desktop app. It reads your audio without changing it.

## Install and check

On the [GitHub Releases page](https://github.com/craft-and-code/FlacCompagnon/releases), choose a standalone archive named `FlacCompagnon-CLI_<version>_<platform>.tar.gz` or `.zip`. The graphical application files begin with `FlacCompagnon-App`, so the two downloads are distinguishable at a glance. The platforms are macOS Apple Silicon, Linux x86_64 and Windows x64.

Extract the archive. In its folder, run:

```sh
# macOS or Linux
./flaccompagnon --help
./flaccompagnon --version
```

```powershell
# Windows PowerShell
.\flaccompagnon.exe --help
.\flaccompagnon.exe --version
```

The examples below assume the executable's folder has been added to your `PATH`. Otherwise replace `flaccompagnon` with `./flaccompagnon`, `.\flaccompagnon.exe`, or its full path. On macOS, follow the release's first-launch instructions if the system blocks this unsigned build.

## Analyze your first file

```sh
flaccompagnon "Music/Album/01 - Track.flac"
```

Progress shows each file before analysis. Measurements are hidden by default; add `--show-results` to print them. An animated loader appears during file discovery and decoding in an interactive terminal. Quotation marks keep a path containing spaces together. An unavailable Rust value may appear as `None`; it is not a measured zero. Progress is sent to stderr; requested results go to stdout.

To scan an album, including its subfolders:

```sh
flaccompagnon "Music/Album"
```

You can mix several files and folders. To inspect only the named folder level:

```sh
flaccompagnon "Music/Album A" "Music/Album B" --no-recursive
```

## Choose the results shown

```sh
flaccompagnon "Music/Album" --analysis loudness
flaccompagnon "Music/Album" --analysis phase --analysis hf-stereo
flaccompagnon "Music/Album" -a clipping,clicks,dropouts
```

**Selection currently filters terminal output only.** The engine still computes the complete analysis. It does not shorten processing to just the requested measurement, and a saved JSON always contains the full report.

| Name           | Terminal results                                            |
| -------------- | ----------------------------------------------------------- |
| `authenticity` | Authenticity summary and detection details                  |
| `bit-depth`    | Declared and effective bit depth                            |
| `spectrum`     | Spectral cutoff                                             |
| `stereo`       | Dual mono and channel balance                               |
| `phase`        | Global correlation, polarity and local/band phase           |
| `hf-stereo`    | High-frequency stereo measurement                           |
| `clipping`     | Clip events, sample peak and true peak                      |
| `loudness`     | Integrated LUFS, M/S maxima and LRA                         |
| `dynamics`     | DR estimate                                                 |
| `clicks`       | Suspected impulse count (the app column is called Impulses) |
| `dropouts`     | Suspected dropout count                                     |
| `dc-offset`    | Channel DC offsets                                          |
| `flac-md5`     | FLAC audio signature status                                 |
| `fingerprints` | Whole-file MD5 and CRC32                                    |

## Save and reopen a JSON report

```sh
flaccompagnon "Music/Album" --json "Music/Album/FlacCompagnon.json"
```

`--json` requires a **file path**, not just a folder or a flag. The parent folder must already exist. Valid measurements in existing reports are reused when audio size and modification time agree. Add `--force` to reanalyze and replace the chosen report. A malformed destination requires `--force`. This command produces one report covering all selected audio files; use `--json-layout album` or `--json-layout artist` to split several albums into separate reports.

The schema is the desktop app's complete, versioned `flaccompagnon-report` format. Drop the JSON onto the desktop results list to reopen the saved analysis without re-decoding the audio. A report is a snapshot: it does not update itself after audio is edited, and saved paths can become stale when files move. See [file fingerprints](fingerprints.md) for identity and relocation limits.

You can request concise terminal output while saving all measurements:

```sh
flaccompagnon "Music/Album" -a loudness,phase -j "Music/Album/FlacCompagnon.json"
```

## One report per album with Aède

[Aède](https://craft-and-code.github.io/aede/) is a Rust music-library catalog. It organizes tracks and metadata and integrates the FlacCompagnon engine directly. It does not need to launch this executable.

With an Aède version that includes the integration, first catalog the files, then analyze them:

```sh
aede scan "Music"
aede analyze "Music" --json
```

Aède analyzes cataloged tracks and writes one `<album>.json` in each album folder, using its catalog grouping (including a shared album folder for multi-disc releases). An existing report can be imported with `aede import "Music/Album/FlacCompagnon.json"`. Consult Aède's README for its version-specific options.

## DSD and FFmpeg

Ordinary PCM analysis does not require FFmpeg. DSD content analysis does; without a usable decoder, container information can still be read but content measurements are unavailable.

```sh
flaccompagnon "Music/DSD Album" --ffmpeg "/opt/homebrew/bin/ffmpeg"
```

Discovery uses the explicit `--ffmpeg` path first, then the `FLACCOMPAGNON_FFMPEG` environment variable, then `PATH`. The CLI does not generate spectrograms or edit tags.

## Option reference

```text
flaccompagnon [OPTIONS] <FILE|FOLDER>...
```

| Option                | Effect                                                                    |
| --------------------- | ------------------------------------------------------------------------- |
| `-a, --analysis NAME` | Select terminal fields; repeat or separate names with commas              |
| `--show-results` | Print terminal measurements (default: progress only) |
| `--force` | Reanalyze files and replace selected reports |
| `--json-layout album\|artist` | Save one report per album in its folder or parent |
| `-j, --json PATH`     | Write the complete JSON report                                            |
| `--recursive`         | Include subfolders (default)                                              |
| `--no-recursive`      | Stay at the named folder level                                            |
| `--no-flac-md5`       | Read stored FLAC signatures without verifying decoded audio against them  |
| `--ffmpeg PATH`       | Choose the FFmpeg executable for DSD decoding                             |
| `-h, --help`          | Print help                                                                |
| `-v, -V, --version`       | Print the version (use on its own)                                        |
| `--`                  | Treat all remaining arguments as paths, including names starting with `-` |

## Exit codes and automation

| Code | Meaning                                                      |
| ---- | ------------------------------------------------------------ |
| `0`  | Processing completed without file errors                     |
| `1`  | Processing, report writing or one or more audio files failed |
| `2`  | Invalid command-line arguments                               |

An authenticity finding or a clipping event does **not** cause exit code 1. Inspect the JSON results if a script needs to react to those findings. A report may contain error entries and still be written before the process exits with code 1. Do not treat its existence alone as success.

## Build from source and use the Rust library

From the repository root, with Rust installed:

```sh
cargo build --release -p flaccompagnon-cli
./target/release/flaccompagnon --help
```

On Windows the executable is `target\release\flaccompagnon.exe`. This build does not require the Tauri desktop runtime. Other Rust software can depend on the `flaccompagnon-core` crate directly; its functions and report types are documented in [Rustdoc](https://craft-and-code.github.io/FlacCompagnon/doc/). A Git dependency needs a pushed commit or tag; it does not need to wait for release binaries to finish building.

## Reuse results and choose report placement

```sh
flaccompagnon "Music/Artist" --json-layout album
flaccompagnon "Music/Artist" --json-layout artist
flaccompagnon "Music/Artist" --json-layout album --force
flaccompagnon "Music/Album" --show-results
```

`album` saves `<album>.json` inside each album folder; `artist` saves the same separate album reports in their parent folder. Audio files are grouped by directory; numbered `CD`, `Disc` and `Disk` subfolders share their parent album. This filesystem grouping does not use tags. `--json-layout` enables saving and cannot be combined with `--json PATH`.

Even without saving, existing FlacCompagnon reports beside albums or in their parent folders can supply successful results. Files absent from reports, changed files and previous errors are analyzed. Reports are snapshots checked by path, size and modification time; use `--force` if those metadata stayed unchanged despite an audio edit or to recompute with a newer engine. `--analysis` also enables terminal results and filters their fields.

Aède accepts the same `--force`, `--show-results` and `--json-layout album|artist` options for `aede analyze`; `--json` retains its default album placement.

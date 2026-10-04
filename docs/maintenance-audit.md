# Security and maintenance audit

This audit, performed on 4 October 2026, covers the analysis core, services, CLI, desktop commands, React state, tests, documentation and build workflows. It fixes reproduced defects and reduces avoidable allocations. It is not a certification that every possible audio file or desktop interaction is safe.

The maintainer excluded the Transcoded, Upscaling and Upsampling algorithms. Their implementation, dedicated tests and technical documentation remain unchanged. Changes to shared decoding reject malformed input without changing those heuristics or their thresholds.

## File and network safety

Conversion now writes and tags a sibling temporary file before publishing a complete output without replacing any existing destination. Batch planning rejects collisions, parent traversal and symbolic links below the selected output root. Copying neighbouring files skips the output subtree and preserves existing files. Cancellation cleanup receives only successfully published outputs and validates their location before deleting them.

Inline renaming uses a native exclusive rename rather than an existence check followed by a replacing rename. Two concurrent requests for the same name cannot overwrite each other's audio, and a dangling destination link is also preserved. If the platform/filesystem cannot provide this operation, renaming fails instead of falling back to replacement. File contents and metadata are not copied or rewritten.

Reports, playlists, extracted artwork and spectrograms use a shared atomic writer. Links and special destinations are rejected; replacing a hard-linked report does not truncate its other names. Fixed-format desktop exports enforce the appropriate extension, and the CLI requires `.json` for reports. Spectrogram PNG data comes from FFmpeg stdout, so FFmpeg never opens the output path. Both FFmpeg consumers restrict input protocols to `file,pipe`.

The automatically chosen `spectrograms` output folder must be a real directory: existing directory links, dangling links and file collisions are rejected before rendering. This prevents a pre-existing generated-folder link from redirecting PNG publication outside the source folder.

JSON report reads validate the opened file and enforce a 64 MiB budget. On Unix they open without following links and without blocking on a named pipe. CSV exports quote all imported text and neutralize spreadsheet formula prefixes. HTTP responses are limited while being read, including responses without `Content-Length`: 4 MiB for provider JSON, 12 MiB for images. Redirects must remain on HTTPS. Local and embedded artwork also has a 12 MiB budget and a 64 megapixel dimension limit.

Artwork imports share the same regular-file opening guard as reports. The dimension check covers PNG, JPEG, GIF, BMP, WebP and classic TIFF, including announced animation frames and chained TIFF pages. Unknown or invalid dimensions, BigTIFF, TIFF SubIFDs and BMP JPEG/PNG wrappers are rejected. These checks parse headers; they neither decode the complete raster nor impose a cumulative animation frame budget.

Tauri's CSP and capabilities were reviewed: scripts stay local, images use local/data sources, and no remote capability or filesystem/shell/HTTP plugin is enabled. Broad `core:default` and `dialog:default` grants were replaced with eight explicit event, window and file-dialog permissions. An automated frontend API inventory checks this list and listener cleanup. The application commands still trust the local frontend; their paths are not restricted to files previously chosen in a dialog. These grants do not establish a file-selection access-control boundary, and no WebView penetration test is claimed.

Discogs credentials now persist exclusively in macOS Keychain, Windows Credential Manager or Linux Secret Service through the native `keyring` adapter. Credential reads, including presence checks, stay in the backend; IPC returns presence rather than the saved token. Provider calls retrieve it for explicit Discogs requests. Save and Forget are explicit actions. Authorization headers are marked sensitive, and native errors are sanitized because some can contain secret bytes. Artwork requests carry no Discogs credential. Store operations run on blocking workers and share a process-local lock; no secret is cached in backend state and no plaintext fallback exists.

Startup removes the legacy browser entry before migration, preserves an already saved native token and shares initialization across React StrictMode replays. Failed migration retains a recovery copy only in session memory and reports the need to retry before closing. Failed browser cleanup reports a possible residual plaintext copy. Forget refuses native deletion until browser cleanup succeeds, so a successful Forget cannot be undone by migration of that surviving entry after a reload. Concurrent writes and edits made during a save have regressions. MusicBrainz searches and details remain independent of credential readiness.

Removing the active browser entry cannot erase older backups. Forget removes the application's credential, not its authorization at Discogs or requests already in flight. OS-vault protection depends on the user's unlocked session and store permissions; it does not protect against a compromised session, and no complete memory zeroization is claimed. The lock serializes this app process, not other instances or external vault editors. Linux needs an available Secret Service provider; failures remain visible rather than silently switching storage.

Audio decoding rejects invalid sample rates, channel counts, non-finite PCM and changing playback layouts. DSD subprocesses are killed and reaped when reading fails; incomplete final PCM frames are rejected. DSF/DFF headers validate chunk boundaries, channel identifiers, compression and required properties. FLAC STREAMINFO can no longer request an initial allocation approaching 1 GiB: the reservation hint is capped at 4 MiB and grows only with actual decoded samples.

## Measurements and asynchronous state

Clipping runs are tracked independently per channel. Interleaved silence can no longer interrupt another channel's clipped run, and adjacent channels cannot manufacture one. True peak includes the stored sample peak and drains its reconstruction filter at the end of a file. A short-signal regression uses a separately derived Blackman sinc convolution. Digital silence no longer triggers the fake stereo verdict. DR uses the unclamped float peak so constant gain does not change the crest factor above full scale; analytic fixtures also check selection of the loudest blocks.

The review also covered global/local phase, channel balance, HF stereo width, DC offset, integrated LUFS/LRA and momentary/short-term maxima, impulses and dropouts. Their existing analytical, injected-signal and EBU reference tests were reviewed; no additional defect was confirmed. Local phase, HF stereo and discontinuity findings remain descriptive evidence rather than universal authenticity or corruption verdicts. Loudness remains limited to mono/stereo because channel counts alone do not identify surround positions.

Stale tag, cover, lookup, playback and missing-file responses cannot overwrite newer state. Removed rows release cached tags and artwork. A save cannot clear edits made while it was in flight or discard a failed partial write. Cover imports retain their original selection and picture role. Native menu actions and keyboard shortcuts share the toolbar's busy guard. Backend play requests are queued before waiting in a blocking worker; stop invalidates pending playback, and extreme seeks return silence without integer overflow.

Relocation uses names and folder suffixes rather than audio fingerprints. Equal-ranking candidates remain missing instead of silently choosing an arbitrary file. Windows paths are recognized when importing a report on another platform. Relocated measurements should be refreshed by analysis.

## Performance evidence

The pure reorder benchmark is reproducible with `node tests/benchmark-row-reorder.mjs`. With 10,000 paths and 5,000 selected paths, one local ten-iteration run measured about 195 ms for the previous repeated array scans and 0.94 ms for the current set-based helpers. Both produce the same display order. These times exclude React, IPC and native drag events.

The browser fixture uses the production results table and search control. With 10,000 synthetic files, 48 rows were mounted in the tested viewport. Search, clearing an empty result, jumping to the final row and selecting all offscreen files were exercised. Interaction timings include two animation frames and are not CPU benchmarks. The fixture does not validate native file drops or an audio device.

Other changes prune generated spectrum folders before descending into them, avoid allocating lowercase extensions, reuse directory metadata and DR blocks, borrow PCM when MP3/Opus sample rates already match, reuse the Opus tail buffer and quantize WAV samples through an iterator. No whole-library throughput claim is made without a representative audio corpus and a before/after measurement.

Conversion and playback can still retain full decoded PCM. Their memory use therefore grows with duration, sample rate and channel count; removing extra copies does not make those paths constant in memory. Very long recordings need a separate streaming/buffering improvement and memory measurements.

LUFS/LRA gate histories and DR block histories also grow with duration. This pass reduces avoidable copies but does not establish a global RAM or CPU budget for extreme-length input.

## Dependency findings

The audit upgraded `h2` to 0.4.16 for [RUSTSEC-2026-0258](https://rustsec.org/advisories/RUSTSEC-2026-0258.html) and `rustls` to 0.23.45 for [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html). `Cargo.lock` is now retained, and CI, rustdoc and release builds use locked resolution. CI runs every JavaScript suite, the complete Rust workspace, strict Clippy, documentation checks and dependency audits.

The local-security follow-up upgraded the compatible Tauri family to 2.12.1. Its `tauri-utils` 2.10.1 / `urlpattern` 0.6 dependency chain replaces the five unmaintained `unic` crates. The resolved lockfile and a fresh advisory database confirm that all five alerts disappeared; the native credential adapter introduces no new advisory. [Tauri](https://docs.rs/crate/tauri/2.12.1), [tauri-utils](https://docs.rs/crate/tauri-utils/2.10.1), [urlpattern](https://docs.rs/crate/urlpattern/0.6.0).

The resulting Cargo audit reports zero entries classified as vulnerabilities and four warnings. The distinction matters: [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) reports unsound iterator implementations in Linux's transitive `glib` 0.18.5. The Tauri GTK dependency chain still selects this version; adding a second newer `glib` dependency would not repair it. The remaining three maintenance advisories affect [audiopus_sys](https://rustsec.org/advisories/RUSTSEC-2026-0150.html), [paste](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) and [proc-macro-error](https://rustsec.org/advisories/RUSTSEC-2024-0370.html). No advisory is suppressed. `npm audit` reports zero vulnerabilities.

Further options are a maintained Opus binding, an upstream `paste` replacement in Lofty, and a coordinated GTK/GLib migration or narrowly scoped backport. A new direct dependency cannot remove the affected transitive branch. Small adapters and focused corrections can be maintained here; rewriting codecs or the desktop platform would add substantially more code to validate. These remaining replacements have not been implemented or validated by this pass.

## Verification and remaining limits

The complete reproducible commands are listed in the [project README](../README.md#testing). Frontend state tests control the order of backend responses; parser tests include malformed inputs; export tests check that audio files remain unchanged. Independent FFmpeg comparisons cover loudness and DC offset, and a real FFmpeg render checks the PNG output path.

Checks run locally on macOS with Rust 1.99.0 and Node 26.10.0:

| Check                                                            | Result                                                |
| ---------------------------------------------------------------- | ----------------------------------------------------- |
| `npx tsc --noEmit` and `npm run build`                           | Passed                                                |
| `npm test`                                                       | 108 passed                                            |
| AppImage Python regression suite                                 | 6 passed                                              |
| `cargo test --workspace --locked`                                | 528 passed; 3 optional native/reference tests ignored |
| Optional loudness/DC FFmpeg comparisons, run separately          | Both passed                                           |
| Optional native credential round-trip and cleanup                | Passed on macOS                                       |
| Artwork tests after the final TIFF iteration cleanup             | 25 passed                                             |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed                                                |
| `cargo build --workspace --locked`                               | Passed                                                |
| Strict workspace rustdoc with `RUSTDOCFLAGS='-D warnings'`       | Passed                                                |
| Rust formatting and `git diff --check`                           | Passed                                                |
| Markdown formatting and documentation site build                 | Passed; 55 pages and 2,789 local links checked        |

Compiler warnings and frontend unused-symbol checks were addressed. The historical public `requant` subsystem has no production caller but is retained because it belongs to the excluded Transcoded scope. Public library APIs cannot be declared dead solely because this repository has no caller. Existing large orchestration components, especially `App.tsx` and `ResultsTable.tsx`, still exceed the repository's component size convention; splitting them requires a focused structural refactor with desktop interaction checks.

`CLAUDE.md` now directs readers to the shared `AGENTS.md` contract instead of duplicating outdated architecture conventions. Frontend payload comments also name the current services crate.

This pass does not include fuzzing, a representative independent musical corpus to estimate false-positive rates, native audio-device tests, or new Windows/Linux installer runs. Synthetic/reference tests demonstrate specific properties; they do not establish universal detection accuracy. Filesystem checks also do not provide a confinement guarantee against another local process concurrently replacing directory ancestors.

Systematic fuzzing would exercise each parser repeatedly with automatically generated and mutated inputs, track code coverage, enforce resource/time budgets and retain minimized inputs that reproduce crashes, panics or hangs. Each confirmed defect would gain a deterministic regression. Current malformed-input unit tests are useful but are not a continuous fuzzing campaign. [LLVM libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html).

The native exclusive rename tests ran on macOS. Run `cargo test -p flaccompagnon-services --locked rename::tests` on Linux and Windows as well; the Windows long-path branch and its regression were not executed locally.

Credential unit tests use an isolated in-memory vault and never touch the maintainer's account. The optional native round-trip test writes, reads and deletes a synthetic entry under a unique temporary service name; it passed on macOS with an unlocked Keychain. Run `cargo test -p flaccompagnon --locked credentials::tests::native_vault_round_trip_and_cleanup -- --ignored` on Windows and Linux with an available store. Those native adapters and interactive desktop drop/menu checks were not exercised locally by this follow-up.

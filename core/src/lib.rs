//! FlacCompagnon core analysis library.
//!
//! This crate is intentionally free of any Tauri / UI dependency so that every
//! analysis routine can be unit-tested in isolation with plain `cargo test`.
//!
//! Public entry points:
//! * [`analyze_file`]  — analyze a single audio file.
//! * [`analyze_folder`] — analyze every supported audio file inside a folder
//!   (non-recursive by default; see [`ScanOptions`]).
//!
//! # Layout
//!
//! This file is the crate's front door and nothing else: module declarations
//! and the re-exports that keep the public API flat (`core::FileAnalysis`,
//! not `core::types::FileAnalysis`). The work is one layer down:
//!
//! * [`pipeline`] — orchestrates one file: pick a decode path, run the
//!   analyzer, assemble the verdict.
//! * [`scan`] — which files to analyze, and analyzing a folder of them.
//! * [`types`] — the shapes that cross the crate boundary, mirrored by the
//!   frontend's `src/types.ts` and by the saved JSON report.
//! * [`decode`] — one module per decode path.
//! * [`analyzer`] — the single streaming pass that produces every measurement.
//! * [`detections`] — measurements in, verdict out.
//!
//! # Example
//!
//! Analyzing one file and reading its verdict. Audio files are only ever
//! opened read-only — nothing is written unless you call [`report::write_csv`]
//! or [`report::write_json`] yourself.
//!
//! ```no_run
//! use std::path::Path;
//! use flaccompagnon_core::{analyze_file, ScanOptions, TranscodeState};
//!
//! let report = analyze_file(Path::new("track.flac"), &ScanOptions::default());
//!
//! if let Some(err) = &report.error {
//!     eprintln!("could not analyze: {err}");
//! } else {
//!     println!("{} — {}", report.file_name, report.detections.summary);
//!     if report.detections.upscaling {
//!         println!("  declared {:?} bits, really {:?}",
//!                  report.declared_bits, report.real_bit_depth);
//!     }
//!     if report.detections.transcoding == TranscodeState::Detected {
//!         println!("  lossy source: {}", report.detections.detail);
//!     }
//! }
//! ```
//!
//! Scanning a whole folder and exporting the result:
//!
//! ```no_run
//! use std::path::Path;
//! use flaccompagnon_core::{analyze_folder, report, ScanOptions};
//!
//! let opts = ScanOptions { recursive: true, ..ScanOptions::default() };
//! let folder = analyze_folder(Path::new("/music/album"), &opts)?;
//!
//! println!("{} files, {} flagged", folder.files.len(),
//!          folder.files.iter().filter(|f| f.detections.summary == "Flagged").count());
//!
//! report::write_csv(Path::new("album.csv"), &folder)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

// This crate is published as rustdoc (see `.github/workflows/docs.yml`), so an
// undocumented public item is a gap in what ships, not just a style nit. Warn
// rather than deny: a new public item without a doc comment should show up in
// `cargo doc`/`cargo build` output for the author to notice, without turning
// an otherwise-green build red on CI while someone is mid-refactor.
#![warn(missing_docs)]

pub mod analyzer;
pub mod bitdepth;
pub mod clipping;
pub mod decode;
pub mod detections;
pub mod dsd;
pub mod flac_md5;
pub mod mdct;
pub mod pipeline;
pub mod playlist;
pub mod report;
pub mod requant;
pub mod scan;
pub mod spectrum;
pub mod stereo;
pub mod tags;
pub mod truepeak;
pub mod types;

pub use decode::{probe_info, BasicInfo};
pub use detections::{Detections, TranscodeState};
pub use flac_md5::FlacMd5Status;
pub use pipeline::analyze_file;
pub use scan::{analyze_folder, is_supported_audio, list_audio_files, SUPPORTED_EXTENSIONS};
pub use types::{AnalysisError, ClippingInfo, FileAnalysis, FolderReport, ScanOptions};

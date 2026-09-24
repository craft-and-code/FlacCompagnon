//! The analysis chain: audio samples in, a verdict out.
//!
//! Everything here is internal machinery — no module in this folder is
//! referenced from outside the crate, which is what made it possible to group
//! them without touching a single caller. What crosses the crate boundary is
//! the *result* ([`crate::types`], [`detections::Detections`]), not the
//! measuring.
//!
//! These metric modules used to be top-level files next to `report.rs`, `playlist.rs`
//! and `scan.rs`, which made the crate root read as a flat pile where the
//! export helpers and the DSP sat at the same level. They are one subsystem
//! and they change together — a new detection touches the analyzer, a metric
//! module and the verdict logic in one go — so they belong under one roof.
//!
//! ## Shape of the chain
//!
//! One pass over the samples, then one decision:
//!
//! * [`analyzer`] runs that single streaming pass and owns the state it needs
//!   across packets. It is the only module here the decode paths talk to.
//! * The per-metric modules do one measurement each: [`spectrum`] (FFT, cut-off),
//!   [`bitdepth`] (effective depth),
//!   [`stereo`] (dual-mono disguised as stereo), [`clipping`] (full-scale
//!   runs), [`truepeak`] (inter-sample peaks, dBTP), [`loudness`] (integrated LUFS).
//! * [`mdct`] and [`requant`] are the AAC-grid detection: the transform, then
//!   the search for coefficients that fall back onto the quantization lattice
//!   only an AAC encoder produces.
//! * [`detections`] takes the finished measurements and turns them into the
//!   verdict the UI shows. It is the only module here that decides anything;
//!   the rest only measure.
//!
//! Keeping the measuring and the deciding apart is deliberate: a threshold
//! that moves should move in [`detections`] alone, without touching the code
//! that produced the number it compares.

pub mod analyzer;
pub mod bitdepth;
pub mod clipping;
pub mod dc_offset;
pub mod detections;
pub mod discontinuities;
pub mod intensity_stereo;
pub mod loudness;
pub mod mdct;
pub mod requant;
pub mod spectrum;
pub mod stereo;
pub mod truepeak;

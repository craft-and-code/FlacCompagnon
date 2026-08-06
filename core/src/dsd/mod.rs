//! DSD (Direct Stream Digital) support.
//!
//! Two genuinely separate jobs, one per file:
//!
//! * [`container`] — **exact** parsing of DSF (Sony) and DFF (Philips DSDIFF)
//!   headers: magic bytes, sample rate, channel count, sample count, DST
//!   compression flag. Byte-level format work, testable against a handcrafted
//!   header, with no notion of audio content.
//! * [`spectral`] — **calibrated heuristics** over an already-decoded spectrum:
//!   spotting DSD that was converted from a PCM source ("fake DSD"), and the
//!   mirror case of hi-res PCM that came from a DSD master. Signal processing
//!   with measured thresholds, no notion of file layout.
//!
//! They were one 400-line file until the two halves were told apart; nothing
//! in the first half calls anything in the second. Both are re-exported here,
//! so callers keep writing `core::dsd::parse` / `core::dsd::DsdInfo`.

pub mod container;
pub mod spectral;

pub use container::{parse, DsdInfo, DSD64_RATE};
pub use spectral::{dsd_heritage_check, pcm_source_check, PcmSourceCheck, PCM_CLIFF_DB};

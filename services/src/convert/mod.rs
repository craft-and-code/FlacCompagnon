//! Convert PCM audio to FLAC, Opus, MP3, or 16-bit WAV.
//!
//! Analysis remains in `flaccompagnon-core`; this services module owns the
//! files it writes. Each conversion finishes audio encoding, metadata copying,
//! and optional timestamp preservation in a temporary file beside its output.
//! Publishing never replaces an existing file, including a source file, and
//! failed or cancelled conversions remove their temporary output.
//!
//! FLAC uses the pure-Rust `flacenc` encoder. Opus and MP3 vendor their C
//! codecs; building Opus requires the autotools documented in the README.
//! DSD sources fail with [`ConvertError::Unsupported`] because the shared PCM
//! decoder does not handle them.
//!
//! Source tags are always copied. A metadata failure returns a warning in
//! [`ConvertOutcome`] while preserving the converted audio. Encoders live in
//! their own modules; `layout` plans destinations and copies neighbouring files,
//! `pcm` shares sample reshaping, and `cleanup` removes a cancelled batch's
//! published outputs.

mod cleanup;
mod flac;
mod layout;
mod mp3;
mod opus;
mod output;
mod paths;
mod pcm;
mod wav;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::decode;

pub use cleanup::undo_batch;
pub use layout::{passthrough_files, plan_batch, ConvertSource};
pub(crate) use pcm::{f32_to_ints, resample_linear};

/// A conversion target format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConvertFormat {
    /// Lossless — the default, and the only lossless target here besides WAV.
    Flac,
    /// Lossy, royalty-free — the modern choice when size matters.
    Opus,
    /// Lossy — kept for compatibility with players/hardware that don't speak
    /// Opus.
    Mp3,
    /// Uncompressed PCM. Not really a "conversion" so much as a guaranteed
    /// real copy — useful for a file this app has flagged as fake-lossless,
    /// where the point is a known-honest baseline rather than smaller files.
    Wav,
}

impl ConvertFormat {
    /// The file extension a converted file gets, without a leading dot.
    pub fn extension(self) -> &'static str {
        match self {
            ConvertFormat::Flac => "flac",
            ConvertFormat::Opus => "opus",
            ConvertFormat::Mp3 => "mp3",
            ConvertFormat::Wav => "wav",
        }
    }
}

/// Default Opus bitrate: comfortably transparent for music at this codec's
/// efficiency (see Opus's own listening-test results), well below where
/// diminishing returns set in.
pub const DEFAULT_OPUS_KBPS: u32 = 160;
/// Default MP3 bitrate — LAME's own commonly-recommended "high quality" rate.
pub const DEFAULT_MP3_KBPS: u32 = 256;

/// How hard the FLAC encoder should work.
///
/// Deliberately *not* libFLAC's `-0`..`-8`. Those are presets defined over
/// libFLAC's own implementation, and this app encodes with `flacenc`, whose
/// knobs are its own — labelling a setting "-5" would claim a byte-for-byte
/// equivalence the output does not have. What the user actually chooses is
/// the trade they care about, so that is what the scale says.
///
/// Every level is lossless. The only thing that varies is how long the
/// encoder spends looking for a smaller representation of the same audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlacEffort {
    /// Shortest predictor search, no mid-side stereo. Noticeably quicker on a
    /// large batch, files a few percent larger.
    Fast,
    /// `flacenc`'s own defaults — the sensible middle, and the default here.
    #[default]
    Balanced,
    /// Longer predictor search and mid-side stereo. Slower, smallest files;
    /// the returns are small enough that this is worth it for archiving
    /// rather than for a batch you are waiting on.
    Maximum,
}

/// What to convert to, and how.
///
/// Some fields only apply to some formats — `bitrate_kbps` to the two lossy
/// ones, `flac_effort` to FLAC — and are simply ignored elsewhere rather than
/// modelled per format: one flat struct crosses the Tauri boundary as one
/// JSON object, and the panel already only shows each control where it means
/// something.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertSettings {
    /// The output format.
    pub format: ConvertFormat,
    /// Target bitrate in kbps, for [`ConvertFormat::Opus`]/[`ConvertFormat::Mp3`].
    /// `None` falls back to [`DEFAULT_OPUS_KBPS`]/[`DEFAULT_MP3_KBPS`].
    pub bitrate_kbps: Option<u32>,
    /// How hard to work when the target is FLAC. Ignored for other formats.
    #[serde(default)]
    pub flac_effort: FlacEffort,
    /// Give the converted file the source's last-modified date instead of
    /// "now".
    ///
    /// Applies to *every* format, which is why it lives here rather than
    /// among the FLAC settings where it was first asked for: it is a property
    /// of the file that gets written, not of the codec that wrote it. Worth
    /// having because a converted library otherwise arrives with every track
    /// stamped the same second, destroying any "sort by date added" ordering
    /// the original had.
    #[serde(default)]
    pub preserve_modtime: bool,
}

/// Errors that can occur while converting one file. Every variant carries the
/// path it happened to, the same shape as [`crate::tags::TagError`], since a
/// batch conversion reports failures per file rather than aborting the rest.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// Could not create the destination folder, or write the output file
    /// (path, reason).
    #[error("{0}: {1}")]
    Io(String, String),
    /// The source file could not be decoded (path, reason).
    #[error("{0}: could not decode the source file ({1})")]
    Decode(String, String),
    /// The encoder rejected the audio or failed mid-encode (path, reason).
    #[error("{0}: could not encode the output file ({1})")]
    Encode(String, String),
    /// The source format, or something about it, isn't convertible yet
    /// (path, reason).
    #[error("{0}: {1}")]
    Unsupported(String, String),
    /// The caller asked to stop partway through this file (path). Distinct
    /// from the other variants because it isn't a failure to report to the
    /// user — a cancelled batch is discarded whole (see [`undo_batch`]), and
    /// counting this file as "failed" in the summary would be misleading.
    #[error("{0}: cancelled")]
    Cancelled(String),
}

/// What a successful [`convert_file`] produced.
///
/// Exists for one reason: "the audio converted, its tags did not" is a real
/// third outcome, and neither `Ok(())` nor `Err` can say it. Folding it into
/// the error would mark a perfectly good file as failed; dropping it would
/// let a file lose its title and cover in silence.
#[derive(Debug, Clone, Default)]
pub struct ConvertOutcome {
    /// `Some` when the file was written but its tags could not be copied
    /// across, carrying the reason. Not an error: the audio is intact.
    pub tag_warning: Option<String>,
}

/// Convert `src` to `dest` per `settings`. `dest`'s parent folder is created
/// if it doesn't exist yet (see [`plan_batch`], which is what typically
/// produces `dest` in the first place).
///
/// `is_cancelled` is polled while decoding — once per packet, and again just
/// before the encoder starts — and returns [`ConvertError::Cancelled`]
/// without writing anything as soon as it answers `true`. Without it, a
/// cancelled batch still has to wait for every in-flight file to decode *and*
/// re-encode in full before it can stop, which on a long hi-res track is
/// several seconds of a UI that looks hung; the encode itself is one opaque
/// call into a codec crate and stays uninterruptible, so the decode half is
/// what there is to give back.
///
/// Returns a [`ConvertOutcome`] rather than `()` because copying the source's
/// tags onto the result can fail on its own, without the conversion having
/// failed — see that type.
pub fn convert_file(
    src: &Path,
    dest: &Path,
    settings: &ConvertSettings,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<ConvertOutcome, ConvertError> {
    let name = || src.display().to_string();

    if matches!(lower_ext(src).as_deref(), Some("dsf" | "dff")) {
        return Err(ConvertError::Unsupported(
            name(),
            "DSD conversion isn't supported yet".to_string(),
        ));
    }
    // Checked before the source is even opened, not only inside the decode
    // loop: a worker that picks up its next file just as Cancel lands should
    // not pay for opening and probing it first.
    if is_cancelled() {
        return Err(ConvertError::Cancelled(name()));
    }

    let pcm = pcm::decode_cancellable(src, is_cancelled)?;
    if is_cancelled() {
        return Err(ConvertError::Cancelled(name()));
    }
    let output = output::StagedOutput::new(dest, settings.format)?;
    let staged = output.path();

    match settings.format {
        ConvertFormat::Flac => {
            flac::encode(&pcm, staged, source_bit_depth(src), settings.flac_effort)
        }
        ConvertFormat::Wav => wav::encode(&pcm, staged),
        ConvertFormat::Opus => opus::encode(
            &pcm,
            staged,
            settings.bitrate_kbps.unwrap_or(DEFAULT_OPUS_KBPS),
        ),
        ConvertFormat::Mp3 => mp3::encode(
            &pcm,
            staged,
            settings.bitrate_kbps.unwrap_or(DEFAULT_MP3_KBPS),
        ),
    }?;

    // Deliberately not `?`. By this point the audio is written and correct;
    // failing the whole file over its metadata would throw away a good
    // conversion and, worse, make the batch report it as failed — sending the
    // user looking for a file that is actually sitting there, complete. The
    // problem is still surfaced, just as what it is: a warning about one
    // file's tags, not a failed conversion.
    let tag_warning = crate::tags::copy_tags(src, staged)
        .err()
        .map(|e| e.to_string());

    // After the tags, never before: writing them rewrites the file and would
    // stamp it "now" again, silently undoing this.
    if settings.preserve_modtime {
        copy_modtime(src, staged)?;
    }
    if is_cancelled() {
        return Err(ConvertError::Cancelled(name()));
    }
    output.publish(dest)?;
    Ok(ConvertOutcome { tag_warning })
}

/// The bit depth to encode FLAC output at: the source's own declared depth
/// when there is one, clamped to 16–24 (FLAC's practical range, and the
/// range every encoder here is expected to see), 16 otherwise — a header
/// read, not a decode, so this costs nothing next to the decode
/// [`convert_file`] already did.
fn source_bit_depth(src: &Path) -> u32 {
    decode::probe_info(src)
        .ok()
        .and_then(|info| info.bits)
        .map(|b| b.clamp(16, 24))
        .unwrap_or(16)
}

fn lower_ext(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

/// Give `dest` the same last-modified time as `src`.
///
/// Only the modification time, not the creation time: the converted file was
/// genuinely created now, and claiming otherwise would be a lie a backup tool
/// might act on. What the user wants preserved is the ordering — when the
/// music was added — and that is what mtime carries.
fn copy_modtime(src: &Path, dest: &Path) -> Result<(), ConvertError> {
    let io_err = |e: std::io::Error| ConvertError::Io(dest.display().to_string(), e.to_string());
    let modified = std::fs::metadata(src)
        .and_then(|m| m.modified())
        .map_err(io_err)?;
    std::fs::File::open(dest)
        .and_then(|f| f.set_modified(modified))
        .map_err(io_err)
}

#[cfg(test)]
#[path = "../../tests/unit/convert/mod.rs"]
mod tests;

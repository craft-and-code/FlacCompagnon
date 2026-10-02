//! FLAC integrity checks that do not run the authenticity detection pipeline.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use flaccompagnon_core::{
    decode::{flac_md5_signature, verify_flac_md5},
    AnalysisError, FileAnalysis, FlacMd5Status,
};

use crate::{args::Args, display::flac_md5_label, progress, reports};

pub(crate) fn run(
    paths: &[PathBuf],
    cached: &BTreeMap<PathBuf, FileAnalysis>,
    args: &Args,
) -> Result<(), String> {
    let mut failed = false;
    for (index, path) in paths.iter().enumerate() {
        let existing = cached.get(&reports::path_key(path)).filter(|file| {
            reports::matches_requested(file, path, args)
                && matches!(
                    file.flac_md5,
                    Some(FlacMd5Status::Match | FlacMd5Status::NoSignature)
                )
        });
        let status = if let Some(file) = existing {
            eprintln!(
                "  ▸ [{}/{}] Reusing {}",
                index + 1,
                paths.len(),
                path.display()
            );
            file.flac_md5.clone()
        } else {
            progress::with_analysis(path, index, paths.len(), || {
                if path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("flac"))
                {
                    Some(
                        verify(path, args.verify_flac_md5)
                            .unwrap_or_else(|error| FlacMd5Status::Error(error.to_string())),
                    )
                } else {
                    None
                }
            })
        };
        failed |= matches!(
            status,
            Some(FlacMd5Status::Mismatch | FlacMd5Status::Error(_))
        );
        println!("  FLAC MD5: {}", flac_md5_label(status.as_ref()));
    }
    if failed {
        Err("one or more FLAC integrity checks failed".into())
    } else {
        Ok(())
    }
}

fn verify(path: &Path, enabled: bool) -> Result<FlacMd5Status, AnalysisError> {
    if !enabled {
        return verify_flac_md5(path, false);
    }
    let signature = flac_md5_signature(path)?;
    // Native libFLAC's optimized decoder is available through the optional
    // `flac` tool. A failed native check must never be masked by a fallback.
    match Command::new("flac")
        .args(["--silent", "--test", "--"])
        .arg(path)
        .stdin(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => Ok(if signature == FlacMd5Status::Present {
            FlacMd5Status::Match
        } else {
            signature
        }),
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr);
            Err(AnalysisError::Decode(format!(
                "flac -t failed: {}",
                detail.trim()
            )))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => verify_flac_md5(path, true),
        Err(error) => Err(AnalysisError::Decode(format!(
            "cannot run flac -t: {error}"
        ))),
    }
}

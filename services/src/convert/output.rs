//! Publishing complete conversion outputs without replacing an existing file.

use std::path::Path;

use super::{ConvertError, ConvertFormat};

pub(super) struct StagedOutput(tempfile::NamedTempFile);

impl StagedOutput {
    pub(super) fn new(dest: &Path, format: ConvertFormat) -> Result<Self, ConvertError> {
        let io_error =
            |error: std::io::Error| ConvertError::Io(dest.display().to_string(), error.to_string());
        // symlink_metadata also rejects dangling symlinks. An existence check
        // that followed their targets would miss them and permit replacement.
        match std::fs::symlink_metadata(dest) {
            Ok(_) => return Err(io_error(std::io::ErrorKind::AlreadyExists.into())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
        let parent = dest
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(io_error)?;
        // Tag probing uses the extension, so the scratch path must retain the
        // output format. Keeping it beside dest also permits an atomic publish.
        tempfile::Builder::new()
            .prefix(".flaccompagnon-")
            .suffix(&format!(".{}", format.extension()))
            .tempfile_in(parent)
            .map(Self)
            .map_err(io_error)
    }

    pub(super) fn path(&self) -> &Path {
        self.0.path()
    }

    pub(super) fn publish(self, dest: &Path) -> Result<(), ConvertError> {
        self.0
            .persist_noclobber(dest)
            .map(|_| ())
            .map_err(|error| ConvertError::Io(dest.display().to_string(), error.error.to_string()))
    }
}

#[cfg(test)]
#[path = "../../tests/unit/convert/output.rs"]
mod tests;

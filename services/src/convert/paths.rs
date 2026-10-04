//! Validate conversion destinations and resolve output-directory aliases.

use std::path::{Component, Path, PathBuf};

// A deliberately selected root may itself be a symlink. Descendants are
// different: following one would place output outside the folder selected.
pub(super) fn validate_output_directory(root: &Path, dest: &Path) -> std::io::Result<()> {
    let relative = dest.strip_prefix(root).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "output escapes its root")
    })?;
    // Validate the whole suffix before disk probing can stop at a missing
    // ancestor; a later '..' must never bypass the traversal guard.
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid output directory",
        ));
    }
    let mut directory = root.to_path_buf();
    for component in relative.parent().into_iter().flat_map(Path::components) {
        directory.push(component);
        match directory.symlink_metadata() {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "output directory contains a symbolic link",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub(super) fn resolve_output_root(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut prefix = absolute.as_path();
    let mut tail = Vec::new();
    loop {
        match prefix.canonicalize() {
            Ok(mut resolved) => {
                for component in tail.into_iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = prefix.file_name().ok_or(error)?;
                tail.push(name.to_os_string());
                prefix = prefix.parent().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid output root")
                })?;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/convert/paths.rs"]
mod tests;

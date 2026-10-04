//! Bounded report reads and atomic replacement of an explicitly chosen report.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use crate::FolderReport;

use super::{build_csv, build_json, parse_json};

/// Resource budget for an imported JSON report: 64 MiB, ample for large libraries.
/// Both the metadata and the bytes actually read are checked, including files
/// that grow while being read.
pub const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;

/// Reject links and special files before replacing an export destination.
/// A missing destination is allowed; its parent must exist when writing.
pub fn validate_destination(dest: &Path) -> io::Result<()> {
    match dest.symlink_metadata() {
        Ok(metadata) if !metadata.file_type().is_file() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Report destination must be a regular file, not a symbolic link or directory.",
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Read a regular JSON report with a bounded allocation.
/// The opened handle is checked, and Unix opens are nonblocking without
/// following links, so a concurrent path swap cannot stall cache discovery.
pub fn read_json(path: &Path) -> Result<FolderReport, String> {
    let file = open_regular_file(path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if metadata.len() > MAX_JSON_BYTES {
        return Err("JSON report exceeds the 64 MiB import limit.".into());
    }
    let mut text = String::new();
    file.take(MAX_JSON_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    if text.len() as u64 > MAX_JSON_BYTES {
        return Err("JSON report exceeds the 64 MiB import limit.".into());
    }
    parse_json(&text)
}

/// Open a regular file without following links or accepting special files.
/// The opened handle is validated because a concurrent rename could replace
/// a previously checked file with a link or pipe. Unix opens are nonblocking.
pub fn open_regular_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_REPARSE_POINT opens the link itself, not its target.
        options.custom_flags(0x0020_0000);
    }
    let file = options.open(path)?;
    if !file.metadata()?.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Input must be a regular file, not a symbolic link or directory.",
        ));
    }
    Ok(file)
}

/// Atomically replace an exported file with `bytes`, rejecting links and special files.
/// Renaming a sibling temporary file avoids truncating an existing export on
/// an interrupted write, and never follows a destination link to an audio file.
pub fn write_atomic_bytes(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    validate_destination(dest)?;
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(dest).map_err(|error| error.error)?;
    Ok(())
}

/// Atomically write the CSV report to `dest`, rejecting links and special files.
pub fn write_csv(dest: &Path, report: &FolderReport) -> io::Result<()> {
    write_atomic_bytes(dest, build_csv(report).as_bytes())
}

/// Atomically write the JSON report to `dest`, rejecting links and special files.
pub fn write_json(dest: &Path, report: &FolderReport) -> io::Result<()> {
    let text =
        build_json(report).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_atomic_bytes(dest, text.as_bytes())
}

#[cfg(test)]
#[path = "../../tests/unit/report/file.rs"]
mod tests;

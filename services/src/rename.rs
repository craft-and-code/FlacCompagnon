//! Rename a regular file without replacing any existing directory entry.

use std::{fs, io, path::Path};

/// Rename `source` to an unused `destination` in the same directory.
///
/// The native operation moves the source entry exclusively, retaining its
/// contents and metadata without a copy or a separate source deletion. A
/// concurrent rename therefore cannot replace another track, including a
/// dangling destination symlink. Source symlinks and directories are refused.
///
/// macOS, Linux and Windows are supported. An OS or filesystem without an
/// exclusive rename primitive returns an error; there is no replacing or
/// copying fallback. A concurrently changed source entry is moved atomically,
/// rather than following its symlink target or deleting a later replacement.
pub fn rename_file_noclobber(source: &Path, destination: &Path) -> io::Result<()> {
    if source.parent() != destination.parent() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "renaming must keep the file in its original directory",
        ));
    }
    if !fs::symlink_metadata(source)?.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only regular files can be renamed",
        ));
    }
    if source == destination {
        return Ok(());
    }
    rename_exclusive(source, destination)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "source contains a null byte"))?;
    let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination contains a null byte",
        )
    })?;
    // SAFETY: both C strings are NUL-terminated with no internal NUL, remain
    // alive for the call, and the native API only reads them. Exclusive flags
    // prevent replacement in the same operation that moves the source.
    let result = unsafe {
        #[cfg(target_os = "linux")]
        {
            libc::renameat2(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        }
        #[cfg(target_os = "macos")]
        {
            // Darwin rename(2): RENAME_EXCL returns EEXIST for an existing entry.
            libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL)
        }
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "windows")]
fn rename_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    // Rust canonicalizes the directory to a verbatim path, so MoveFileExW
    // supports long names without relying on a system/manifest MAX_PATH opt-in.
    // Keep the final names unresolved: a replaced source link is moved as an
    // entry, rather than following it to another user's file.
    let parent = source
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent)?;
    let source =
        parent.join(source.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid source file name")
        })?);
    let destination = parent.join(destination.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid destination file name")
    })?);
    // MoveFileExW's flags=0 neither replaces an existing entry nor simulates a
    // cross-volume move using copy/delete. Official contract:
    // https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }
    let to_wide = |path: &Path| -> io::Result<Vec<u16>> {
        let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
        if value.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path contains a null character",
            ));
        }
        value.push(0);
        Ok(value)
    };
    let source = to_wide(&source)?;
    let destination = to_wide(&destination)?;
    // SAFETY: both pointers reference live, NUL-terminated UTF-16 buffers.
    // The API reads the buffers during the call, and destination is non-null.
    let result = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) };
    if result != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn rename_exclusive(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "exclusive file renaming is unsupported on this platform",
    ))
}

#[cfg(test)]
#[path = "../tests/unit/rename.rs"]
mod tests;

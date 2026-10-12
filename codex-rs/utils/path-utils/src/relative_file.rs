//! Pinned local relative reads, for trusted hosts consuming untrusted task files.
use std::fs::File;
use std::io;
use std::path::Path;

/// Open a regular, single-link file without following any relative symlink.
/// The root is trusted host data; callers must bound the file's size/read length.
#[cfg(unix)]
pub fn open_relative_regular_file(root: &Path, relative: &Path) -> io::Result<File> {
    use std::ffi::CString;
    use std::fs::OpenOptions;
    use std::os::fd::AsRawFd;
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Component;
    let mut parts = relative.components().peekable();
    if relative.as_os_str().is_empty() || !parts.clone().all(|p| matches!(p, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "relative file path must contain only normal components",
        ));
    }
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)?;
    while let Some(part) = parts.next() {
        let name = CString::new(part.as_os_str().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if parts.peek().is_some() {
                libc::O_DIRECTORY
            } else {
                libc::O_NONBLOCK
            };
        // SAFETY: live pinned directory and a terminated, single relative name.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat returns a fresh owned file descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        if parts.peek().is_none() {
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(io::Error::other(
                    "relative task file must be regular and single-link",
                ));
            }
            return Ok(file);
        }
        directory = file;
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "missing relative file name",
    ))
}

#[cfg(not(unix))]
pub fn open_relative_regular_file(_root: &Path, _relative: &Path) -> io::Result<File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "pinned relative task reads require the qualified Unix backend",
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn pinned_reader_rejects_ancestor_leaf_and_hard_link_redirection() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("nested")).unwrap();
        std::fs::write(root.path().join("nested/file"), "declared bytes").unwrap();
        let mut content = String::new();
        open_relative_regular_file(root.path(), Path::new("nested/file"))
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "declared bytes");
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret"), root.path().join("leaf"))
            .unwrap();
        assert!(open_relative_regular_file(root.path(), Path::new("escape/secret")).is_err());
        assert!(open_relative_regular_file(root.path(), Path::new("leaf")).is_err());
        assert!(open_relative_regular_file(root.path(), Path::new("../secret")).is_err());
        assert!(open_relative_regular_file(root.path(), Path::new("/etc/passwd")).is_err());
        std::fs::hard_link(root.path().join("nested/file"), root.path().join("alias")).unwrap();
        assert!(open_relative_regular_file(root.path(), Path::new("nested/file")).is_err());
        assert!(open_relative_regular_file(root.path(), Path::new("nested")).is_err());
    }
}

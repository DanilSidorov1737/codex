//! Owner-side snapshot for Neovim: never give the editor an agent-writable path.
use anyhow::{Context, Result, ensure};
use codex_app_server_protocol::OrganizationArtifactEvidence;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) struct Preview {
    directory: tempfile::TempDir,
    root: PathBuf,
    relative: String,
    expected: Option<OrganizationArtifactEvidence>,
    hash: String,
}
impl Preview {
    pub(crate) fn new(
        home: &Path,
        root: &Path,
        relative: &str,
        expected: Option<&OrganizationArtifactEvidence>,
    ) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix(".organization-preview-")
            .tempdir_in(home)?;
        let mut preview = Self {
            directory,
            root: root.into(),
            relative: relative.into(),
            expected: expected.cloned(),
            hash: String::new(),
        };
        preview.refresh()?;
        Ok(preview)
    }
    pub(crate) fn path(&self) -> PathBuf {
        self.directory.path().join("artifact")
    }
    pub(crate) fn refresh(&mut self) -> Result<bool> {
        let file =
            codex_utils_path::open_relative_regular_file(&self.root, Path::new(&self.relative))?;
        ensure!(
            file.metadata()?.len() <= 8 * 1024 * 1024,
            "editor artifact exceeds 8 MiB"
        );
        let mut bytes = Vec::new();
        file.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "editor artifact grew beyond 8 MiB"
        );
        let hash = format!("{:x}", Sha256::digest(&bytes));
        if let Some(expected) = &self.expected {
            ensure!(
                expected.sha256 == hash && expected.bytes == bytes.len() as u64,
                "artifact changed after recorded review; refresh execution evidence before viewing it"
            );
        }
        if hash == self.hash {
            return Ok(false);
        }
        let mut staging = tempfile::NamedTempFile::new_in(self.directory.path())?;
        staging.write_all(&bytes)?;
        staging.as_file().sync_all()?;
        staging
            .persist(self.path())
            .context("persist protected editor preview")?;
        self.hash = hash;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_preview_tracks_real_bytes_without_redirecting_neovim_to_agent_paths() {
        let home = tempfile::tempdir().unwrap();
        let task = tempfile::tempdir().unwrap();
        std::fs::write(task.path().join("report"), b"agent one").unwrap();
        let mut preview = Preview::new(home.path(), task.path(), "report", None).unwrap();
        assert!(preview.path().starts_with(home.path()));
        assert!(!preview.path().starts_with(task.path()));
        assert_eq!(std::fs::read(preview.path()).unwrap(), b"agent one");
        assert!(!preview.refresh().unwrap());
        std::fs::write(task.path().join("report"), b"agent two").unwrap();
        assert!(preview.refresh().unwrap());
        assert_eq!(std::fs::read(preview.path()).unwrap(), b"agent two");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(preview.path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(preview.directory.path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            std::fs::remove_file(task.path().join("report")).unwrap();
            std::os::unix::fs::symlink(
                home.path().join("owner-secret"),
                task.path().join("report"),
            )
            .unwrap();
            std::fs::write(home.path().join("owner-secret"), "private").unwrap();
            assert!(preview.refresh().is_err());
            assert_eq!(std::fs::read(preview.path()).unwrap(), b"agent two");
        }
    }
    #[test]
    fn reviewed_preview_rejects_changed_artifacts_and_oversized_files() {
        let home = tempfile::tempdir().unwrap();
        let task = tempfile::tempdir().unwrap();
        std::fs::write(task.path().join("report"), b"accepted").unwrap();
        let evidence = OrganizationArtifactEvidence {
            path: "report".into(),
            sha256: format!("{:x}", Sha256::digest(b"accepted")),
            bytes: 8,
        };
        let mut preview =
            Preview::new(home.path(), task.path(), "report", Some(&evidence)).unwrap();
        std::fs::write(task.path().join("report"), b"unreviewed").unwrap();
        assert!(preview.refresh().is_err());
        assert_eq!(std::fs::read(preview.path()).unwrap(), b"accepted");
        let file = std::fs::File::create(task.path().join("large")).unwrap();
        file.set_len(8 * 1024 * 1024 + 1).unwrap();
        assert!(Preview::new(home.path(), task.path(), "large", None).is_err());
    }
}

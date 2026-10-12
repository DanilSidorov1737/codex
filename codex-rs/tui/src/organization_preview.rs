//! Owner-side snapshot for Neovim: never give the editor an agent-writable path.
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_app_server_protocol::OrganizationArtifactEvidence;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use std::time::SystemTime;

/// Observed metadata chooses candidates; Preview still verifies every selected file.
pub(crate) struct FollowFiles {
    pub(crate) root: PathBuf,
    pub(crate) expected: Vec<OrganizationArtifactEvidence>,
    seen: BTreeMap<String, Option<(SystemTime, u64)>>,
    checked: Instant,
}
impl FollowFiles {
    pub(crate) fn new(
        root: PathBuf,
        files: impl IntoIterator<Item = String>,
        expected: Vec<OrganizationArtifactEvidence>,
    ) -> Result<Self> {
        let mut seen = BTreeMap::new();
        for path in files {
            seen.insert(path.clone(), Self::fingerprint(&root, &path)?);
        }
        ensure!(seen.len() <= 1064, "follow exceeds declared-file ceiling");
        Ok(Self {
            root,
            expected,
            seen,
            checked: Instant::now(),
        })
    }
    fn fingerprint(root: &Path, path: &str) -> Result<Option<(SystemTime, u64)>> {
        let file = match codex_utils_path::open_relative_regular_file(root, Path::new(path)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let metadata = file.metadata()?;
        ensure!(
            metadata.len() <= 8 * 1024 * 1024,
            "follow file exceeds 8 MiB"
        );
        Ok(Some((metadata.modified()?, metadata.len())))
    }
    pub(crate) fn latest(&self) -> Option<&str> {
        self.seen
            .iter()
            .filter_map(|(path, metadata)| metadata.map(|m| (path, m)))
            .max_by_key(|(path, m)| (m.0, *path))
            .map(|(path, _)| path.as_str())
    }
    pub(crate) fn changed(&mut self) -> Result<Option<String>> {
        // ponytail: poll at most 1064 metadata entries once/second; use an OS watcher if measured overhead or missed timestamp-preserving writes warrant it.
        if self.checked.elapsed() < std::time::Duration::from_secs(1) {
            return Ok(None);
        }
        self.checked = Instant::now();
        let mut current = BTreeMap::new();
        for path in self.seen.keys() {
            current.insert(path.clone(), Self::fingerprint(&self.root, path)?);
        }
        let latest = current
            .iter()
            .filter(|(path, metadata)| self.seen.get(*path) != Some(*metadata))
            .filter_map(|(path, metadata)| metadata.map(|m| (path, m)))
            .max_by_key(|(path, m)| (m.0, *path))
            .map(|(path, _)| path.clone());
        self.seen = current;
        Ok(latest)
    }
}

pub(crate) struct Preview {
    directory: tempfile::TempDir,
    root: PathBuf,
    relative: String,
    expected: Option<OrganizationArtifactEvidence>,
    hash: String,
}
impl Preview {
    pub(crate) fn captured_text(home: &Path, text: &str) -> Result<Self> {
        ensure!(text.len() <= 1024 * 1024, "captured text exceeds 1 MiB");
        let directory = tempfile::Builder::new()
            .prefix(".organization-preview-")
            .tempdir_in(home)?;
        let mut file = tempfile::NamedTempFile::new_in(directory.path())?;
        file.write_all(text.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(directory.path().join("artifact"))
            .context("persist captured text")?;
        Ok(Self {
            root: directory.path().into(),
            directory,
            relative: "artifact".into(),
            expected: None,
            hash: format!("{:x}", Sha256::digest(text.as_bytes())),
        })
    }
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
    fn recent_follow_tracks_only_declared_files_and_rejects_symlink_replacements() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("first"), "first permitted file").unwrap();
        let mut follow = FollowFiles::new(
            root.path().into(),
            ["first".into(), "second".into()],
            vec![],
        )
        .unwrap();
        assert_eq!(follow.latest(), Some("first"));
        std::fs::write(
            root.path().join("unlisted"),
            "This more recent file has no task declaration",
        )
        .unwrap();
        follow.checked -= std::time::Duration::from_secs(2);
        assert_eq!(follow.changed().unwrap(), None);
        std::fs::write(root.path().join("second"), "new permitted output").unwrap();
        follow.checked -= std::time::Duration::from_secs(2);
        assert_eq!(follow.changed().unwrap().as_deref(), Some("second"));
        #[cfg(unix)]
        {
            std::fs::remove_file(root.path().join("second")).unwrap();
            std::os::unix::fs::symlink(root.path().join("unlisted"), root.path().join("second"))
                .unwrap();
            follow.checked -= std::time::Duration::from_secs(2);
            assert!(follow.changed().is_err());
        }
    }
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

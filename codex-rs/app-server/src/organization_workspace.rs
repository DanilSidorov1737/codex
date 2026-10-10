//! Small, explicit input snapshots and the existing native sandbox for checks.

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_core::config::Config;
use codex_core::exec::ExecCapturePolicy;
use codex_core::exec::ExecExpiration;
use codex_core::exec::ExecParams;
use codex_core::exec::process_exec_tool_call;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::SandboxPermissions;
use codex_protocol::organization_execution::OrganizationArtifactBase;
use codex_protocol::organization_execution::OrganizationArtifactEvidence;
use codex_protocol::organization_execution::OrganizationCheckEvidence;
use codex_protocol::organization_execution::OrganizationExecutionScope;
use codex_protocol::permissions::*;
use codex_utils_absolute_path::AbsolutePathBuf;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) struct TaskWorkspace {
    pub(crate) root: AbsolutePathBuf,
    pub(crate) profile: PermissionProfile,
    pub(crate) artifact_bases: Vec<OrganizationArtifactBase>,
}

impl TaskWorkspace {
    pub(crate) fn create(
        config: &Config,
        project: &Path,
        attempt_id: &str,
        scope: &OrganizationExecutionScope,
    ) -> Result<Self> {
        ensure!(
            cfg!(target_os = "linux"),
            "managed organization execution is qualified only on Linux"
        );
        Uuid::parse_str(attempt_id)?;
        let project = std::fs::canonicalize(project)?;
        ensure!(project.is_dir(), "project root is not a directory");
        let protected = [
            std::fs::canonicalize(&config.codex_home)?,
            std::fs::canonicalize(config.sqlite_config().home())?,
        ];
        let parent = std::env::temp_dir().join(format!("codex-organization-{}", current_uid()));
        private_directory(&parent, false)?;
        let root = parent.join(attempt_id);
        private_directory(&root, true)?;
        let root = AbsolutePathBuf::try_from(root)?;
        let mut total = 0u64;
        for relative in &scope.input_files {
            let path = regular_path(&project, relative)?;
            ensure!(
                !protected.iter().any(|root| path.starts_with(root)),
                "input would expose protected Codex or organization state"
            );
            let bytes = read_bounded(&path)?;
            total = total
                .checked_add(bytes.len() as u64)
                .context("input size overflow")?;
            ensure!(total <= MAX_INPUT_BYTES, "input snapshot exceeds 64 MiB");
            let target = root.join(relative);
            std::fs::create_dir_all(target.parent().context("input has no parent")?)?;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
            }
            let mut file = options.open(&target)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        let artifact_bases = scope
            .artifacts
            .iter()
            .map(|relative| {
                let path = project.join(relative);
                ensure!(
                    !protected.iter().any(|root| path.starts_with(root)),
                    "artifact destination is inside protected Codex or organization state"
                );
                let sha256 = match std::fs::symlink_metadata(&path) {
                    Ok(_) => Some(format!(
                        "{:x}",
                        Sha256::digest(read_bounded(&regular_path(&project, relative)?)?)
                    )),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error.into()),
                };
                Ok(OrganizationArtifactBase {
                    path: relative.clone(),
                    sha256,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let profile = workspace_profile(config, &root, &project, &protected, scope)?;
        Ok(Self {
            root,
            profile,
            artifact_bases,
        })
    }

    pub(crate) fn artifacts(
        &self,
        scope: &OrganizationExecutionScope,
    ) -> Result<Vec<OrganizationArtifactEvidence>> {
        scope
            .artifacts
            .iter()
            .map(|path| {
                let bytes = read_bounded(&regular_path(self.root.as_path(), path)?)?;
                Ok(OrganizationArtifactEvidence {
                    path: path.clone(),
                    sha256: format!("{:x}", Sha256::digest(&bytes)),
                    bytes: bytes.len() as u64,
                })
            })
            .collect()
    }

    pub(crate) async fn checks(
        &self,
        config: &Config,
        scope: &OrganizationExecutionScope,
        cancellation: CancellationToken,
    ) -> Result<Vec<OrganizationCheckEvidence>> {
        let mut checks = Vec::new();
        for argv in &scope.checks {
            ensure!(!cancellation.is_cancelled(), "check cancelled");
            let output = process_exec_tool_call(
                ExecParams {
                    command: argv.clone(),
                    cwd: self.root.clone(),
                    expiration: ExecExpiration::TimeoutOrCancellation {
                        timeout: Duration::from_secs(30),
                        cancellation: cancellation.clone(),
                    },
                    capture_policy: ExecCapturePolicy::ShellTool,
                    env: HashMap::from([
                        ("PATH".into(), "/usr/bin:/bin".into()),
                        ("HOME".into(), self.root.to_string_lossy().into_owned()),
                        ("TMPDIR".into(), self.root.to_string_lossy().into_owned()),
                    ]),
                    network: None,
                    network_environment_id: None,
                    sandbox_permissions: SandboxPermissions::UseDefault,
                    windows_sandbox_level: WindowsSandboxLevel::Disabled,
                    justification: None,
                    arg0: None,
                },
                &self.profile,
                &self.root,
                &[],
                &config.codex_linux_sandbox_exe,
                &config.codex_self_exe,
                false,
                None,
            )
            .await?;
            let mut text = output.aggregated_output.text;
            let truncated = text.len() > 65536;
            if truncated {
                let mut boundary = 65536;
                while !text.is_char_boundary(boundary) {
                    boundary -= 1;
                }
                text.truncate(boundary);
            }
            checks.push(OrganizationCheckEvidence {
                argv: argv.clone(),
                exit_code: (!output.timed_out).then_some(output.exit_code),
                output: text,
                truncated,
            });
        }
        Ok(checks)
    }
}

fn workspace_profile(
    config: &Config,
    root: &AbsolutePathBuf,
    project: &Path,
    protected: &[PathBuf],
    scope: &OrganizationExecutionScope,
) -> Result<PermissionProfile> {
    let mut entries = vec![
        FileSystemSandboxEntry::new(
            FileSystemPath::Special {
                value: FileSystemSpecialPath::Minimal,
            },
            FileSystemAccessMode::Read,
        ),
        FileSystemSandboxEntry::new(root.clone().into(), FileSystemAccessMode::Write),
    ];
    let helpers = [&config.codex_linux_sandbox_exe, &config.codex_self_exe]
        .into_iter()
        .flatten()
        .map(|path| std::fs::canonicalize(path).map_err(anyhow::Error::from))
        .collect::<Result<Vec<_>>>()?;
    // The daemon installs helpers in its private home. Minimal's allowlist
    // exposes only the exact executable files there, never their siblings.
    // Native deny masks cannot reopen a read-only child, so reject this overlap
    // if system defaults would expose the rest of that protected directory.
    for path in protected.iter().map(PathBuf::as_path).chain([project]) {
        if helpers.iter().any(|helper| helper.starts_with(path)) {
            ensure!(
                path != project,
                "install runtime helpers outside the original project"
            );
            ensure!(
                !codex_sandboxing::LINUX_PLATFORM_DEFAULT_READ_ROOTS
                    .iter()
                    .filter_map(|root| std::fs::canonicalize(root).ok())
                    .any(|root| path.starts_with(&root) || root.starts_with(path)),
                "runtime helpers inside protected system-readable state are unsupported"
            );
            continue;
        }
        entries.push(FileSystemSandboxEntry::new(
            AbsolutePathBuf::try_from(path)?.into(),
            FileSystemAccessMode::Deny,
        ));
    }
    for relative in &scope.immutable_files {
        entries.push(FileSystemSandboxEntry::new(
            root.join(relative).into(),
            FileSystemAccessMode::Read,
        ));
    }
    // Allow executable files required by the native runner, never their whole
    // installation/home directories. No credential/state sibling becomes readable.
    for path in helpers {
        let path = AbsolutePathBuf::try_from(path)?;
        entries.push(FileSystemSandboxEntry::new(
            path.into(),
            FileSystemAccessMode::Read,
        ));
    }
    Ok(PermissionProfile::from_runtime_permissions(
        &FileSystemSandboxPolicy::restricted(entries),
        NetworkSandboxPolicy::Restricted,
    ))
}

fn regular_path(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !relative.is_empty()
            && Path::new(relative)
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_))),
        "path must be relative without traversal"
    );
    let path = root.join(relative);
    ensure!(
        std::fs::canonicalize(&path)? == path,
        "symlink or alias in input/artifact path"
    );
    ensure!(
        std::fs::symlink_metadata(&path)?.is_file(),
        "input/artifact must be a regular file"
    );
    Ok(path)
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            file.metadata()?.nlink() == 1,
            "hard-linked input/artifact rejected"
        );
    }
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() <= MAX_FILE_BYTES,
        "file exceeds 8 MiB or is not regular"
    );
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_FILE_BYTES,
        "file grew beyond 8 MiB"
    );
    Ok(bytes)
}

fn private_directory(path: &Path, exclusive: bool) -> Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => {}
        Err(error) if !exclusive && error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "unsafe task directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            metadata.uid() == current_uid() && metadata.mode() & 0o777 == 0o700,
            "task directory must be current-user owned and mode 0700"
        );
    }
    File::open(path)?.sync_all()?;
    Ok(())
}

fn current_uid() -> u32 {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() }
    }
    #[cfg(not(unix))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_collection_rejects_traversal_symlinks_and_oversized_files() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let root = std::fs::canonicalize(fixture.path())?;
        std::fs::write(root.join("report.txt"), b"actual artifact")?;
        assert_eq!(
            read_bounded(&regular_path(&root, "report.txt")?)?,
            b"actual artifact"
        );
        assert!(regular_path(&root, "../report.txt").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("report.txt"), root.join("alias.txt"))?;
            assert!(regular_path(&root, "alias.txt").is_err());
            std::fs::hard_link(root.join("report.txt"), root.join("hard-link.txt"))?;
            assert!(read_bounded(&root.join("hard-link.txt")).is_err());
        }
        let file = File::create(root.join("oversized.txt"))?;
        file.set_len(MAX_FILE_BYTES + 1)?;
        assert!(read_bounded(&root.join("oversized.txt")).is_err());
        Ok(())
    }
}

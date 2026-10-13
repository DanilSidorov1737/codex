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
use codex_protocol::organization_execution::OrganizationTaskCheckpoint;
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

/// Fresh saved-owner-file evidence; never a backend result, grant or acceptance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerFileVerification {
    pub version: u32,
    pub root: PathBuf,
    pub scope: OrganizationExecutionScope,
    pub inputs: Vec<OrganizationArtifactEvidence>,
    pub artifacts: Vec<OrganizationArtifactEvidence>,
    pub checks: Vec<OrganizationCheckEvidence>,
    pub passed: bool,
    pub verification_digest: String,
}

/// Reuse the isolated native check runner, freezing every captured input.
pub async fn verify_owner_files(
    config: &Config,
    project: &Path,
    scope: OrganizationExecutionScope,
) -> Result<OwnerFileVerification> {
    ensure!(
        (1..=1024).contains(&scope.input_files.len())
            && (1..=32).contains(&scope.artifacts.len())
            && (1..=8).contains(&scope.checks.len()),
        "final check needs 1–1024 inputs, 1–32 outputs and 1–8 captured checks"
    );
    for argv in &scope.checks {
        ensure!(
            (1..=64).contains(&argv.len())
                && argv
                    .iter()
                    .all(|arg| !arg.is_empty() && arg.len() <= 4096 && !arg.contains('\0')),
            "invalid captured check argv"
        );
    }
    let mut snapshot_scope = scope.clone();
    for path in &scope.artifacts {
        if !snapshot_scope.input_files.contains(path) {
            snapshot_scope.input_files.push(path.clone());
        }
    }
    ensure!(
        snapshot_scope.input_files.len() <= 1024
            && scope
                .immutable_files
                .iter()
                .all(|path| scope.input_files.contains(path)),
        "invalid final check input/immutable scope"
    );
    for path in &snapshot_scope.input_files {
        ensure!(!path.is_empty() && path.len() <= 4096 && !path.contains('\0') && Path::new(path).components().all(|component| matches!(component, std::path::Component::Normal(name) if name != ".git")), "invalid final check path");
    }
    let unique: std::collections::HashSet<_> = snapshot_scope.input_files.iter().collect();
    ensure!(
        unique.len() == snapshot_scope.input_files.len(),
        "duplicate final check input"
    );
    let unique: std::collections::HashSet<_> = scope.artifacts.iter().collect();
    ensure!(
        unique.len() == scope.artifacts.len(),
        "duplicate final check output"
    );
    snapshot_scope.immutable_files = snapshot_scope.input_files.clone();
    // Final checks never Apply: outputs are already frozen inputs, so no base blobs are needed.
    snapshot_scope.artifacts.clear();
    let root = project.canonicalize()?;
    let capture = |base: &Path| -> Result<Vec<OrganizationArtifactEvidence>> {
        let mut total = 0;
        snapshot_scope
            .input_files
            .iter()
            .map(|path| {
                let bytes = read_relative(base, path)?;
                total += bytes.len();
                ensure!(
                    total <= MAX_INPUT_BYTES as usize,
                    "final check snapshot exceeds 64 MiB"
                );
                Ok(OrganizationArtifactEvidence {
                    path: path.clone(),
                    sha256: format!("{:x}", Sha256::digest(&bytes)),
                    bytes: bytes.len() as u64,
                })
            })
            .collect()
    };
    let inputs = capture(&root)?;
    let workspace = TaskWorkspace::create(
        config,
        &root,
        &Uuid::now_v7().to_string(),
        &snapshot_scope,
        &[],
        None,
    )?;
    let result: Result<OwnerFileVerification> = async {
        ensure!(
            capture(workspace.root.as_path())? == inputs,
            "owner files changed while capturing final check"
        );
        let checks = workspace
            .checks(config, &snapshot_scope, CancellationToken::new())
            .await?;
        let artifacts = workspace.artifacts(&scope)?;
        let passed = checks.iter().all(|check| check.exit_code == Some(0))
            && capture(workspace.root.as_path())? == inputs
            && capture(&root)? == inputs;
        let verification_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                &root, &scope, &inputs, &artifacts, &checks, passed
            ))?)
        );
        Ok(OwnerFileVerification {
            version: 1,
            root,
            scope,
            inputs,
            artifacts,
            checks,
            passed,
            verification_digest,
        })
    }
    .await;
    // This temporary check snapshot has no task, result, grant or recovery references.
    let cleanup = std::fs::remove_dir_all(workspace.root.as_path());
    let evidence = result?;
    cleanup.context("remove unreferenced final-check snapshot")?;
    Ok(evidence)
}

/// Frozen bytes are prepared before the durable intent; every base is checked
/// again immediately before its atomic replacement. No cross-file transaction
/// is claimed: interruptions get an uncertain receipt and are never replayed.
pub(crate) struct PreparedApplication {
    project: PathBuf,
    files: Vec<(String, Vec<u8>, Option<String>)>,
}
impl PreparedApplication {
    pub(crate) fn prepare_hunks(
        config: &Config,
        project: &Path,
        source: &Path,
        result: &codex_protocol::organization_execution::OrganizationTaskResult,
        paths: Option<&[String]>,
        hunks: Option<&[codex_protocol::organization_execution::OrganizationHunkSelection]>,
    ) -> Result<Self> {
        ensure!(
            cfg!(target_os = "linux"),
            "artifact application is qualified only on Linux"
        );
        ensure!(result.accepted, "unaccepted result cannot be applied");
        let paths = result
            .application_paths(paths)
            .map_err(anyhow::Error::msg)?;
        let selected_hunks = result
            .application_hunks(Some(&paths), hunks)
            .map_err(anyhow::Error::msg)?;
        ensure!(
            std::fs::canonicalize(project)? == project && std::fs::canonicalize(source)? == source,
            "aliased application root rejected"
        );
        private_directory(source, false)?;
        let protected = [
            std::fs::canonicalize(&config.codex_home)?,
            std::fs::canonicalize(config.sqlite_config().home())?,
        ];
        let mut files = Vec::new();
        let mut total = 0;
        ensure!(
            result.artifacts.len() == result.artifact_bases.len()
                && result
                    .artifacts
                    .iter()
                    .map(|a| &a.path)
                    .eq(result.artifact_bases.iter().map(|a| &a.path)),
            "artifact base evidence mismatch"
        );
        for (artifact, base) in result.artifacts.iter().zip(&result.artifact_bases) {
            if !paths.contains(&artifact.path) {
                continue;
            }
            ensure!(
                Path::new(&artifact.path)
                    .components()
                    .all(|p| matches!(p,std::path::Component::Normal(name) if name!=".git")),
                "protected or invalid artifact destination"
            );
            let destination = project.join(&artifact.path);
            ensure!(
                !protected
                    .iter()
                    .any(|root| destination.starts_with(root) || root.starts_with(&destination)),
                "protected state destination rejected"
            );
            let mut bytes = read_relative(source, &artifact.path)?;
            total += bytes.len() as u64;
            ensure!(total <= MAX_INPUT_BYTES, "application exceeds 64 MiB");
            ensure!(
                bytes.len() as u64 == artifact.bytes
                    && format!("{:x}", Sha256::digest(&bytes)) == artifact.sha256,
                "artifact changed after acceptance"
            );
            ensure!(
                current_base(project, &artifact.path)? == base.sha256,
                "original file changed since task snapshot"
            );
            if let Some(selection) = selected_hunks.iter().find(|s| s.path == artifact.path) {
                ensure!(
                    base.sha256.is_some(),
                    "hunk Apply requires an existing original base"
                );
                let original = read_relative(project, &artifact.path)?;
                let (patch, output) = codex_protocol::organization_execution::selected_hunks(
                    std::str::from_utf8(&original)?,
                    std::str::from_utf8(&bytes)?,
                    &selection.indices,
                )
                .map_err(anyhow::Error::msg)?;
                ensure!(
                    format!("{:x}", Sha256::digest(patch.as_bytes())) == selection.patch_sha256
                        && format!("{:x}", Sha256::digest(output.as_bytes()))
                            == selection.output_sha256,
                    "hunk preview changed; recapture exact selection"
                );
                bytes = output.into_bytes();
            }
            files.push((artifact.path.clone(), bytes, base.sha256.clone()));
        }
        Ok(Self {
            project: project.into(),
            files,
        })
    }
    pub(crate) fn apply(self) -> Result<()> {
        // Check every original again before the first write, then each immediately
        // before rename. The host freezes its editor inputs during this operation.
        for (path, _, base) in &self.files {
            ensure!(
                current_base(&self.project, path)? == *base,
                "original file changed before application"
            );
        }
        for (path, bytes, base) in self.files {
            write_relative_checked(&self.project, &path, &bytes, Some(base.as_deref()))?;
        }
        Ok(())
    }
}
fn current_base(root: &Path, path: &str) -> Result<Option<String>> {
    match read_relative(root, path) {
        Ok(bytes) => Ok(Some(format!("{:x}", Sha256::digest(bytes)))),
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(crate) struct TaskWorkspace {
    pub(crate) root: AbsolutePathBuf,
    pub(crate) profile: PermissionProfile,
    pub(crate) artifact_bases: Vec<OrganizationArtifactBase>,
}

impl TaskWorkspace {
    /// Validate declared local sources under the same native tool lock as writes.
    pub(crate) fn validate_research(
        &self,
        scope: &OrganizationExecutionScope,
        citations: &[codex_protocol::organization_coordination::OrganizationResearchCitation],
    ) -> Result<()> {
        ensure!(
            (1..=16).contains(&citations.len()),
            "research needs 1–16 citations"
        );
        let mut total = 0;
        for citation in citations {
            ensure!(
                scope.input_files.contains(&citation.path)
                    || scope.artifacts.contains(&citation.path),
                "research source outside declared local files"
            );
            ensure!(
                citation.start_line > 0
                    && citation.start_line <= citation.end_line
                    && citation.end_line - citation.start_line < 64,
                "citation range must contain 1–64 lines"
            );
            let bytes = read_relative(self.root.as_path(), &citation.path)?;
            ensure!(
                format!("{:x}", Sha256::digest(&bytes)) == citation.sha256,
                "research source hash changed"
            );
            let body =
                std::str::from_utf8(&bytes).context("research citations require UTF-8 text")?;
            let lines: Vec<_> = body
                .lines()
                .skip(citation.start_line as usize - 1)
                .take((citation.end_line - citation.start_line + 1) as usize)
                .collect();
            ensure!(
                lines.len() == (citation.end_line - citation.start_line + 1) as usize,
                "citation range exceeds local source"
            );
            total += lines.iter().map(|line| line.len() + 1).sum::<usize>();
            ensure!(total <= 16 * 1024, "cited local text exceeds 16 KiB");
        }
        Ok(())
    }

    /// Snapshot only granted inputs and existing outputs. The model never supplies
    /// file names or hashes; this runs under the host's native-tool lock.
    pub(crate) fn checkpoint_files(
        &self,
        scope: &OrganizationExecutionScope,
    ) -> Result<Vec<OrganizationArtifactEvidence>> {
        let mut files = Vec::new();
        let mut total = 0u64;
        for path in scope.input_files.iter().chain(&scope.artifacts) {
            if files
                .iter()
                .any(|file: &OrganizationArtifactEvidence| &file.path == path)
            {
                continue;
            }
            if !scope.input_files.contains(path)
                && let Err(error) = std::fs::symlink_metadata(self.root.join(path))
            {
                if error.kind() == std::io::ErrorKind::NotFound {
                    continue;
                }
                return Err(error.into());
            }
            let bytes = read_relative(self.root.as_path(), path)?;
            total = total
                .checked_add(bytes.len() as u64)
                .context("checkpoint size overflow")?;
            ensure!(
                total <= MAX_INPUT_BYTES,
                "checkpoint snapshot exceeds 64 MiB"
            );
            files.push(OrganizationArtifactEvidence {
                path: path.clone(),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                bytes: bytes.len() as u64,
            });
        }
        Ok(files)
    }

    pub(crate) fn import_checkpoint(
        &mut self,
        source: &Path,
        checkpoint: &OrganizationTaskCheckpoint,
        scope: &OrganizationExecutionScope,
    ) -> Result<()> {
        ensure!(
            checkpoint
                .artifact_bases
                .iter()
                .map(|base| &base.path)
                .eq(scope.artifacts.iter()),
            "checkpoint original bases differ from grant"
        );
        let mut total = 0;
        for file in &checkpoint.files {
            ensure!(
                scope.input_files.contains(&file.path) || scope.artifacts.contains(&file.path),
                "checkpoint file outside grant"
            );
            let bytes = verified_checkpoint_file(source, file, &mut total)?;
            write_relative(self.root.as_path(), &file.path, &bytes)?;
        }
        ensure!(
            scope
                .input_files
                .iter()
                .all(|path| checkpoint.files.iter().any(|file| &file.path == path)),
            "checkpoint inputs incomplete"
        );
        self.artifact_bases = checkpoint.artifact_bases.clone();
        Ok(())
    }

    /// Copy only declared inputs from the parent's private snapshot. The child
    /// keeps its own sandbox; no grant exposes another attempt's directory.
    pub(crate) fn inherit_inputs(
        &self,
        parent: &Path,
        scope: &OrganizationExecutionScope,
    ) -> Result<()> {
        let mut total = 0;
        for relative in &scope.input_files {
            let bytes = read_relative(parent, relative)?;
            total += bytes.len() as u64;
            ensure!(total <= MAX_INPUT_BYTES, "inherited inputs exceed 64 MiB");
            write_relative(self.root.as_path(), relative, &bytes)?;
        }
        Ok(())
    }

    /// Accepted child evidence is rechecked on the real file before import.
    /// Writes stay within the receiving attempt, never the original project.
    pub(crate) fn import_result(
        &self,
        source: &Path,
        result: &codex_protocol::organization_execution::OrganizationTaskResult,
    ) -> Result<()> {
        ensure!(result.accepted, "cannot import an unaccepted child result");
        let mut total = 0;
        for artifact in &result.artifacts {
            let bytes = read_relative(source, &artifact.path)?;
            total += bytes.len() as u64;
            ensure!(total <= MAX_INPUT_BYTES, "child artifacts exceed 64 MiB");
            ensure!(
                bytes.len() as u64 == artifact.bytes
                    && format!("{:x}", Sha256::digest(&bytes)) == artifact.sha256,
                "child artifact changed after acceptance"
            );
            write_relative(self.root.as_path(), &artifact.path, &bytes)?;
        }
        Ok(())
    }

    /// A later workflow step edits accepted bytes against the producer's original
    /// base, not a fresh hash that could hide an intervening owner edit.
    pub(crate) fn import_workflow_result(
        &mut self,
        source: &Path,
        result: &codex_protocol::organization_execution::OrganizationTaskResult,
    ) -> Result<()> {
        self.import_result(source, result)?;
        for base in &mut self.artifact_bases {
            if result
                .artifacts
                .iter()
                .any(|artifact| artifact.path == base.path)
            {
                *base = result
                    .artifact_bases
                    .iter()
                    .find(|original| original.path == base.path)
                    .context("workflow prerequisite original base missing")?
                    .clone();
            }
        }
        Ok(())
    }

    pub(crate) fn create(
        config: &Config,
        project: &Path,
        attempt_id: &str,
        scope: &OrganizationExecutionScope,
        deferred_inputs: &[OrganizationArtifactEvidence],
        checkpoint: Option<&OrganizationTaskCheckpoint>,
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
        let mut total = deferred_inputs.iter().try_fold(0u64, |total, artifact| {
            total
                .checked_add(artifact.bytes)
                .context("workflow input size overflow")
        })?;
        ensure!(total <= MAX_INPUT_BYTES, "workflow inputs exceed 64 MiB");
        for relative in &scope.input_files {
            if checkpoint.is_some() {
                continue;
            }
            if deferred_inputs
                .iter()
                .any(|artifact| &artifact.path == relative)
            {
                ensure!(
                    !scope.immutable_files.contains(relative),
                    "workflow cannot replace immutable input"
                );
                continue;
            }
            let path = regular_path(&project, relative)?;
            ensure!(
                !protected.iter().any(|root| path.starts_with(root)),
                "input would expose protected Codex or organization state"
            );
            let bytes = read_relative(&project, relative)?;
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
        let mut base_bytes = 0u64;
        let artifact_bases = scope
            .artifacts
            .iter()
            .map(|relative| {
                let path = project.join(relative);
                ensure!(
                    !protected.iter().any(|root| path.starts_with(root)),
                    "artifact destination is inside protected Codex or organization state"
                );
                let sha256 = if let Some(checkpoint) = checkpoint {
                    checkpoint
                        .artifact_bases
                        .iter()
                        .find(|base| &base.path == relative)
                        .context("checkpoint original base missing")?
                        .sha256
                        .clone()
                } else {
                    match std::fs::symlink_metadata(&path) {
                        Ok(_) => {
                            let bytes = read_relative(&project, relative)?;
                            base_bytes = base_bytes
                                .checked_add(bytes.len() as u64)
                                .context("base snapshot size overflow")?;
                            ensure!(
                                base_bytes <= MAX_INPUT_BYTES,
                                "artifact bases exceed 64 MiB"
                            );
                            Some(capture_artifact_base(&config.codex_home, &bytes)?)
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                        Err(error) => return Err(error.into()),
                    }
                };
                Ok(OrganizationArtifactBase {
                    captured: checkpoint.is_none_or(|checkpoint| {
                        checkpoint
                            .artifact_bases
                            .iter()
                            .any(|base| base.path == *relative && base.captured)
                    }),
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
                let bytes = read_relative(self.root.as_path(), path)?;
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
        let immutable_inputs: Vec<_> = scope
            .immutable_files
            .iter()
            .map(|path| {
                let bytes = read_relative(&self.root, path)?;
                Ok(OrganizationArtifactEvidence {
                    path: path.clone(),
                    sha256: format!("{:x}", Sha256::digest(&bytes)),
                    bytes: bytes.len() as u64,
                })
            })
            .collect::<Result<_>>()?;
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
                immutable_inputs: Some(immutable_inputs.clone()),
                argv: argv.clone(),
                exit_code: (!output.timed_out).then_some(output.exit_code),
                output: text,
                truncated,
            });
        }
        Ok(checks)
    }
}

/// Content-addressed host bytes live outside all granted agent workspaces.
fn capture_artifact_base(home: &Path, bytes: &[u8]) -> Result<String> {
    ensure!(
        bytes.len() as u64 <= MAX_FILE_BYTES,
        "artifact base exceeds 8 MiB"
    );
    let directory = home.join("organization-artifact-bases");
    private_directory(&directory, false)?;
    let hash = format!("{:x}", Sha256::digest(bytes));
    match read_relative(&directory, &hash) {
        Ok(existing) => ensure!(
            existing == bytes,
            "captured base changed; inspect host recovery"
        ),
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            write_relative_checked(&directory, &hash, bytes, Some(None))?
        }
        Err(error) => return Err(error),
    }
    Ok(hash)
}

fn verified_checkpoint_file(
    source: &Path,
    file: &OrganizationArtifactEvidence,
    total: &mut u64,
) -> Result<Vec<u8>> {
    let bytes = read_relative(source, &file.path)?;
    *total = total
        .checked_add(bytes.len() as u64)
        .context("checkpoint size overflow")?;
    ensure!(
        *total <= MAX_INPUT_BYTES
            && bytes.len() as u64 == file.bytes
            && format!("{:x}", Sha256::digest(&bytes)) == file.sha256,
        "checkpoint file changed; inspect before handoff"
    );
    Ok(bytes)
}

pub(crate) fn verify_checkpoint(
    source: &Path,
    checkpoint: &OrganizationTaskCheckpoint,
) -> Result<()> {
    let mut total = 0;
    for file in &checkpoint.files {
        verified_checkpoint_file(source, file, &mut total)?;
    }
    Ok(())
}

/// Recheck the exact text before saving immutable revision-pinned owner feedback.
pub(crate) fn validate_review_comment(
    source: &Path,
    artifact: &OrganizationArtifactEvidence,
    start_line: u32,
    end_line: u32,
) -> Result<()> {
    ensure!(source.canonicalize()? == source, "aliased comment source");
    private_directory(source, false)?;
    let bytes = read_relative(source, &artifact.path)?;
    ensure!(
        bytes.len() as u64 == artifact.bytes
            && format!("{:x}", Sha256::digest(&bytes)) == artifact.sha256,
        "review comment artifact changed"
    );
    let body = std::str::from_utf8(&bytes).context("review comments require UTF-8 text")?;
    ensure!(
        !body.contains('\0')
            && start_line > 0
            && start_line <= end_line
            && end_line - start_line < 64
            && end_line as usize <= body.lines().count(),
        "review comment range exceeds the verified text"
    );
    Ok(())
}

/// Reuse the host's bounded, no-follow checkpoint reader for an exact result repair.
pub(crate) fn repair_checkpoint_files(
    source: &Path,
    scope: &OrganizationExecutionScope,
    result: &codex_protocol::organization_execution::OrganizationTaskResult,
) -> Result<Vec<OrganizationArtifactEvidence>> {
    ensure!(source.canonicalize()? == source, "aliased repair workspace");
    private_directory(source, false)?;
    ensure!(
        !result.checks.is_empty(),
        "repair needs recorded host checks"
    );
    let workspace = TaskWorkspace {
        root: AbsolutePathBuf::try_from(source)?,
        profile: PermissionProfile::default(),
        artifact_bases: result.artifact_bases.clone(),
    };
    let files = workspace.checkpoint_files(scope)?;
    for artifact in &result.artifacts {
        ensure!(
            files.contains(artifact),
            "result bytes changed before repair"
        );
    }
    for check in &result.checks {
        let immutable = check
            .immutable_inputs
            .as_ref()
            .context("repair needs captured immutable check inputs")?;
        ensure!(
            immutable.iter().all(|file| files.contains(file)),
            "immutable check input changed before repair"
        );
    }
    Ok(files)
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

#[cfg(any(test, not(unix)))]
fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = options.open(path)?;
    read_bounded_file(file)
}

fn read_bounded_file(file: File) -> Result<Vec<u8>> {
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

// Directory descriptors prevent an agent swapping a parent for a symlink
// between validation and host copying. Reuse libc; no filesystem framework.
#[cfg(unix)]
fn relative_parent(root: &Path, relative: &str, create: bool) -> Result<(File, std::ffi::CString)> {
    use std::os::fd::AsRawFd;
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::OpenOptionsExt;
    let mut parts = Path::new(relative).components().peekable();
    ensure!(
        !relative.is_empty()
            && parts
                .clone()
                .all(|part| matches!(part, std::path::Component::Normal(_))),
        "invalid relative snapshot path"
    );
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)?;
    while let Some(part) = parts.next() {
        let name = std::ffi::CString::new(part.as_os_str().as_bytes())?;
        if parts.peek().is_none() {
            return Ok((directory, name));
        }
        if create {
            // SAFETY: directory owns a live fd; name is a terminated, single component.
            let result = unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) };
            if result < 0 {
                let error = std::io::Error::last_os_error();
                ensure!(
                    error.kind() == std::io::ErrorKind::AlreadyExists,
                    "cannot create snapshot parent: {error}"
                );
            }
        }
        // SAFETY: fd and name stay live through the call. NOFOLLOW rejects links.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: openat returned a new owned descriptor.
        directory = unsafe { File::from_raw_fd(fd) };
    }
    anyhow::bail!("missing snapshot file name")
}

#[cfg(unix)]
fn read_relative(root: &Path, relative: &str) -> Result<Vec<u8>> {
    read_bounded_file(codex_utils_path::open_relative_regular_file(
        root,
        Path::new(relative),
    )?)
}

#[cfg(unix)]
fn file_at(parent: &File, name: &std::ffi::CStr) -> Result<File> {
    use std::os::fd::AsRawFd;
    use std::os::fd::FromRawFd;
    // SAFETY: descriptors and CString are live; returned descriptor is owned.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn write_relative(root: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    write_relative_checked(root, relative, bytes, None)
}

#[cfg(unix)]
fn write_relative_checked(
    root: &Path,
    relative: &str,
    bytes: &[u8],
    expected: Option<Option<&str>>,
) -> Result<()> {
    use std::os::fd::AsRawFd;
    use std::os::fd::FromRawFd;
    let (parent, name) = relative_parent(root, relative, true)?;
    let base_at = || -> Result<Option<(String, std::fs::Permissions)>> {
        match file_at(&parent, &name) {
            Ok(file) => {
                let permissions = file.metadata()?.permissions();
                let bytes = read_bounded_file(file)?;
                Ok(Some((format!("{:x}", Sha256::digest(bytes)), permissions)))
            }
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    };
    let old = base_at()?;
    // Reading an existing destination rejects links and hard links before replacing
    // its name. renameat never follows a destination swapped after that check.
    match read_relative(root, relative) {
        Ok(_) => {}
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) => {}
        Err(error) => return Err(error),
    }
    let temporary = std::ffi::CString::new(format!(".snapshot-{}", Uuid::now_v7()))?;
    // SAFETY: live directory and terminated names, exclusive owned file creation.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: openat returned a new owned descriptor.
    let mut file = unsafe { File::from_raw_fd(fd) };
    let result = (|| -> Result<()> {
        file.write_all(bytes)?;
        if expected.is_some()
            && let Some((_, permissions)) = &old
        {
            use std::os::unix::fs::PermissionsExt;
            // Preserve ordinary executable/access bits without copying set-id bits.
            file.set_permissions(std::fs::Permissions::from_mode(permissions.mode() & 0o777))?;
        }
        file.sync_all()?;
        if let Some(expected) = expected {
            ensure!(
                base_at()?.as_ref().map(|(hash, _)| hash.as_str()) == expected,
                "original file changed before atomic rename"
            );
        }
        // SAFETY: both names are relative to the same live pinned directory.
        if unsafe {
            libc::renameat(
                parent.as_raw_fd(),
                temporary.as_ptr(),
                parent.as_raw_fd(),
                name.as_ptr(),
            )
        } < 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        parent.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        // SAFETY: removes only this exclusive staging name within the pinned directory.
        unsafe {
            libc::unlinkat(parent.as_raw_fd(), temporary.as_ptr(), 0);
        }
    }
    result
}

#[cfg(not(unix))]
fn write_relative_checked(
    _root: &Path,
    _relative: &str,
    _bytes: &[u8],
    _expected: Option<Option<&str>>,
) -> Result<()> {
    anyhow::bail!("safe application requires the qualified Linux filesystem backend")
}

#[cfg(not(unix))]
fn read_relative(root: &Path, relative: &str) -> Result<Vec<u8>> {
    read_bounded(&regular_path(root, relative)?)
}

#[cfg(not(unix))]
fn write_relative(_root: &Path, _relative: &str, _bytes: &[u8]) -> Result<()> {
    anyhow::bail!("delegated snapshots require the qualified Linux host")
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
    fn research_citations_check_real_declared_bytes_ranges_and_symlinks() -> Result<()> {
        use codex_protocol::organization_coordination::OrganizationResearchCitation;
        let home = tempfile::tempdir()?;
        let body = "first line\nObserved result\nassumption follows\n";
        std::fs::write(home.path().join("source.txt"), body)?;
        let workspace = TaskWorkspace {
            root: AbsolutePathBuf::try_from(home.path().to_path_buf())?,
            profile: PermissionProfile::default(),
            artifact_bases: vec![],
        };
        let mut scope = OrganizationExecutionScope {
            cost_budget: None,
            code_mode: false,
            input_files: vec!["source.txt".into()],
            immutable_files: vec![],
            artifacts: vec![],
            checks: vec![],
            model: "fixture".into(),
            max_tokens: 100,
            max_seconds: 30,
            max_model_requests: 1,
            max_attempts: 1,
            require_tool_approval: true,
            delegates: vec![],
            max_children: 0,
            auto_accept_children: false,
            not_before: None,
        };
        let citation = OrganizationResearchCitation {
            path: "source.txt".into(),
            sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
            start_line: 2,
            end_line: 3,
        };
        workspace.validate_research(&scope, std::slice::from_ref(&citation))?;
        for changed in [
            OrganizationResearchCitation {
                start_line: 0,
                ..citation.clone()
            },
            OrganizationResearchCitation {
                end_line: 4,
                ..citation.clone()
            },
            OrganizationResearchCitation {
                path: "../source.txt".into(),
                ..citation.clone()
            },
            OrganizationResearchCitation {
                sha256: "0".repeat(64),
                ..citation.clone()
            },
        ] {
            assert!(workspace.validate_research(&scope, &[changed]).is_err());
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                home.path().join("source.txt"),
                home.path().join("alias.txt"),
            )?;
            scope.input_files.push("alias.txt".into());
            assert!(
                workspace
                    .validate_research(
                        &scope,
                        &[OrganizationResearchCitation {
                            path: "alias.txt".into(),
                            ..citation.clone()
                        }]
                    )
                    .is_err()
            );
        }
        std::fs::write(home.path().join("source.txt"), "modified after capture")?;
        assert!(workspace.validate_research(&scope, &[citation]).is_err());
        Ok(())
    }
    #[test]
    fn captured_common_base_is_private_reused_verified_and_never_overwrites_corruption()
    -> Result<()> {
        let home = tempfile::tempdir()?;
        let bytes = b"original shared base\n";
        let hash = capture_artifact_base(home.path(), bytes)?;
        let path = home.path().join("organization-artifact-bases").join(&hash);
        assert_eq!(read_relative(path.parent().unwrap(), &hash)?, bytes);
        assert_eq!(capture_artifact_base(home.path(), bytes)?, hash);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path.parent().unwrap())?
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(&path)?.permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::write(&path, b"corrupt owner snapshot")?;
        assert!(capture_artifact_base(home.path(), bytes).is_err());
        assert_eq!(std::fs::read(&path)?, b"corrupt owner snapshot");
        assert!(capture_artifact_base(home.path(), &vec![0; MAX_FILE_BYTES as usize + 1]).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn exact_hunk_application_rejects_stale_hashes_and_preserves_unselected_changes()
    -> Result<()> {
        use codex_protocol::organization_execution::*;
        let home = tempfile::tempdir()?;
        let project = tempfile::tempdir()?;
        let source = tempfile::tempdir()?;
        let config = codex_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await?;
        let middle = (0..30)
            .map(|i| format!("context {i}\n"))
            .collect::<String>();
        let base = format!("old first\n{middle}old last\n");
        let artifact = format!("new first\n{middle}new last\n");
        write_relative(project.path(), "file.txt", base.as_bytes())?;
        write_relative(source.path(), "file.txt", artifact.as_bytes())?;
        let hash = |s: &str| format!("{:x}", Sha256::digest(s.as_bytes()));
        let result = OrganizationTaskResult {
            attempt_id: "attempt".into(),
            digest: "accepted".into(),
            accepted: true,
            checks: vec![],
            artifacts: vec![OrganizationArtifactEvidence {
                path: "file.txt".into(),
                bytes: artifact.len() as u64,
                sha256: hash(&artifact),
            }],
            artifact_bases: vec![OrganizationArtifactBase {
                captured: false,
                path: "file.txt".into(),
                sha256: Some(hash(&base)),
            }],
        };
        let (patch, output) = selected_hunks(&base, &artifact, &[0]).map_err(anyhow::Error::msg)?;
        let selection = OrganizationHunkSelection {
            path: "file.txt".into(),
            indices: vec![0],
            patch_sha256: hash(&patch),
            output_sha256: hash(&output),
        };
        let paths = vec!["file.txt".into()];
        let mut stale = selection.clone();
        stale.output_sha256 = "0".repeat(64);
        assert!(
            PreparedApplication::prepare_hunks(
                &config,
                project.path(),
                source.path(),
                &result,
                Some(&paths),
                Some(&[stale])
            )
            .is_err()
        );
        assert_eq!(read_relative(project.path(), "file.txt")?, base.as_bytes());
        let prepared = PreparedApplication::prepare_hunks(
            &config,
            project.path(),
            source.path(),
            &result,
            Some(&paths),
            Some(std::slice::from_ref(&selection)),
        )?;
        write_relative(project.path(), "file.txt", b"external owner change")?;
        assert!(prepared.apply().is_err());
        assert_eq!(
            read_relative(project.path(), "file.txt")?,
            b"external owner change"
        );
        write_relative(project.path(), "file.txt", base.as_bytes())?;
        PreparedApplication::prepare_hunks(
            &config,
            project.path(),
            source.path(),
            &result,
            Some(&paths),
            Some(&[selection]),
        )?
        .apply()?;
        assert_eq!(
            read_relative(project.path(), "file.txt")?,
            output.as_bytes()
        );
        assert!(output.ends_with("old last\n"));
        Ok(())
    }

    #[test]
    fn checkpoint_copies_only_granted_bytes_preserving_even_missing_output_bases() -> Result<()> {
        let source = tempfile::tempdir()?;
        let destination = tempfile::tempdir()?;
        let original = tempfile::tempdir()?;
        let scope: OrganizationExecutionScope = serde_json::from_value(serde_json::json!({
            "model":"fixture", "inputFiles":["input.txt"], "immutableFiles":["input.txt"],
            "artifacts":["report.txt","later.txt"], "checks":[], "maxAttempts":3,
            "maxSeconds":120,"maxTokens":1000,"maxModelRequests":4,
            "requireToolApproval":true,"delegates":[],"maxChildren":0
        }))?;
        write_relative(source.path(), "input.txt", b"pinned input")?;
        write_relative(source.path(), "report.txt", b"partial report")?;
        write_relative(source.path(), "private.txt", b"never transferred")?;
        write_relative(original.path(), "later.txt", b"owner changed absent output")?;
        let bases = vec![
            OrganizationArtifactBase {
                captured: false,
                path: "report.txt".into(),
                sha256: None,
            },
            OrganizationArtifactBase {
                captured: false,
                path: "later.txt".into(),
                sha256: Some(format!("{:x}", Sha256::digest(b"initial original"))),
            },
        ];
        let workspace = TaskWorkspace {
            root: AbsolutePathBuf::try_from(source.path())?,
            profile: PermissionProfile::default(),
            artifact_bases: bases.clone(),
        };
        let files = workspace.checkpoint_files(&scope)?;
        assert_eq!(
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["input.txt", "report.txt"]
        );
        let checkpoint = OrganizationTaskCheckpoint {
            id: "checkpoint".into(),
            task_id: "task".into(),
            project_id: "project".into(),
            owner_id: "owner".into(),
            attempt_id: "source".into(),
            authorization_id: "same-grant".into(),
            created_at: 1000,
            completed_action_ids: vec![],
            content: codex_protocol::organization_coordination::OrganizationCheckpointContent {
                completed_steps: vec!["Partial report".into()],
                remaining_steps: vec!["Finish report".into()],
                blocker: Some("Need owner decision".into()),
            },
            files,
            artifact_bases: bases.clone(),
        };
        let mut receiver = TaskWorkspace {
            root: AbsolutePathBuf::try_from(destination.path())?,
            profile: PermissionProfile::default(),
            artifact_bases: vec![],
        };
        verify_checkpoint(source.path(), &checkpoint)?;
        receiver.import_checkpoint(source.path(), &checkpoint, &scope)?;
        assert_eq!(receiver.artifact_bases, bases);
        assert_eq!(
            read_relative(destination.path(), "input.txt")?,
            b"pinned input"
        );
        assert_eq!(
            read_relative(destination.path(), "report.txt")?,
            b"partial report"
        );
        assert!(!destination.path().join("private.txt").exists());
        assert!(!destination.path().join("later.txt").exists());
        assert!(
            write_relative_checked(
                original.path(),
                "later.txt",
                b"lost owner edit",
                Some(checkpoint.artifact_bases[1].sha256.as_deref())
            )
            .is_err()
        );
        write_relative(source.path(), "report.txt", b"tampered after checkpoint")?;
        assert!(verify_checkpoint(source.path(), &checkpoint).is_err());
        assert!(
            receiver
                .import_checkpoint(source.path(), &checkpoint, &scope)
                .is_err()
        );
        assert_eq!(
            read_relative(destination.path(), "report.txt")?,
            b"partial report"
        );
        std::fs::remove_file(source.path().join("input.txt"))?;
        assert!(workspace.checkpoint_files(&scope).is_err());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn checkpoint_verification_rejects_file_aliases_and_traversal() -> Result<()> {
        let root = tempfile::tempdir()?;
        write_relative(root.path(), "file", b"original")?;
        let evidence = |path: &str| OrganizationArtifactEvidence {
            path: path.into(),
            bytes: 8,
            sha256: format!("{:x}", Sha256::digest(b"original")),
        };
        assert!(verified_checkpoint_file(root.path(), &evidence("../file"), &mut 0).is_err());
        std::os::unix::fs::symlink(root.path().join("file"), root.path().join("link"))?;
        assert!(verified_checkpoint_file(root.path(), &evidence("link"), &mut 0).is_err());
        std::fs::hard_link(root.path().join("file"), root.path().join("hard"))?;
        assert!(verified_checkpoint_file(root.path(), &evidence("file"), &mut 0).is_err());
        Ok(())
    }

    #[test]
    fn workflow_import_preserves_first_original_base_and_rechecks_accepted_bytes() -> Result<()> {
        let source = tempfile::tempdir()?;
        let destination = tempfile::tempdir()?;
        let original = tempfile::tempdir()?;
        write_relative(original.path(), "report.txt", b"owner edit between steps")?;
        write_relative(source.path(), "report.txt", b"accepted first step")?;
        let first_base = OrganizationArtifactBase {
            captured: false,
            path: "report.txt".into(),
            sha256: Some(format!("{:x}", Sha256::digest(b"initial original"))),
        };
        let result = codex_protocol::organization_execution::OrganizationTaskResult {
            attempt_id: "source".into(),
            digest: "recorded".into(),
            accepted: true,
            artifacts: vec![OrganizationArtifactEvidence {
                path: "report.txt".into(),
                bytes: 19,
                sha256: format!("{:x}", Sha256::digest(b"accepted first step")),
            }],
            artifact_bases: vec![first_base.clone()],
            checks: vec![],
        };
        let mut workspace = TaskWorkspace {
            root: AbsolutePathBuf::try_from(destination.path())?,
            profile: PermissionProfile::default(),
            artifact_bases: vec![OrganizationArtifactBase {
                captured: false,
                path: "report.txt".into(),
                sha256: Some(format!("{:x}", Sha256::digest(b"owner edit between steps"))),
            }],
        };
        workspace.import_workflow_result(source.path(), &result)?;
        assert_eq!(workspace.artifact_bases, vec![first_base.clone()]);
        assert_eq!(
            read_relative(destination.path(), "report.txt")?,
            b"accepted first step"
        );
        assert!(
            write_relative_checked(
                original.path(),
                "report.txt",
                b"second step",
                Some(first_base.sha256.as_deref())
            )
            .is_err()
        );
        assert_eq!(
            read_relative(original.path(), "report.txt")?,
            b"owner edit between steps"
        );
        write_relative(source.path(), "report.txt", b"tampered after acceptance")?;
        assert!(
            workspace
                .import_workflow_result(source.path(), &result)
                .is_err()
        );
        let mut unaccepted = result;
        unaccepted.accepted = false;
        assert!(
            workspace
                .import_workflow_result(source.path(), &unaccepted)
                .is_err()
        );
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn checked_replacement_preserves_changed_bases_and_rejects_aliases() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().canonicalize()?;
        write_relative(&root, "file", b"base")?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.join("file"), std::fs::Permissions::from_mode(0o755))?;
        let base = current_base(&root, "file")?;
        write_relative_checked(&root, "file", b"accepted", Some(base.as_deref()))?;
        assert_eq!(read_relative(&root, "file")?, b"accepted");
        assert_eq!(
            std::fs::metadata(root.join("file"))?.permissions().mode() & 0o777,
            0o755
        );
        assert!(
            write_relative_checked(&root, "file", b"lost edit", Some(base.as_deref())).is_err()
        );
        assert_eq!(read_relative(&root, "file")?, b"accepted");
        write_relative_checked(&root, "new", b"new file", Some(None))?;
        assert!(write_relative_checked(&root, "new", b"overwrite", Some(None)).is_err());
        std::os::unix::fs::symlink(root.join("file"), root.join("link"))?;
        assert!(write_relative_checked(&root, "link", b"lost edit", Some(None)).is_err());
        std::fs::hard_link(root.join("file"), root.join("hard"))?;
        assert!(write_relative_checked(&root, "file", b"lost edit", Some(None)).is_err());
        assert_eq!(std::fs::read(root.join("file"))?, b"accepted");
        assert!(!std::fs::read_dir(&root)?.any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".snapshot-")
        }));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn host_snapshot_copy_rejects_links_and_does_not_write_outside_root() -> Result<()> {
        let destination = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let root = std::fs::canonicalize(destination.path())?;
        std::fs::write(outside.path().join("secret"), "owner-only")?;
        std::os::unix::fs::symlink(outside.path(), root.join("escape"))?;
        assert!(read_relative(&root, "escape/secret").is_err());
        assert!(write_relative(&root, "escape/secret", b"changed").is_err());
        assert!(write_relative(&root, "../secret", b"changed").is_err());
        assert_eq!(
            std::fs::read_to_string(outside.path().join("secret"))?,
            "owner-only"
        );
        write_relative(&root, "nested/report.txt", b"first")?;
        write_relative(&root, "nested/report.txt", b"accepted child")?;
        assert_eq!(
            read_relative(&root, "nested/report.txt")?,
            b"accepted child"
        );
        std::fs::hard_link(root.join("nested/report.txt"), root.join("linked.txt"))?;
        assert!(write_relative(&root, "linked.txt", b"changed").is_err());
        assert!(read_relative(&root, "linked.txt").is_err());
        assert_eq!(
            std::fs::read_to_string(root.join("nested/report.txt"))?,
            "accepted child"
        );
        Ok(())
    }

    #[test]
    fn review_comment_refuses_nonexistent_lines_changed_bytes_and_symlink_sources() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let root = fixture.path().canonicalize()?;
        let bytes = b"first line\nsecond line\n";
        std::fs::write(root.join("review.txt"), bytes)?;
        let artifact = OrganizationArtifactEvidence {
            path: "review.txt".into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            bytes: bytes.len() as u64,
        };
        validate_review_comment(&root, &artifact, 1, 2)?;
        assert!(validate_review_comment(&root, &artifact, 0, 1).is_err());
        assert!(validate_review_comment(&root, &artifact, 1, 3).is_err());
        std::fs::write(root.join("review.txt"), b"changed\n")?;
        assert!(validate_review_comment(&root, &artifact, 1, 1).is_err());
        #[cfg(unix)]
        {
            std::fs::remove_file(root.join("review.txt"))?;
            std::fs::write(root.join("other.txt"), bytes)?;
            std::os::unix::fs::symlink("other.txt", root.join("review.txt"))?;
            assert!(validate_review_comment(&root, &artifact, 1, 2).is_err());
        }
        Ok(())
    }

    #[test]
    fn result_repair_snapshot_refuses_changed_artifacts_or_immutable_oracles() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let root = fixture.path().canonicalize()?;
        std::fs::write(root.join("code.py"), b"model candidate\n")?;
        std::fs::write(root.join("oracle.py"), b"immutable owner oracle\n")?;
        let scope: OrganizationExecutionScope = serde_json::from_value(
            serde_json::json!({"inputFiles":["code.py","oracle.py"],"immutableFiles":["oracle.py"],"artifacts":["code.py"],"checks":[["python3","oracle.py"]],"model":"fixture","maxTokens":100,"maxSeconds":30,"maxModelRequests":4,"maxAttempts":3,"requireToolApproval":true,"delegates":[],"maxChildren":0}),
        )?;
        let evidence = |path: &str, bytes: &[u8]| OrganizationArtifactEvidence {
            path: path.into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            bytes: bytes.len() as u64,
        };
        let result = codex_protocol::organization_execution::OrganizationTaskResult {
            attempt_id: Uuid::now_v7().to_string(),
            artifacts: vec![evidence("code.py", b"model candidate\n")],
            artifact_bases: vec![OrganizationArtifactBase {
                path: "code.py".into(),
                sha256: None,
                captured: true,
            }],
            checks: vec![OrganizationCheckEvidence {
                argv: scope.checks[0].clone(),
                immutable_inputs: Some(vec![evidence("oracle.py", b"immutable owner oracle\n")]),
                exit_code: Some(1),
                output: "regression failure".into(),
                truncated: false,
            }],
            digest: "a".repeat(64),
            accepted: false,
        };
        assert_eq!(repair_checkpoint_files(&root, &scope, &result)?.len(), 2);
        std::fs::write(root.join("code.py"), b"unreviewed changed candidate\n")?;
        assert!(repair_checkpoint_files(&root, &scope, &result).is_err());
        std::fs::write(root.join("code.py"), b"model candidate\n")?;
        std::fs::write(root.join("oracle.py"), b"changed owner oracle\n")?;
        assert!(repair_checkpoint_files(&root, &scope, &result).is_err());
        Ok(())
    }

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

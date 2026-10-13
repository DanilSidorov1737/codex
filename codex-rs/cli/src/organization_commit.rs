//! Explicit local owner Git flow. Captured plans are consumed once, including failures.
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_app_server::OwnerFileVerification;
use codex_protocol::organization_execution::OrganizationArtifactEvidence;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CommitPreview {
    version: u32,
    root: PathBuf,
    head: Option<String>,
    branch: Option<String>,
    verification: OwnerFileVerification,
    paths: Vec<OrganizationArtifactEvidence>,
    message: String,
    preview_digest: String,
}

pub(crate) fn write_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    let mut file = tempfile::NamedTempFile::new_in(&parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

pub(crate) fn read_private<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= 4 * 1024 * 1024,
        "invalid or oversized private owner receipt"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            metadata.nlink() == 1
                && metadata.mode() & 0o077 == 0
                && metadata.uid() == unsafe { libc::geteuid() },
            "owner receipt must be private, single-link and owner-controlled"
        );
    }
    let mut bytes = Vec::new();
    file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4 * 1024 * 1024,
        "owner receipt grew beyond its limit"
    );
    Ok(serde_json::from_slice(&bytes)?)
}

fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn secret_name(path: &str) -> bool {
    Path::new(path).components().any(|part| {
        let name = part.as_os_str().to_string_lossy().to_ascii_lowercase();
        name.starts_with(".env")
            || matches!(
                name.as_str(),
                ".ssh" | ".aws" | ".codex" | "auth.json" | "credentials" | "credentials.json"
            )
            || name.ends_with(".pem")
            || name.ends_with(".key")
    })
}

fn secret_signature(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    [
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
    ]
    .iter()
    .any(|prefix| text.contains(prefix))
        || text
            .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_')))
            .any(|word| {
                word.len() > 24
                    && (word.starts_with("sk-proj-")
                        || word.starts_with("sk-live-")
                        || word.starts_with("ghp_"))
            })
}

fn verify_file(
    root: &Path,
    evidence: &OrganizationArtifactEvidence,
    screen_secrets: bool,
) -> Result<()> {
    let file = codex_utils_path::open_relative_regular_file(root, Path::new(&evidence.path))?;
    let mut bytes = Vec::new();
    file.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024
            && bytes.len() as u64 == evidence.bytes
            && format!("{:x}", Sha256::digest(&bytes)) == evidence.sha256,
        "saved owner file changed after final verification"
    );
    if screen_secrets {
        ensure!(
            !secret_name(&evidence.path) && !secret_signature(&bytes),
            "reviewed output matches a common secret filename/token signature; exclude it and review manually"
        );
    }
    Ok(())
}

async fn validate(verification: &OwnerFileVerification) -> Result<serde_json::Value> {
    ensure!(
        cfg!(target_os = "linux")
            && verification.version == 1
            && verification.passed
            && !verification.checks.is_empty()
            && verification.checks.iter().all(|c| c.exit_code == Some(0))
            && verification
                .checks
                .iter()
                .map(|c| &c.argv)
                .eq(verification.scope.checks.iter()),
        "fresh passing owner verification with every captured check is required"
    );
    ensure!(
        digest(&(
            &verification.root,
            &verification.scope,
            &verification.inputs,
            &verification.artifacts,
            &verification.checks,
            verification.passed
        ))? == verification.verification_digest,
        "owner verification receipt changed"
    );
    ensure!(
        verification
            .artifacts
            .iter()
            .map(|a| &a.path)
            .eq(verification.scope.artifacts.iter()),
        "verification output scope changed"
    );
    let expected_inputs: std::collections::BTreeSet<_> = verification
        .scope
        .input_files
        .iter()
        .chain(&verification.scope.artifacts)
        .collect();
    let captured_inputs: std::collections::BTreeSet<_> =
        verification.inputs.iter().map(|file| &file.path).collect();
    ensure!(
        expected_inputs == captured_inputs
            && captured_inputs.len() == verification.inputs.len()
            && verification
                .checks
                .iter()
                .all(|check| check.immutable_inputs.as_ref() == Some(&verification.inputs)),
        "final verification input/check pins are incomplete"
    );
    let repo = crate::organization_repo::preview(&verification.root, &verification.scope).await?;
    ensure!(
        repo["repositoryRoot"] == serde_json::to_value(&verification.root)?,
        "local commit requires verification at the Git working-tree root"
    );
    for input in &verification.inputs {
        verify_file(&verification.root, input, false)?;
    }
    for artifact in &verification.artifacts {
        verify_file(&verification.root, artifact, true)?;
    }
    let index = crate::organization_repo::git(
        &verification.root,
        &[
            "diff",
            "--cached",
            "--quiet",
            "--no-ext-diff",
            "--no-textconv",
        ],
    )
    .await?;
    ensure!(
        index.status.success(),
        "Git index is nonempty; preserve unrelated staged changes and review them separately"
    );
    let signing =
        crate::organization_repo::git(&verification.root, &["config", "--bool", "commit.gpgsign"])
            .await?;
    ensure!(
        signing.stdout != b"true\n",
        "signed commits require the native owner terminal; signing configuration is preserved"
    );
    let mut attributes = vec!["check-attr", "-z", "filter", "--"];
    attributes.extend(verification.artifacts.iter().map(|a| a.path.as_str()));
    let attributes = crate::organization_repo::git(&verification.root, &attributes).await?;
    ensure!(
        attributes.status.success(),
        "Git attribute preflight failed"
    );
    let fields: Vec<_> = attributes
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect();
    ensure!(
        fields.len() == verification.artifacts.len() * 3
            && fields
                .chunks(3)
                .all(|field| matches!(field[2], b"unspecified" | b"unset")),
        "configured Git clean filters require the native owner terminal"
    );
    Ok(repo)
}

pub(crate) async fn preview(
    verification: OwnerFileVerification,
    message: String,
) -> Result<CommitPreview> {
    ensure!(
        !message.trim().is_empty()
            && message.len() <= 4096
            && !message.contains('\0')
            && !secret_signature(message.as_bytes()),
        "invalid local commit message"
    );
    let repo = validate(&verification).await?;
    let head = repo["head"].as_str().map(str::to_owned);
    let branch = repo["branch"].as_str().map(str::to_owned);
    let root = verification.root.clone();
    let mut paths = Vec::new();
    for artifact in &verification.artifacts {
        let changed = if let Some(head) = &head {
            let spec = format!("{head}:{}", artifact.path);
            let blob = crate::organization_repo::git(&root, &["show", &spec]).await?;
            !blob.status.success()
                || format!("{:x}", Sha256::digest(&blob.stdout)) != artifact.sha256
        } else {
            true
        };
        if changed {
            paths.push(artifact.clone());
        }
    }
    ensure!(
        !paths.is_empty(),
        "no declared checked outputs differ from HEAD"
    );
    let preview_digest = digest(&(1, &root, &head, &branch, &verification, &paths, &message))?;
    Ok(CommitPreview {
        version: 1,
        root,
        head,
        branch,
        verification,
        paths,
        message,
        preview_digest,
    })
}

pub(crate) async fn commit(plan: CommitPreview, home: &Path) -> Result<serde_json::Value> {
    ensure!(
        plan.version == 1
            && digest(&(
                plan.version,
                &plan.root,
                &plan.head,
                &plan.branch,
                &plan.verification,
                &plan.paths,
                &plan.message
            ))? == plan.preview_digest,
        "reviewed commit plan changed"
    );
    let current = preview(plan.verification.clone(), plan.message.clone()).await?;
    ensure!(
        current.preview_digest == plan.preview_digest,
        "repository HEAD/branch or reviewed files changed"
    );
    let home = home.canonicalize()?;
    let directory = home.join("organization-git-receipts");
    match std::fs::create_dir(&directory) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    ensure!(
        directory.canonicalize()? == directory,
        "aliased Git receipt directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let m = directory.metadata()?;
        ensure!(
            m.is_dir() && m.mode() & 0o077 == 0 && m.uid() == unsafe { libc::geteuid() },
            "Git receipts must remain private and owner-controlled"
        );
    }
    let receipt = directory.join(format!("{}.json", plan.preview_digest));
    write_new(
        &receipt,
        &serde_json::json!({"state":"intent","plan":plan,"warning":"local staging/commit only; hooks disabled; on interruption inspect HEAD/index and this receipt, never replay"}),
    )?;
    let outcome = async {
        let mut args = vec!["-c","core.hooksPath=/dev/null","add","--"];
        args.extend(plan.paths.iter().map(|a| a.path.as_str()));
        ensure!(crate::organization_repo::git(&plan.root,&args).await?.status.success(), "reviewed Git staging failed; inspect the index");
        let staged = crate::organization_repo::git(&plan.root,&["diff","--cached","--name-only","-z","--no-renames","--no-ext-diff","--no-textconv"]).await?;
        ensure!(staged.status.success(), "staged path inspection failed");
        let actual: std::collections::BTreeSet<_> = staged.stdout.split(|b| *b==0).filter(|p| !p.is_empty()).map(std::str::from_utf8).collect::<std::result::Result<_,_>>()?;
        let expected: std::collections::BTreeSet<_> = plan.paths.iter().map(|a| a.path.as_str()).collect();
        ensure!(actual == expected, "staged paths differ from the reviewed output set; inspect before proceeding");
        for artifact in &plan.paths {
            let spec = format!(":{}",artifact.path);
            let blob = crate::organization_repo::git(&plan.root,&["show",&spec]).await?;
            ensure!(blob.status.success() && blob.stdout.len() as u64 == artifact.bytes && format!("{:x}",Sha256::digest(&blob.stdout)) == artifact.sha256 && !secret_signature(&blob.stdout), "staged bytes differ from the final checked bytes or match a secret signature");
        }
        let before = crate::organization_repo::preview(&plan.root,&plan.verification.scope).await?;
        ensure!(before["head"] == serde_json::to_value(&plan.head)? && before["branch"] == serde_json::to_value(&plan.branch)?, "repository identity changed during staging");
        // Recheck test inputs as well as staged outputs immediately before committing.
        for input in &plan.verification.inputs {
            verify_file(&plan.root, input, false)?;
        }
        let output = crate::organization_repo::git(&plan.root,&["-c","core.hooksPath=/dev/null","commit","-m",&plan.message]).await?;
        ensure!(output.status.success(), "local Git commit did not finish; inspect HEAD/index and receipt before any manual recovery");
        let head = crate::organization_repo::git(&plan.root,&["rev-parse","--verify","HEAD"]).await?;
        ensure!(head.status.success(), "committed HEAD inspection failed");
        let head = String::from_utf8(head.stdout)?.trim().to_owned();
        let parents = crate::organization_repo::git(&plan.root,&["rev-list","--parents","-n1",&head]).await?;
        let parent_text = std::str::from_utf8(&parents.stdout)?;
        let parent_fields: Vec<_> = parent_text.split_whitespace().collect();
        ensure!(parents.status.success() && parent_fields.first().copied() == Some(head.as_str()) && parent_fields.get(1).copied() == plan.head.as_deref() && parent_fields.len() == if plan.head.is_some() {2} else {1}, "committed parent differs from the reviewed base; inspect receipt");
        let changed = crate::organization_repo::git(&plan.root,&["diff-tree","--root","--no-commit-id","--name-only","-r","-z",&head]).await?;
        let actual: std::collections::BTreeSet<_> = changed.stdout.split(|b| *b==0).filter(|p| !p.is_empty()).map(std::str::from_utf8).collect::<std::result::Result<_,_>>()?;
        ensure!(changed.status.success() && actual == expected, "committed paths differ from the reviewed set; inspect receipt");
        for artifact in &plan.paths {
            let spec=format!("{head}:{}",artifact.path);
            let blob=crate::organization_repo::git(&plan.root,&["show",&spec]).await?;
            ensure!(blob.status.success() && format!("{:x}",Sha256::digest(&blob.stdout)) == artifact.sha256, "committed bytes differ from final verification; inspect receipt");
        }
        Ok::<_,anyhow::Error>(head)
    }.await;
    let value = match &outcome {
        Ok(head) => {
            serde_json::json!({"state":"completed","commit":head,"plan":plan,"receipt":receipt,"pushed":false})
        }
        Err(error) => {
            serde_json::json!({"state":"needsInspection","error":error.to_string(),"plan":plan,"receipt":receipt,"replayAllowed":false})
        }
    };
    let mut file = tempfile::NamedTempFile::new_in(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(&serde_json::to_vec_pretty(&value)?)?;
    file.as_file().sync_all()?;
    file.persist(&receipt)?;
    std::fs::File::open(&directory)?.sync_all()?;
    outcome.context("reviewed local Git operation needs inspection")?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::organization_execution::OrganizationCheckEvidence;
    #[tokio::test]
    async fn reviewed_commit_preserves_unrelated_work_refuses_stale_index_and_consumes_plan_once()
    -> Result<()> {
        let repo = tempfile::tempdir()?;
        let home = tempfile::tempdir()?;
        let root = repo.path().canonicalize()?;
        let run = |args: &[&str]| -> Result<()> {
            ensure!(
                std::process::Command::new("git")
                    .current_dir(&root)
                    .args(args)
                    .status()?
                    .success(),
                "fixture Git failed"
            );
            Ok(())
        };
        run(&["init", "--quiet"])?;
        run(&["config", "user.name", "Qualification"])?;
        run(&["config", "user.email", "qualification@example.invalid"])?;
        run(&["config", "commit.gpgsign", "false"])?;
        std::fs::write(root.join("code.py"), b"baseline\n")?;
        std::fs::write(root.join("oracle.py"), b"fixture-only oracle\n")?;
        std::fs::write(root.join("unrelated.txt"), b"original unrelated\n")?;
        run(&["add", "--", "code.py", "oracle.py", "unrelated.txt"])?;
        run(&["commit", "--quiet", "-m", "fixture baseline"])?;
        std::fs::write(root.join("code.py"), b"reviewed final bytes\n")?;
        std::fs::write(root.join("unrelated.txt"), b"owner unrelated dirty bytes\n")?;
        let scope = serde_json::from_value(
            serde_json::json!({"inputFiles":["code.py","oracle.py"],"immutableFiles":["oracle.py"],"artifacts":["code.py"],"checks":[["python3","oracle.py"]],"model":"fixture","maxTokens":100,"maxSeconds":30,"maxModelRequests":1,"maxAttempts":1,"requireToolApproval":true,"delegates":[],"maxChildren":0}),
        )?;
        let evidence = |path: &str| -> Result<OrganizationArtifactEvidence> {
            let bytes = std::fs::read(root.join(path))?;
            Ok(OrganizationArtifactEvidence {
                path: path.into(),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                bytes: bytes.len() as u64,
            })
        };
        let inputs = vec![evidence("code.py")?, evidence("oracle.py")?];
        let checks = vec![OrganizationCheckEvidence {
            immutable_inputs: Some(inputs.clone()),
            argv: vec!["python3".into(), "oracle.py".into()],
            exit_code: Some(0),
            output: "Synthetic unit receipt; actual native final-check qualified separately".into(),
            truncated: false,
        }];
        let mut verification = OwnerFileVerification {
            version: 1,
            root: root.clone(),
            scope,
            inputs: inputs.clone(),
            artifacts: vec![inputs[0].clone()],
            checks,
            passed: true,
            verification_digest: String::new(),
        };
        verification.verification_digest = digest(&(
            &verification.root,
            &verification.scope,
            &verification.inputs,
            &verification.artifacts,
            &verification.checks,
            verification.passed,
        ))?;
        let index = std::fs::read(root.join(".git/index"))?;
        let plan = preview(verification.clone(), "fix: reviewed fixture output".into()).await?;
        assert_eq!(std::fs::read(root.join(".git/index"))?, index);
        std::fs::write(root.join("code.py"), b"unreviewed later edit\n")?;
        assert!(preview(verification.clone(), "stale".into()).await.is_err());
        std::fs::write(root.join("code.py"), b"reviewed final bytes\n")?;
        run(&["add", "--", "unrelated.txt"])?;
        assert!(
            preview(verification.clone(), "unrelated staged".into())
                .await
                .is_err()
        );
        run(&["reset", "--quiet", "--", "unrelated.txt"])?;
        let record = commit(plan.clone(), home.path()).await?;
        assert_eq!(record["state"], "completed");
        assert_eq!(record["pushed"], false);
        assert_eq!(
            std::fs::read(root.join("unrelated.txt"))?,
            b"owner unrelated dirty bytes\n"
        );
        assert!(
            commit(plan, home.path()).await.is_err(),
            "captured plan cannot replay"
        );
        let output = repo.path().join("receipt.json");
        write_new(&output, &verification)?;
        let _: OwnerFileVerification = read_private(&output)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o644))?;
            assert!(read_private::<OwnerFileVerification>(&output).is_err());
        }
        Ok(())
    }
}

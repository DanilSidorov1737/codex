//! Owner-only, read-only Git and declared-file preview. No task or grant is created.
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization_execution::OrganizationExecutionScope;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::Component;
use std::path::Path;
use std::process::Output;
use std::time::Duration;

async fn git(root: &Path, args: &[&str]) -> Result<Output> {
    let mut command = tokio::process::Command::new("git");
    command
        .current_dir(root)
        .args([
            "-c",
            codex_git_utils::SAFE_BARE_REPOSITORY_CONFIG,
            "-c",
            "core.fsmonitor=false",
            "--literal-pathspecs",
        ])
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .kill_on_drop(true);
    for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        command.env_remove(name);
    }
    tokio::time::timeout(Duration::from_secs(5), command.output())
        .await
        .context("repository preview Git command timed out")?
        .context("run native Git for repository preview")
}

pub(crate) async fn preview(
    root: &Path,
    scope: &OrganizationExecutionScope,
) -> Result<serde_json::Value> {
    ensure!(
        scope.input_files.len() <= 1024 && scope.artifacts.len() <= 32,
        "scope file limit exceeded"
    );
    for name in scope
        .input_files
        .iter()
        .chain(&scope.immutable_files)
        .chain(&scope.artifacts)
    {
        ensure!(
            !name.is_empty()
                && name.len() <= 4096
                && !name.contains('\0')
                && Path::new(name)
                    .components()
                    .all(|c| matches!(c, Component::Normal(p) if p != ".git")),
            "invalid requested repository path"
        );
    }
    ensure!(
        scope
            .immutable_files
            .iter()
            .all(|p| scope.input_files.contains(p)),
        "immutable files must be inputs"
    );
    let root = root
        .canonicalize()
        .context("resolve owner repository root")?;
    let repo = git(&root, &["rev-parse", "--show-toplevel"]).await?;
    ensure!(
        repo.status.success(),
        "owner path is not a working Git repository"
    );
    let repository_path = std::str::from_utf8(&repo.stdout)?;
    let repo = Path::new(
        repository_path
            .strip_suffix('\n')
            .context("repository root output lacks its line terminator")?,
    )
    .canonicalize()?;
    let head = git(&root, &["rev-parse", "--verify", "HEAD"]).await?;
    let branch = git(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"]).await?;
    let status = git(
        &root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--no-renames",
            "--untracked-files=normal",
        ],
    )
    .await?;
    ensure!(
        status.status.success() && status.stdout.len() <= 1024 * 1024,
        "repository status unavailable or exceeds 1 MiB"
    );
    let mut changes = Vec::new();
    for entry in status
        .stdout
        .split(|b| *b == 0)
        .filter(|entry| !entry.is_empty())
    {
        ensure!(
            entry.len() >= 4 && entry[2] == b' ' && changes.len() < 2000,
            "invalid or oversized repository status"
        );
        changes.push(json!({"status":std::str::from_utf8(&entry[..2])?,"path":std::str::from_utf8(&entry[3..])?}));
    }
    let mut bases = Vec::new();
    for name in &scope.artifacts {
        let hash = match codex_utils_path::open_relative_regular_file(&root, Path::new(name)) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
                ensure!(
                    bytes.len() <= 8 * 1024 * 1024,
                    "requested output base exceeds 8 MiB"
                );
                Some(format!("{:x}", Sha256::digest(&bytes)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        bases.push(json!({"path":name,"sha256":hash}));
    }
    let components: BTreeSet<_> = scope
        .artifacts
        .iter()
        .filter_map(|name| Path::new(name).parent())
        .map(|path| {
            if path.as_os_str().is_empty() {
                Path::new(".")
            } else {
                path
            }
        })
        .collect();
    let mut advice = BTreeSet::new();
    for name in &scope.artifacts {
        advice.insert(match Path::new(name).extension().and_then(|s| s.to_str()) {
            Some("rs") => "Rust outputs: run affected crate unit/integration checks and workspace formatting; public types also need generated schema/SDK contract checks.",
            Some("py") => "Python outputs: retain real-file and package/import checks; CLI/package changes also need native smoke checks.",
            Some("json" | "toml" | "yaml" | "yml") => "Configuration/contract outputs: check parsing, compatibility and affected consumers.",
            _ => "Owner-defined outputs: verify their intended consumers and actual display/content behavior.",
        });
    }
    Ok(
        json!({"root":root,"repositoryRoot":repo,"head":head.status.success().then(|| String::from_utf8_lossy(&head.stdout).trim().to_owned()),"branch":branch.status.success().then(|| String::from_utf8_lossy(&branch.stdout).trim().to_owned()),"changes":changes,"requestedInputs":scope.input_files,"requestedImmutableFiles":scope.immutable_files,"declaredOutputs":scope.artifacts,"outputBases":bases,"checks":scope.checks,"checkAdvice":{"declaredComponents":components,"advice":advice,"capturedChecksPreserved":true,"limits":"File-type advice only, not dependency analysis or test discovery. Every owner-captured check remains required; add applicable cross-component checks instead of dropping them."},"authorizationCreated":false,"scopeValidation":"requested file preview only; native authorization separately revalidates identity, paths, checks, permissions and budgets"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn preview_preserves_dirty_files_and_index_and_never_reads_unrelated_contents()
    -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .current_dir(root)
                .args(args)
                .status()?;
            ensure!(status.success(), "fixture Git failed");
            Ok::<_, anyhow::Error>(())
        };
        run(&["init", "--quiet"])?;
        std::fs::write(root.join("code.py"), "old\n")?;
        run(&["add", "--", "code.py"])?;
        run(&[
            "-c",
            "user.name=Qualification",
            "-c",
            "user.email=qualification@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture baseline",
        ])?;
        std::fs::write(root.join("code.py"), "owner dirty bytes\n")?;
        std::fs::write(
            root.join("private.env"),
            "UNRELATED_FIXTURE_CONTENT_NEVER_READ\n",
        )?;
        let scope: OrganizationExecutionScope = serde_json::from_value(
            json!({"inputFiles":["code.py"],"immutableFiles":[],"artifacts":["code.py","new.py"],"checks":[["python3","oracle.py"]],"model":"fixture","maxTokens":100,"maxSeconds":30,"maxModelRequests":1,"maxAttempts":1,"requireToolApproval":true,"delegates":[],"maxChildren":0}),
        )?;
        let before = git(root, &["status", "--porcelain=v1", "-z"]).await?.stdout;
        let index = std::fs::read(root.join(".git/index"))?;
        let result = preview(root, &scope).await?;
        assert!(result["head"].is_string());
        assert_eq!(result["authorizationCreated"], false);
        assert_eq!(result["checks"], serde_json::to_value(&scope.checks)?);
        assert_eq!(result["checkAdvice"]["capturedChecksPreserved"], true);
        assert!(
            result["checkAdvice"]["advice"]
                .to_string()
                .contains("package/import")
        );
        assert_eq!(
            result["outputBases"][0]["sha256"],
            format!("{:x}", Sha256::digest(b"owner dirty bytes\n"))
        );
        assert!(result["outputBases"][1]["sha256"].is_null());
        assert!(
            !result
                .to_string()
                .contains("UNRELATED_FIXTURE_CONTENT_NEVER_READ")
        );
        assert_eq!(
            git(root, &["status", "--porcelain=v1", "-z"]).await?.stdout,
            before
        );
        assert_eq!(std::fs::read(root.join(".git/index"))?, index);
        assert_eq!(std::fs::read(root.join("code.py"))?, b"owner dirty bytes\n");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("private.env", root.join("linked.py"))?;
            let mut linked = scope.clone();
            linked.artifacts = vec!["linked.py".into()];
            assert!(preview(root, &linked).await.is_err());
        }
        let mut invalid = scope;
        invalid.artifacts.push("../outside".into());
        assert!(preview(root, &invalid).await.is_err());
        Ok(())
    }
}

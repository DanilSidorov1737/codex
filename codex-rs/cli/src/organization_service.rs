//! Render a reviewable user unit; installation/activation belongs to the owner.
use anyhow::Result;
use anyhow::ensure;
use std::path::Path;

fn quote(path: &Path) -> Result<String> {
    ensure!(path.is_absolute(), "service paths must be absolute");
    let text = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("service path is not UTF-8"))?;
    ensure!(
        !text.chars().any(char::is_control),
        "service path contains control characters"
    );
    Ok(format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    ))
}

pub(crate) fn unit(
    cli: &Path,
    home: &Path,
    environment: Option<&Path>,
    stall_seconds: Option<u32>,
) -> Result<String> {
    ensure!(
        cfg!(target_os = "linux"),
        "user service units require Linux"
    );
    ensure!(
        stall_seconds.is_none_or(|seconds| (10..=3600).contains(&seconds)),
        "stall threshold must be 10..3600"
    );
    let cli = quote(cli)?;
    let home = quote(home)?;
    let value = &home[1..home.len() - 1];
    let mut settings =
        format!("Environment=\"CODEX_HOME={value}\"\nEnvironment=\"CODEX_SQLITE_HOME={value}\"\n");
    if let Some(path) = environment {
        let metadata = std::fs::symlink_metadata(path)?;
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "environment file must be regular"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(
                metadata.uid() == unsafe { libc::geteuid() }
                    && metadata.mode() & 0o777 == 0o600
                    && metadata.nlink() == 1,
                "environment file must be current-user owned, 0600 and not hard-linked"
            );
        }
        settings.push_str(&format!("EnvironmentFile={}\n", quote(path)?));
    }
    if let Some(seconds) = stall_seconds {
        settings.push_str(&format!(
            "Environment=CODEX_ORGANIZATION_STALL_SECONDS={seconds}\n"
        ));
    }
    Ok(format!(
        "[Unit]\nDescription=Codex organization daemon\nStartLimitIntervalSec=120\nStartLimitBurst=5\n\n[Service]\nType=simple\n{settings}ExecStart=:{cli} app-server --listen unix:// --managed-daemon\nRestart=on-failure\nRestartSec=5\nTimeoutStopSec=330\nKillMode=mixed\nUMask=0077\n\n[Install]\nWantedBy=default.target\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_unit_handles_spaces_specifiers_and_refuses_unsafe_inputs() {
        let unit = unit(
            Path::new("/opt/Codex $test/bin/codex"),
            Path::new("/tmp/owner %x"),
            None,
            Some(10),
        )
        .unwrap();
        assert!(unit.contains("ExecStart=:\"/opt/Codex $test/bin/codex\""));
        assert!(unit.contains("\"CODEX_HOME=/tmp/owner %%x\""));
        assert!(!unit.contains("PIDFile="));
        for home in ["relative", "/tmp/bad\nExecStart=/bad"] {
            assert!(super::unit(Path::new("/opt/codex"), Path::new(home), None, None).is_err());
        }
        assert!(
            super::unit(
                Path::new("/opt/codex"),
                Path::new("/tmp/home"),
                None,
                Some(9)
            )
            .is_err()
        );
    }
}

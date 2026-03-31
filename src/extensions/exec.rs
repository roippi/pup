use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::config::Config;

/// Spawn the extension executable with inherited stdio and auth environment.
/// Returns the extension's exit code.
pub fn exec_extension(name: &str, ext_path: &Path, args: &[String], cfg: &Config) -> Result<i32> {
    // Preflight: verify binary integrity (Feature 3)
    verify_extension_binary(name, ext_path)?;

    // Load manifest for preflight checks (scope + user-agent). Missing manifest is non-fatal
    // for execution — the binary integrity was already verified above.
    let manifest_path = ext_path
        .parent()
        .context("extension path has no parent directory")?
        .join("manifest.json");

    // Determine extension version for User-Agent (default "unknown" if manifest unreadable).
    let (scope_manifest, ext_version) =
        match crate::extensions::manifest::Manifest::load(&manifest_path) {
            Ok(m) => {
                let v = m.version.clone();
                (Some(m), v)
            }
            Err(_) => (None, "unknown".to_string()),
        };

    // Preflight: check OAuth2 scope coverage (Feature 1)
    if let Some(ref manifest) = scope_manifest {
        check_extension_scopes(manifest, cfg)?;
    }

    let mut cmd = std::process::Command::new(ext_path);
    cmd.args(args);

    // Inject auth env vars and User-Agent (Feature 4)
    inject_auth_env(&mut cmd, cfg, name, &ext_version);

    let status = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to execute extension {}: {e}", ext_path.display()))?;

    // On Unix, if the process was killed by a signal, status.code() returns None.
    // Use the standard convention of 128 + signal_number.
    let exit_code = status.code().unwrap_or_else(|| {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            status.signal().map(|s| 128 + s).unwrap_or(1)
        }
        #[cfg(not(unix))]
        {
            1
        }
    });
    Ok(exit_code)
}

/// Verify the extension binary checksum against the lock file before execution.
///
/// Skip cases (not errors):
/// - `lockfile_path()` returns None: no config dir available (e.g. WASM env) — skip silently
/// - Lock file does not exist on disk: truly fresh system, no extensions ever installed — skip silently
/// - Empty stored checksum: symlinked extension — user-controlled target, intentionally exempt
///
/// Fail-closed cases (errors):
/// - Lock file exists on disk but has no entry for this extension: the extension was not registered
///   in the lock file, which is invalid since all installed extensions must have a lock entry.
///   Return Err with actionable reinstall message.
/// - Corrupt lock file: unreadable lock file means we cannot verify integrity — return Err.
/// - Checksum mismatch: binary has been modified since installation — return Err.
fn verify_extension_binary(name: &str, ext_path: &Path) -> Result<()> {
    use crate::extensions::lockfile;

    let lock_path = match lockfile::lockfile_path() {
        Some(p) => p,
        None => return Ok(()), // no config dir (WASM / unconfigured env): skip
    };

    // Lock file does not exist at all: fresh system, no extensions ever installed.
    if !lock_path.exists() {
        return Ok(());
    }

    // Lock file exists — any failure to read or parse it is an error (not a silent pass).
    let lock = lockfile::LockFile::load(&lock_path)
        .with_context(|| format!(
            "reading lock file {} — if corrupt, remove it and reinstall all extensions with 'pup extension install <name> --force'",
            lock_path.display()
        ))?;

    // Lock file exists but has no entry for this extension. This is invalid: every installed
    // extension must have been registered in the lock file during installation.
    // There is no backwards-compatibility exception — reinstall is required.
    let entry = lock.get(name).ok_or_else(|| {
        anyhow::anyhow!(
        "extension '{name}' has no lock file entry — it may not have been installed correctly.\n\
         To fix: pup extension install {name} --force"
    )
    })?;

    if entry.sha256.is_empty() {
        return Ok(()); // symlink: intentionally exempt from checksum verification
    }

    let actual = lockfile::sha256_of_file(ext_path)
        .with_context(|| format!("computing checksum for extension '{name}'"))?;

    if actual != entry.sha256 {
        bail!(
            "extension '{name}' binary checksum mismatch — the binary may have been tampered with.\n\
             Expected: {}\n\
             Actual:   {}\n\
             To reinstall: pup extension install {name} --force",
            entry.sha256,
            actual
        );
    }

    Ok(())
}

/// Preflight scope check for extensions.
///
/// Returns Ok(()) if:
///   - The manifest declares no required scopes, or
///   - Auth is API key (no access_token), or
///   - token_scopes is unavailable (e.g. DD_ACCESS_TOKEN env var), or
///   - All required scopes are present in the token.
///
/// Returns Err with an actionable message if any required scope is missing.
fn check_extension_scopes(
    manifest: &crate::extensions::manifest::Manifest,
    cfg: &crate::config::Config,
) -> Result<()> {
    if manifest.required_scopes.is_empty() {
        return Ok(());
    }
    if cfg.access_token.is_none() {
        return Ok(()); // API key auth: no scope concept
    }
    let scope_str = match cfg.token_scopes.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()), // no scope info available: skip silently
    };

    let token_scopes: std::collections::HashSet<&str> = scope_str.split_whitespace().collect();

    let missing: Vec<&str> = manifest
        .required_scopes
        .iter()
        .map(String::as_str)
        .filter(|s| !token_scopes.contains(*s))
        .collect();

    if !missing.is_empty() {
        bail!(
            "extension '{}' requires OAuth2 scopes that your current token does not cover.\n\
             Missing scopes: {}\n\
             Run 'pup auth login' to obtain a new token with the required scopes.",
            manifest.name,
            missing.join(", ")
        );
    }

    Ok(())
}

/// Set (or remove) auth and config environment variables on the child process command.
/// Variables not active in the current config are explicitly removed to prevent
/// stale credentials from leaking through the parent environment.
fn inject_auth_env(
    cmd: &mut std::process::Command,
    cfg: &Config,
    ext_name: &str,
    ext_version: &str,
) {
    // Always set site and output format.
    cmd.env("DD_SITE", &cfg.site);
    cmd.env("PUP_OUTPUT", cfg.output_format.to_string());

    // Set or unset auth variables based on current config.
    match &cfg.access_token {
        Some(token) => {
            cmd.env("DD_ACCESS_TOKEN", token);
        }
        None => {
            cmd.env_remove("DD_ACCESS_TOKEN");
        }
    }
    match &cfg.api_key {
        Some(key) => {
            cmd.env("DD_API_KEY", key);
        }
        None => {
            cmd.env_remove("DD_API_KEY");
        }
    }
    match &cfg.app_key {
        Some(key) => {
            cmd.env("DD_APP_KEY", key);
        }
        None => {
            cmd.env_remove("DD_APP_KEY");
        }
    }
    match &cfg.org {
        Some(org) => {
            cmd.env("DD_ORG", org);
        }
        None => {
            cmd.env_remove("DD_ORG");
        }
    }

    // Boolean mode flags - set when active, unset when not.
    if cfg.auto_approve {
        cmd.env("PUP_AUTO_APPROVE", "true");
    } else {
        cmd.env_remove("PUP_AUTO_APPROVE");
    }
    if cfg.read_only {
        cmd.env("PUP_READ_ONLY", "true");
    } else {
        cmd.env_remove("PUP_READ_ONLY");
    }
    if cfg.agent_mode {
        cmd.env("PUP_AGENT_MODE", "true");
    } else {
        cmd.env_remove("PUP_AGENT_MODE");
    }

    // Inject User-Agent string so extensions can attribute their Datadog API calls.
    // Extensions should read PUP_USER_AGENT and use it as their HTTP User-Agent header.
    let ua = crate::useragent::get_for_extension(ext_name, ext_version);
    cmd.env("PUP_USER_AGENT", ua);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, OutputFormat};
    use crate::extensions::manifest::Manifest;

    fn make_cfg_oauth(scopes: Option<&str>) -> Config {
        Config {
            api_key: None,
            app_key: None,
            access_token: Some("test-token".into()),
            token_scopes: scopes.map(String::from),
            site: "datadoghq.com".into(),
            org: None,
            output_format: OutputFormat::Json,
            auto_approve: false,
            agent_mode: false,
            read_only: false,
        }
    }

    fn make_cfg_apikeys() -> Config {
        Config {
            api_key: Some("key".into()),
            app_key: Some("app".into()),
            access_token: None,
            token_scopes: None,
            site: "datadoghq.com".into(),
            org: None,
            output_format: OutputFormat::Json,
            auto_approve: false,
            agent_mode: false,
            read_only: false,
        }
    }

    fn make_manifest(required_scopes: Vec<&str>) -> Manifest {
        Manifest {
            name: "test-ext".into(),
            version: "1.0.0".into(),
            source: "github:owner/pup-test-ext".into(),
            installed_at: "2026-01-01T00:00:00Z".into(),
            binary: "pup-test-ext".into(),
            description: "".into(),
            installed_by_pup: "0.39.0".into(),
            required_scopes: required_scopes.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_no_required_scopes_always_passes() {
        let cfg = make_cfg_oauth(Some("monitors_read"));
        let m = make_manifest(vec![]);
        assert!(check_extension_scopes(&m, &cfg).is_ok());
    }

    #[test]
    fn test_api_key_auth_skips_check() {
        let cfg = make_cfg_apikeys();
        let m = make_manifest(vec!["monitors_write", "org_management"]);
        assert!(check_extension_scopes(&m, &cfg).is_ok());
    }

    #[test]
    fn test_no_scope_info_skips_check() {
        // token present but token_scopes is None (DD_ACCESS_TOKEN path)
        let cfg = make_cfg_oauth(None);
        let m = make_manifest(vec!["monitors_read"]);
        assert!(check_extension_scopes(&m, &cfg).is_ok());
    }

    #[test]
    fn test_empty_scope_string_skips_check() {
        let cfg = make_cfg_oauth(Some(""));
        let m = make_manifest(vec!["monitors_read"]);
        assert!(check_extension_scopes(&m, &cfg).is_ok());
    }

    #[test]
    fn test_all_scopes_present_passes() {
        let cfg = make_cfg_oauth(Some("monitors_read monitors_write dashboards_read"));
        let m = make_manifest(vec!["monitors_read", "monitors_write"]);
        assert!(check_extension_scopes(&m, &cfg).is_ok());
    }

    #[test]
    fn test_missing_scope_returns_actionable_error() {
        let cfg = make_cfg_oauth(Some("monitors_read"));
        let m = make_manifest(vec!["monitors_read", "monitors_write"]);
        let err = check_extension_scopes(&m, &cfg).unwrap_err().to_string();
        assert!(
            err.contains("monitors_write"),
            "should name missing scope: {err}"
        );
        assert!(
            err.contains("pup auth login"),
            "should suggest re-login: {err}"
        );
        assert!(err.contains("test-ext"), "should name the extension: {err}");
    }

    #[test]
    fn test_multiple_missing_scopes_all_named() {
        let cfg = make_cfg_oauth(Some("dashboards_read"));
        let m = make_manifest(vec!["monitors_read", "monitors_write"]);
        let err = check_extension_scopes(&m, &cfg).unwrap_err().to_string();
        assert!(
            err.contains("monitors_read"),
            "should name first missing: {err}"
        );
        assert!(
            err.contains("monitors_write"),
            "should name second missing: {err}"
        );
    }
}

//! Required OAuth2 scopes for built-in pup command groups.
//! Used for OAuth2 preflight scope checks. API key auth bypasses these checks.
//! Scope mapping uses the clap subcommand name (lowercase, matching the Commands enum's
//! rename_all = "kebab-case" or similar).

use std::collections::HashMap;

/// Returns a map from clap subcommand name to minimum required OAuth2 scopes.
/// The key matches the string returned by clap for the top-level subcommand
/// (e.g. "monitors", "audit-logs"). Only read scopes are declared here — the
/// check fires for all subcommands in a group, including write operations.
/// This means the check is conservative: it only verifies the user has read
/// access, not necessarily write access. Users who only have read tokens will
/// see a 403 for write operations (acceptable for the initial implementation).
pub fn command_scopes() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
    m.insert("monitors", vec!["monitors_read"]);
    m.insert("logs", vec!["logs_read_data"]);
    m.insert("dashboards", vec!["dashboards_read"]);
    m.insert("metrics", vec!["metrics_read"]);
    m.insert("slos", vec!["slos_read"]);
    m.insert("synthetics", vec!["synthetics_read"]);
    m.insert("events", vec!["events_read"]);
    m.insert("downtime", vec!["monitors_read"]);
    m.insert("tags", vec!["hosts_read"]);
    m.insert("users", vec!["user_access_read"]);
    m.insert("infrastructure", vec!["hosts_read"]);
    m.insert("idp", vec!["org_management"]);
    m.insert("audit-logs", vec!["audit_logs_read"]);
    m.insert("security", vec!["security_monitoring_rules_read"]);
    m.insert("organizations", vec!["org_management"]);
    m.insert("change-management", vec!["monitors_read"]);
    m.insert("cloud", vec!["integrations_read"]);
    m.insert("cases", vec!["cases_read"]);
    m.insert("service-catalog", vec!["apm_service_catalog_read"]);
    m.insert("api-keys", vec!["user_access_read"]);
    m.insert("app-keys", vec!["user_access_read"]);
    m.insert("usage", vec!["usage_read"]);
    m.insert("notebooks", vec!["notebooks_read"]);
    m.insert("rum", vec!["rum_apps_read"]);
    m.insert("cicd", vec!["ci_visibility_read"]);
    m.insert("on-call", vec!["incident_read"]);
    m.insert("fleet", vec!["hosts_read"]);
    m.insert("error-tracking", vec!["error_tracking_read"]);
    m.insert("code-coverage", vec!["code_coverage_read"]);
    m.insert("integrations", vec!["integrations_read"]);
    m.insert("containers", vec!["hosts_read"]);
    m.insert("apm", vec!["apm_read"]);
    m.insert("ddsql", vec!["logs_read_data"]);
    m.insert("investigations", vec!["bits_investigations_read"]);
    m.insert("obs-pipelines", vec!["logs_read_config"]);
    m.insert("traces", vec!["apm_read"]);
    // Commands with no OAuth2 scope restriction or handled elsewhere:
    // misc, completions, version, auth, extension, network (mixed public/private),
    // scorecards, acp, agent, alias, skills, status-pages, hamr, data-governance
    m
}

/// Check whether the current config's token covers the required scopes for `command`.
///
/// Bypassed when:
/// - Using API key auth (no access_token)
/// - token_scopes is None or empty (token from DD_ACCESS_TOKEN or legacy token without scope)
/// - Command is not in the scope table
///
/// Returns Err with actionable message when a required scope is missing.
pub fn check_command_scopes(command: &str, cfg: &crate::config::Config) -> anyhow::Result<()> {
    if cfg.access_token.is_none() {
        return Ok(());
    }
    let scope_str = match cfg.token_scopes.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };
    let required = match command_scopes().get(command) {
        Some(r) => r.clone(),
        None => return Ok(()),
    };
    let token_scopes: std::collections::HashSet<&str> = scope_str.split_whitespace().collect();
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|s| !token_scopes.contains(s))
        .collect();
    if !missing.is_empty() {
        anyhow::bail!(
            "command '{command}' requires OAuth2 scopes that your current token does not cover.\n\
             Missing scopes: {}\n\
             Run 'pup auth login' to obtain a new token with the required scopes.",
            missing.join(", ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, OutputFormat};

    fn make_oauth_cfg(scopes: Option<&str>) -> Config {
        Config {
            api_key: None,
            app_key: None,
            access_token: Some("tok".into()),
            token_scopes: scopes.map(String::from),
            site: "datadoghq.com".into(),
            org: None,
            output_format: OutputFormat::Json,
            auto_approve: false,
            agent_mode: false,
            read_only: false,
        }
    }

    #[test]
    fn test_api_key_auth_bypasses_all_scope_checks() {
        let cfg = Config {
            api_key: Some("k".into()),
            app_key: Some("a".into()),
            access_token: None,
            token_scopes: None,
            site: "datadoghq.com".into(),
            org: None,
            output_format: OutputFormat::Json,
            auto_approve: false,
            agent_mode: false,
            read_only: false,
        };
        for cmd in &["monitors", "logs", "dashboards", "security"] {
            assert!(
                check_command_scopes(cmd, &cfg).is_ok(),
                "should pass for {cmd}"
            );
        }
    }

    #[test]
    fn test_no_scope_info_bypasses_check() {
        let cfg = make_oauth_cfg(None);
        assert!(check_command_scopes("monitors", &cfg).is_ok());
    }

    #[test]
    fn test_sufficient_scopes_pass() {
        let cfg = make_oauth_cfg(Some(
            "monitors_read monitors_write dashboards_read logs_read_data",
        ));
        assert!(check_command_scopes("monitors", &cfg).is_ok());
        assert!(check_command_scopes("dashboards", &cfg).is_ok());
        assert!(check_command_scopes("logs", &cfg).is_ok());
    }

    #[test]
    fn test_missing_scope_returns_clear_error() {
        let cfg = make_oauth_cfg(Some("dashboards_read"));
        let err = check_command_scopes("monitors", &cfg)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("monitors_read"),
            "should name the missing scope: {err}"
        );
        assert!(
            err.contains("pup auth login"),
            "should suggest re-login: {err}"
        );
        assert!(err.contains("monitors"), "should name the command: {err}");
    }

    #[test]
    fn test_unlisted_commands_always_pass() {
        let cfg = make_oauth_cfg(Some("dashboards_read"));
        assert!(check_command_scopes("version", &cfg).is_ok());
        assert!(check_command_scopes("auth", &cfg).is_ok());
        assert!(check_command_scopes("completions", &cfg).is_ok());
        assert!(check_command_scopes("extension", &cfg).is_ok());
        assert!(check_command_scopes("misc", &cfg).is_ok());
    }

    #[test]
    fn test_all_declared_scopes_are_known_oauth_scopes() {
        // Validates that every scope string in the map is a recognized Datadog OAuth2 scope.
        use crate::auth::types::all_known_scopes;
        let known: std::collections::HashSet<&str> = all_known_scopes().into_iter().collect();
        for (cmd, scopes) in command_scopes() {
            for scope in &scopes {
                assert!(
                    known.contains(*scope),
                    "scope '{scope}' for command '{cmd}' not found in all_known_scopes()"
                );
            }
        }
    }

    #[test]
    fn test_all_scope_map_commands_match_known_commands() {
        // Sanity check: every key in command_scopes() corresponds to a real pup command name.
        // This is a documentation test — update the list if commands are added/renamed.
        let known_commands = vec![
            "monitors",
            "logs",
            "dashboards",
            "metrics",
            "slos",
            "synthetics",
            "events",
            "downtime",
            "tags",
            "users",
            "infrastructure",
            "idp",
            "audit-logs",
            "security",
            "organizations",
            "change-management",
            "cloud",
            "cases",
            "service-catalog",
            "api-keys",
            "app-keys",
            "usage",
            "notebooks",
            "rum",
            "cicd",
            "on-call",
            "fleet",
            "error-tracking",
            "code-coverage",
            "integrations",
            "containers",
            "apm",
            "ddsql",
            "investigations",
            "obs-pipelines",
            "traces",
        ];
        let known_set: std::collections::HashSet<&&str> = known_commands.iter().collect();
        for cmd in command_scopes().keys() {
            assert!(
                known_set.contains(&cmd),
                "command '{cmd}' in scope map but not in known_commands list — update the test"
            );
        }
    }
}

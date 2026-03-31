# Extensions Guide

## Overview

Pup extensions are standalone executables that add new subcommands to pup. When you run `pup terraform ...`, pup checks if `terraform` is a built-in command. If not, it looks for an installed extension named `pup-terraform` and runs it with your arguments and auth credentials.

Extensions let teams ship experimental features independently without modifying pup's core or doing a full release. Any language works - extensions are just executables.

## Quick Start

### Install an extension from GitHub

```bash
# Install from a GitHub repository (downloads the latest release)
pup extension install jkirsteins/pup-hello

# Install a specific release version
pup extension install jkirsteins/pup-hello --tag v1.0.0
```

### Install from a local file

```bash
# Install from a local binary
pup extension install --local /path/to/pup-my-tool

# Install as a symlink (for development)
pup extension install --local /path/to/pup-my-tool --link
```

### Use it

```bash
# The extension becomes a pup subcommand
pup my-tool --some-flag value
```

### Manage extensions

```bash
# List installed extensions
pup extension list

# List in table format
pup -o table extension list

# Upgrade a single extension to the latest release
pup extension upgrade my-tool

# Upgrade all installed extensions
pup extension upgrade --all

# Remove an extension
pup extension remove my-tool
```

## Writing an Extension

An extension is any executable named `pup-<name>`. It can be a shell script, a compiled binary, a Python script with a shebang - anything that can run.

### Minimal example (shell script)

```bash
#!/bin/bash
echo "Hello from pup extension!"
echo "Site: $DD_SITE"
echo "Args: $@"
```

Save this as `pup-hello`, make it executable (`chmod +x pup-hello`), and install it:

```bash
pup extension install --local ./pup-hello
pup hello world
# Output:
# Hello from pup extension!
# Site: datadoghq.com
# Args: world
```

### Naming rules

- The executable must be named `pup-<name>` (or `pup-<name>.exe` on Windows)
- `<name>` must be lowercase letters, digits, and hyphens only, starting with a letter
- `<name>` must not conflict with a built-in pup command (e.g., `monitors`, `logs`, `auth`)

Valid: `pup-terraform`, `pup-cost-report`, `pup-lint`
Invalid: `pup-Terraform`, `pup-2fast`, `pup-my_tool`, `pup-monitors`

## Auth Forwarding

Extensions receive pup's auth credentials via environment variables. This means extensions don't need to implement keychain access, token refresh, or config file parsing.

### Environment variables set by pup

| Variable | Set When | Value |
|---|---|---|
| `DD_ACCESS_TOKEN` | OAuth2 auth is active | Current (non-expired) access token |
| `DD_API_KEY` | API key is configured | API key |
| `DD_APP_KEY` | App key is configured | Application key |
| `DD_SITE` | Always | Datadog site (e.g., `datadoghq.com`) |
| `DD_ORG` | Org is specified | Named org session |
| `PUP_OUTPUT` | Always | Output format (`json`, `table`, `yaml`, `csv`, `tsv`) |
| `PUP_AUTO_APPROVE` | `--yes` flag or agent mode | `true` |
| `PUP_READ_ONLY` | Read-only mode | `true` |
| `PUP_AGENT_MODE` | Agent mode | `true` |
| `PUP_USER_AGENT` | Always (when spawned via pup) | `pup/<version> extension/<name>/<version>` |

Pup refreshes the OAuth2 token if needed before passing it to the extension, so extensions always receive a valid token.

Variables not active in the current session are explicitly removed from the child environment to prevent stale credentials from leaking through the parent shell.

### Example: using auth in a Python extension

```python
#!/usr/bin/env python3
import os, requests

token = os.environ.get("DD_ACCESS_TOKEN")
site = os.environ.get("DD_SITE", "datadoghq.com")

headers = {"Authorization": f"Bearer {token}"} if token else {
    "DD-API-KEY": os.environ.get("DD_API_KEY", ""),
    "DD-APPLICATION-KEY": os.environ.get("DD_APP_KEY", ""),
}

resp = requests.get(f"https://api.{site}/api/v1/dashboard", headers=headers)
print(resp.json())
```

### User-Agent attribution

When pup spawns an extension, it sets the `PUP_USER_AGENT` environment variable to a string that identifies both pup and the extension:

```
pup/<pup-version> extension/<ext-name>/<ext-version>
```

Example: `pup/0.41.0 extension/hello/1.2.3`

Extensions should read this variable and use it as (or incorporate it into) the `User-Agent` HTTP header when constructing their Datadog API client. This allows Datadog to attribute extension-induced API traffic back to the specific extension.

```bash
# Shell: use PUP_USER_AGENT as your User-Agent header
curl -H "User-Agent: ${PUP_USER_AGENT}" \
     -H "DD-API-KEY: ${DD_API_KEY}" \
     "https://api.${DD_SITE}/api/v1/dashboard"
```

```python
# Python: incorporate PUP_USER_AGENT into your HTTP client
import os, requests

ua = os.environ.get("PUP_USER_AGENT", f"extension/{os.path.basename(__file__)}")
headers = {"User-Agent": ua, "DD-API-KEY": os.environ.get("DD_API_KEY", "")}
resp = requests.get(f"https://api.{os.environ.get('DD_SITE','datadoghq.com')}/api/v1/dashboard", headers=headers)
```

If `PUP_USER_AGENT` is not set (e.g., the extension is invoked directly outside pup), fall back to your own identity string.

### Example: using auth in a Rust extension

Extensions written in Rust can use the `datadog-api-client` crate. The standard Datadog SDK env vars (`DD_API_KEY`, `DD_APP_KEY`, `DD_SITE`) are forwarded automatically, so most SDKs will work without any extra configuration.

## Global Flags

Pup's global flags (`--output`, `--yes`, `--agent`, `--read-only`, `--org`) are parsed by pup before dispatching to the extension. They are NOT passed as CLI arguments to the extension - instead, they are forwarded as environment variables (see the table above).

```bash
# --output table is consumed by pup, extension receives PUP_OUTPUT=table
pup --output table my-tool do-something

# The extension receives only: ["do-something"]
# Not: ["--output", "table", "do-something"]
```

Extension-specific flags (anything pup doesn't recognize) are passed through to the extension unchanged:

```bash
pup my-tool plan --workspace prod --var-file vars.tfvars
# Extension receives: ["plan", "--workspace", "prod", "--var-file", "vars.tfvars"]
```

## Installation Details

### GitHub install

```bash
pup extension install owner/repo
```

Downloads the platform-specific binary from the repository's latest GitHub Release and installs it. The extension name is derived from the repo name (stripping the `pup-` prefix if present). For example, `jkirsteins/pup-hello` installs as `hello`.

GitHub releases must include assets following the naming convention:

```
pup-<name>-<os>-<arch>
```

Where:
- `<name>` is the extension name (e.g., `hello`)
- `<os>` is one of: `darwin`, `linux`, `windows`
- `<arch>` is one of: `x86_64`, `aarch64`

Example assets for an extension named `hello`:

```
pup-hello-darwin-aarch64
pup-hello-darwin-x86_64
pup-hello-linux-aarch64
pup-hello-linux-x86_64
pup-hello-windows-x86_64.exe
```

To install a specific release tag:

```bash
pup extension install owner/repo --tag v1.0.0
```

### Local install (copy)

```bash
pup extension install --local /path/to/pup-my-tool
```

Copies the binary into pup's extensions directory and sets executable permissions.

### Local install (symlink)

```bash
pup extension install --local /path/to/pup-my-tool --link
```

Creates a symlink instead of copying. Useful during development so changes to the source binary take effect immediately without reinstalling.

### Custom name

```bash
pup extension install --local /path/to/my-binary --name my-tool
```

By default, the extension name is derived from the filename (stripping `pup-` prefix and `.exe` suffix) for local installs, or from the repo name for GitHub installs. Use `--name` to override.

### Force reinstall

```bash
pup extension install --local /path/to/pup-my-tool --force
pup extension install owner/repo --force
```

Overwrites an existing extension with the same name.

### Declaring required OAuth2 scopes

```bash
pup extension install owner/repo --scopes monitors_read,dashboards_read
pup extension install --local ./pup-my-tool --scopes monitors_read,monitors_write
```

The `--scopes` flag accepts a comma-separated list of OAuth2 scope names. These are stored in the extension's manifest and checked at invocation time.

**When the check fires**: only when the user is authenticated via OAuth2 (`pup auth login`) and the current token's scope list is available.

**Bypassed when**:
- Using API key auth (`DD_API_KEY` + `DD_APP_KEY`) — scopes are an OAuth2 concept
- The token was provided via `DD_ACCESS_TOKEN` environment variable (no scope metadata available)
- The extension declares no required scopes

**Error example** when scopes are missing:

```
extension 'my-tool' requires OAuth2 scopes that your current token does not cover.
Missing scopes: monitors_read
Run 'pup auth login' to obtain a new token with the required scopes.
```

Scopes are preserved across upgrades — `pup extension upgrade` carries forward the `required_scopes` from the installed manifest.

## Upgrading Extensions

### Upgrade a single extension

```bash
pup extension upgrade hello
```

Checks the GitHub release for a newer version. If one is available, downloads and installs it. If the extension is already at the latest version, prints a message and does nothing.

### Upgrade all extensions

```bash
pup extension upgrade --all
```

Iterates through all installed extensions. GitHub-sourced extensions are checked for updates and upgraded if a newer release is available. Local extensions are skipped with a message.

Only GitHub-sourced extensions can be upgraded automatically. Extensions installed from local files must be reinstalled manually:

```bash
pup extension install --local /path/to/updated-binary --force
```

## Extension Directory

Extensions are stored in pup's config directory:

```
<config_dir>/
  extensions.lock          # version + SHA256 pin per extension (written by pup)
  extensions/
    pup-my-tool/
      pup-my-tool          # the executable
      manifest.json        # metadata (written by pup at install time)
```

`extensions.lock` is a JSON file maintained by pup. It records the installed version and SHA256 checksum of each extension binary. On every invocation, pup verifies the on-disk binary against the stored checksum. If the binary has been modified since installation, pup refuses to run it with an error.

Symlinked extensions (installed with `--link`) are exempt — the symlink target is user-controlled and may change legitimately during development.

The config directory location depends on your platform:
- **macOS**: `~/Library/Application Support/pup/extensions/`
- **Linux**: `~/.config/pup/extensions/` (or `$XDG_CONFIG_HOME/pup/extensions/`)
- **Windows**: `%APPDATA%\pup\extensions\`

Override with `PUP_CONFIG_DIR` environment variable.

## Exit Codes

Pup propagates the extension's exit code. If the extension exits with code 1, pup exits with code 1. On Unix, if the extension is killed by a signal, pup exits with 128 + signal number (standard convention).

## Read-Only Mode

When pup runs in read-only mode (`--read-only`), the built-in `pup extension install`, `pup extension remove`, and `pup extension upgrade` commands are blocked. Extension dispatch itself is not blocked - instead, `PUP_READ_ONLY=true` is forwarded and the extension is responsible for honoring it.

## Command Discovery via `pup agent schema`

Extensions that need to know what pup commands are available (e.g., to generate tool definitions for AI assistants) can consume the output of `pup agent schema`. This outputs a JSON object describing pup's full command tree.

```bash
pup agent schema | jq '.commands[0]'
```

### Schema structure per command

Each command in the `commands` array has:

| Field | Type | Present | Description |
|---|---|---|---|
| `name` | string | Always | Command name (e.g., `"get"`) |
| `full_path` | string | Always | Full command path (e.g., `"monitors get"`) |
| `description` | string | When available | Human-readable description |
| `read_only` | bool | Always | `true` if the command does not modify state |
| `args` | array | When command has positional args | Positional arguments (see below) |
| `flags` | array | When command has flags | Named `--flags` (see below) |
| `subcommands` | array | When command is a group | Nested commands |

### Positional args (`args[]`)

| Field | Type | Description |
|---|---|---|
| `name` | string | Argument identifier (e.g., `"monitor_id"`) |
| `type` | string | Always `"string"` |
| `required` | bool | Whether the argument is mandatory |
| `index` | number | 1-based position order for CLI invocation |
| `description` | string | Human-readable description (when available) |

### Named flags (`flags[]`)

| Field | Type | Description |
|---|---|---|
| `name` | string | Flag with prefix (e.g., `"--query"`) |
| `type` | string | `"bool"`, `"int"`, or `"string"` |
| `required` | bool | Whether the flag is mandatory |
| `default` | string | Default value (when one exists) |
| `description` | string | Human-readable description (when available) |

### Identifying actionable commands

Only **leaf commands** (those without `subcommands`) can be executed. Group commands like `monitors` just organize subcommands. To find leaf commands, walk the tree and collect commands where `subcommands` is absent.

### Constructing CLI invocations

To execute a command from the schema:

```
pup --output json --yes <full_path segments> <positional args in index order> --flag value
```

Positional args must come before named flags, ordered by their `index` field.

### Example: building a tool definition from schema

```python
import json, subprocess

schema = json.loads(subprocess.check_output(["pup", "agent", "schema"]))

for cmd in schema["commands"]:
    for leaf in walk_leaves(cmd):  # your recursive walker
        tool = {
            "name": leaf["full_path"].replace(" ", "_"),
            "description": leaf.get("description", ""),
            "parameters": {}
        }
        # Merge args and flags into parameters...
```

## Migrating a Feature to an Extension

To extract an existing pup feature into an extension:

1. Create a standalone executable that implements the feature
2. Read auth from environment variables instead of calling pup's internal auth
3. Name it `pup-<feature>` and test it via `pup extension install --local`
4. Remove the feature from pup's core `Commands` enum
5. Distribute the extension binary separately

## Demo Extension

A demo extension is available for testing at [jkirsteins/pup-hello](https://github.com/jkirsteins/pup-hello):

```bash
pup extension install jkirsteins/pup-hello
pup hello world
# Output:
# Hello from pup extension! (v1.1.0)
# Site: datadoghq.com
# Args: world
```

## Limitations

- **Public repositories only**: GitHub-based installation works with public repositories. Private repository support (token forwarding) is not implemented.
- **Source must be a regular file**: `pup extension install --local` requires the source path to be a regular file, not a directory.
- **Agent-mode help**: `pup --agent <ext-name> --help` prints pup's top-level schema, not the extension's help. In normal mode, `--help` is passed through to the extension.
- **Checksum verification**: pup verifies the SHA256 of each extension binary against the lock file on every invocation. Symlinked (development) installs are exempt. If an extension binary has been modified since installation, pup refuses to run it with an actionable error (`pup extension install <name> --force` to reinstall).
- **No code-signing**: Binaries are verified by SHA256 checksum (tamper detection) but not by cryptographic code signature. Only install extensions from trusted sources.

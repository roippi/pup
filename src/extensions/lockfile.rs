use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One entry in the lock file — one per installed extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockEntry {
    /// Extension version string (from manifest; "unknown" for local installs).
    pub version: String,
    /// Hex-encoded SHA256 of the installed binary.
    /// Empty string for symlinked (local-link) extensions — target is user-controlled.
    pub sha256: String,
    /// Source string from manifest (e.g. "github:owner/repo", "local:/path", "local-link:/path").
    pub source: String,
}

/// The full lock file. Keyed by extension name (without "pup-" prefix).
/// Uses BTreeMap for deterministic serialization order.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub extensions: BTreeMap<String, LockEntry>,
}

impl LockFile {
    /// Load the lock file from disk. Returns an empty LockFile if the file does not exist.
    /// Returns Err only on parse failure (corrupt file) — includes an actionable hint.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(LockFile::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading lock file {}", path.display()))?;
        serde_json::from_str(&content).with_context(|| {
            format!(
                "parsing lock file {} (corrupt — try removing it and reinstalling extensions)",
                path.display()
            )
        })
    }

    /// Save the lock file to disk (pretty-printed JSON, deterministic key order via BTreeMap).
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
            .with_context(|| format!("writing lock file {}", path.display()))?;
        Ok(())
    }

    /// Upsert an entry for an extension.
    pub fn set(&mut self, name: &str, entry: LockEntry) {
        self.extensions.insert(name.to_string(), entry);
    }

    /// Remove an entry. No-op if not present.
    pub fn remove(&mut self, name: &str) {
        self.extensions.remove(name);
    }

    /// Get an entry by extension name.
    pub fn get(&self, name: &str) -> Option<&LockEntry> {
        self.extensions.get(name)
    }
}

/// Compute the SHA256 of a file at the given path. Returns a lowercase hex string.
/// Returns an empty string for symlinks — the target binary is user-controlled and may
/// change legitimately (e.g., during development rebuilds). Checksumming the symlink
/// target would cause false positives.
pub fn sha256_of_file(path: &Path) -> Result<String> {
    if path.is_symlink() {
        return Ok(String::new());
    }
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading binary for checksum: {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Returns the path to the lock file: config_dir()/extensions.lock
/// Returns None when config_dir() is unavailable (browser WASM, unconfigured env).
/// Callers: return Ok(()) when None (no lock file support in this environment).
pub fn lockfile_path() -> Option<PathBuf> {
    crate::config::config_dir().map(|d| d.join("extensions.lock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(suffix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pup-test-lockfile-{suffix}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_lockfile_roundtrip() {
        let dir = test_dir("roundtrip");
        let path = dir.join("extensions.lock");

        let mut lock = LockFile::default();
        lock.set(
            "hello",
            LockEntry {
                version: "1.0.0".to_string(),
                sha256: "abc123".to_string(),
                source: "github:owner/pup-hello".to_string(),
            },
        );
        lock.save(&path).unwrap();

        let loaded = LockFile::load(&path).unwrap();
        let entry = loaded.get("hello").unwrap();
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.sha256, "abc123");
        assert_eq!(entry.source, "github:owner/pup-hello");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_lockfile_missing_returns_empty() {
        let result = LockFile::load(Path::new("/nonexistent/extensions.lock"));
        assert!(result.is_ok());
        assert!(result.unwrap().extensions.is_empty());
    }

    #[test]
    fn test_lockfile_corrupt_returns_err_with_hint() {
        let dir = test_dir("corrupt");
        let path = dir.join("extensions.lock");
        std::fs::write(&path, "not valid json").unwrap();

        let err = LockFile::load(&path).unwrap_err().to_string();
        assert!(
            err.contains("corrupt") || err.contains("reinstalling"),
            "error should include recovery hint: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_lockfile_remove_entry() {
        let mut lock = LockFile::default();
        lock.set(
            "hello",
            LockEntry {
                version: "1.0.0".into(),
                sha256: "abc".into(),
                source: "github:x/y".into(),
            },
        );
        assert!(lock.get("hello").is_some());
        lock.remove("hello");
        assert!(lock.get("hello").is_none());
    }

    #[test]
    fn test_lockfile_remove_nonexistent_is_noop() {
        let mut lock = LockFile::default();
        lock.remove("nonexistent"); // should not panic
    }

    #[test]
    fn test_lockfile_btreemap_sorted_keys() {
        // BTreeMap guarantees keys are sorted — verify in the serialized output
        let dir = test_dir("sorted");
        let path = dir.join("extensions.lock");
        let mut lock = LockFile::default();
        lock.set(
            "zebra",
            LockEntry {
                version: "1.0".into(),
                sha256: "z".into(),
                source: "local:/z".into(),
            },
        );
        lock.set(
            "alpha",
            LockEntry {
                version: "1.0".into(),
                sha256: "a".into(),
                source: "local:/a".into(),
            },
        );
        lock.save(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let alpha_pos = content.find("alpha").unwrap();
        let zebra_pos = content.find("zebra").unwrap();
        assert!(
            alpha_pos < zebra_pos,
            "keys should be sorted alphabetically"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sha256_of_real_file() {
        let dir = test_dir("sha256");
        let path = dir.join("test.bin");
        std::fs::write(&path, b"hello world").unwrap();
        let result = sha256_of_file(&path).unwrap();
        assert_eq!(result.len(), 64, "SHA256 hex should be 64 chars");
        assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
        // Deterministic: same content produces same hash
        let result2 = sha256_of_file(&path).unwrap();
        assert_eq!(result, result2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sha256_different_contents_differ() {
        let dir = test_dir("sha256-diff");
        let p1 = dir.join("a.bin");
        let p2 = dir.join("b.bin");
        std::fs::write(&p1, b"hello").unwrap();
        std::fs::write(&p2, b"world").unwrap();
        assert_ne!(sha256_of_file(&p1).unwrap(), sha256_of_file(&p2).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn test_sha256_symlink_returns_empty() {
        let dir = test_dir("sha256-symlink");
        let target = dir.join("target");
        std::fs::write(&target, b"hello").unwrap();
        let link = dir.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let result = sha256_of_file(&link).unwrap();
        assert_eq!(result, "", "symlink should return empty string");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

use anyhow::Result;
use std::path::PathBuf;

use super::manifest::Manifest;
use crate::config;

/// Information about an installed extension, for display.
pub struct ExtensionInfo {
    pub name: String,
    pub version: String,
    pub source: String,
    pub description: String,
    pub installed_at: String,
}

/// Return the base extensions directory: config_dir()/extensions/
pub fn extension_dir() -> Option<PathBuf> {
    let dir = config::config_dir()?;
    Some(dir.join("extensions"))
}

/// Platform-appropriate executable name for an extension.
fn extension_executable_name(name: &str) -> String {
    let base = format!("pup-{name}");
    if cfg!(target_os = "windows") {
        format!("{base}.exe")
    } else {
        base
    }
}

/// Look up an installed extension by name. Returns the path to the executable if found.
pub fn extension_path(name: &str) -> Option<PathBuf> {
    let dir = extension_dir()?;
    let exe_name = extension_executable_name(name);
    let path = dir.join(format!("pup-{name}")).join(&exe_name);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// List all installed extensions by scanning the extensions directory.
pub fn list_extensions() -> Result<Vec<ExtensionInfo>> {
    let dir = match extension_dir() {
        Some(d) if d.exists() => d,
        _ => return Ok(Vec::new()),
    };

    let mut extensions = Vec::new();

    let entries = std::fs::read_dir(&dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("manifest.json");
        if let Ok(manifest) = Manifest::load(&manifest_path) {
            extensions.push(ExtensionInfo {
                name: manifest.name,
                version: manifest.version,
                source: manifest.source,
                description: manifest.description,
                installed_at: manifest.installed_at,
            });
        }
    }

    extensions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(extensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pup-test-discovery-{suffix}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_extension_executable_name() {
        let name = extension_executable_name("hello");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "pup-hello.exe");
        } else {
            assert_eq!(name, "pup-hello");
        }
    }

    #[test]
    fn test_extension_path_found() {
        let dir = make_test_dir("path-found");
        let ext_dir = dir.join("extensions").join("pup-hello");
        std::fs::create_dir_all(&ext_dir).unwrap();
        let exe_name = extension_executable_name("hello");
        let exe_path = ext_dir.join(&exe_name);
        std::fs::write(&exe_path, "#!/bin/bash\necho hello").unwrap();

        // Use PUP_CONFIG_DIR to point discovery at our test dir
        let _guard = crate::test_utils::ENV_LOCK.lock().unwrap();
        std::env::set_var("PUP_CONFIG_DIR", &dir);

        let result = extension_path("hello");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), exe_path);

        std::env::remove_var("PUP_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extension_path_not_found() {
        let dir = make_test_dir("path-not-found");
        std::fs::create_dir_all(dir.join("extensions")).unwrap();

        let _guard = crate::test_utils::ENV_LOCK.lock().unwrap();
        std::env::set_var("PUP_CONFIG_DIR", &dir);

        let result = extension_path("nonexistent");
        assert!(result.is_none());

        std::env::remove_var("PUP_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_extensions_empty() {
        let dir = make_test_dir("list-empty");
        std::fs::create_dir_all(dir.join("extensions")).unwrap();

        let _guard = crate::test_utils::ENV_LOCK.lock().unwrap();
        std::env::set_var("PUP_CONFIG_DIR", &dir);

        let result = list_extensions().unwrap();
        assert!(result.is_empty());

        std::env::remove_var("PUP_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_extensions_with_manifests() {
        let dir = make_test_dir("list-manifests");
        let ext_dir = dir.join("extensions").join("pup-hello");
        std::fs::create_dir_all(&ext_dir).unwrap();

        let manifest = Manifest {
            name: "hello".to_string(),
            version: "1.0.0".to_string(),
            source: "local:/tmp/pup-hello".to_string(),
            installed_at: "2026-03-29T00:00:00Z".to_string(),
            binary: "pup-hello".to_string(),
            description: "Hello world".to_string(),
            installed_by_pup: "0.39.0".to_string(),
        };
        manifest.save(&ext_dir.join("manifest.json")).unwrap();

        let _guard = crate::test_utils::ENV_LOCK.lock().unwrap();
        std::env::set_var("PUP_CONFIG_DIR", &dir);

        let result = list_extensions().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "hello");
        assert_eq!(result[0].version, "1.0.0");

        std::env::remove_var("PUP_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

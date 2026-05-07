// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::models::{Loader, VersionManifest};
use log::{debug, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolves the full classpath for a Minecraft launch, handling:
///   - platform rules (via `Library::should_include`)
///   - native JAR exclusion
///   - parent/child version merging with child-wins conflict resolution
///   - Forge universal JAR detection
pub struct ClasspathResolver<'a> {
    manifest: &'a VersionManifest,
    /// Optional parent manifest (when `inheritsFrom` is set).
    parent: Option<&'a VersionManifest>,
    lib_dir: PathBuf,
}

impl<'a> ClasspathResolver<'a> {
    pub fn new(
        manifest: &'a VersionManifest,
        parent: Option<&'a VersionManifest>,
        lib_dir: &Path,
    ) -> Self {
        Self {
            manifest,
            parent,
            lib_dir: lib_dir.to_path_buf(),
        }
    }

    /// Build and return the platform classpath string.
    pub fn build(&self) -> String {
        let mut paths: Vec<String> = Vec::new();
        // key → path, used for child-wins conflict resolution
        let mut seen: HashMap<String, String> = HashMap::new();

        // Parent libraries first, then child overrides them.
        if let Some(parent) = self.parent {
            self.collect_libraries(parent, false, &mut paths, &mut seen);
        }
        self.collect_libraries(self.manifest, true, &mut paths, &mut seen);

        // Add version JARs.
        self.add_version_jars(&mut paths);

        #[cfg(target_os = "windows")]
        return paths.join(";");
        #[cfg(not(target_os = "windows"))]
        return paths.join(":");
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn collect_libraries(
        &self,
        manifest: &VersionManifest,
        is_child: bool,
        paths: &mut Vec<String>,
        seen: &mut HashMap<String, String>,
    ) {
        for lib in &manifest.libraries {
            if !lib.should_include() {
                continue;
            }
            if lib.is_native() {
                debug!("Skipping native: {}", lib.name);
                continue;
            }

            let rel_path = lib.get_path();
            let full_path = self.lib_dir.join(&rel_path);

            if !full_path.exists() {
                warn!("Library not found: {}", full_path.display());
                continue;
            }

            let path_str = full_path.to_string_lossy().to_string();
            // Maven coordinate without version → conflict key.
            let key = maven_key(&lib.name);

            if let Some(existing) = seen.get(&key) {
                if is_child {
                    // Child wins: replace existing path.
                    debug!("Conflict resolved (child wins): {key}");
                    paths.retain(|p| p != existing);
                    paths.push(path_str.clone());
                    seen.insert(key, path_str);
                }
                // Parent loses silently.
            } else {
                seen.insert(key, path_str.clone());
                paths.push(path_str);
            }
        }
    }

    fn add_version_jars(&self, paths: &mut Vec<String>) {
        let loader = Loader::from_version_id(&self.manifest.id_raw);

        // Resolved version ID: use parent's ID if we inherit.
        let resolved_id = self
            .parent
            .map(|p| p.id_raw.as_str())
            .unwrap_or(&self.manifest.id_raw);

        let client_jar = self
            .lib_dir
            .parent() // shared_dir
            .unwrap_or(Path::new("."))
            .join("versions")
            .join(resolved_id)
            .join(format!("{resolved_id}.jar"));

        let version_jar = self
            .lib_dir
            .parent()
            .unwrap_or(Path::new("."))
            .join("versions")
            .join(&self.manifest.id_raw)
            .join(format!("{}.jar", self.manifest.id_raw));

        match loader {
            Loader::Forge(_) => {
                self.push_if_exists(paths, &client_jar);
                self.push_if_exists(paths, &version_jar);
                if let Some(forge_jar) = self.find_forge_universal() {
                    self.push_if_exists(paths, &forge_jar);
                }
            }
            Loader::NeoForge(_) => {
                self.push_if_exists(paths, &version_jar);
            }
            _ => {
                self.push_if_exists(paths, &client_jar);
                if client_jar != version_jar {
                    self.push_if_exists(paths, &version_jar);
                }
            }
        }
    }

    fn push_if_exists(&self, paths: &mut Vec<String>, p: &Path) {
        if p.exists() {
            let s = p.to_string_lossy().to_string();
            if !paths.contains(&s) {
                debug!("Adding JAR: {s}");
                paths.push(s);
            }
        }
    }

    fn find_forge_universal(&self) -> Option<PathBuf> {
        let check = |manifest: &VersionManifest| -> Option<PathBuf> {
            manifest.libraries.iter().find_map(|lib| {
                if lib.name.contains("net.minecraftforge:forge:")
                    || lib.name.contains("net.minecraftforge:minecraftforge:")
                {
                    let parts: Vec<&str> = lib.name.splitn(3, ':').collect();
                    if parts.len() < 3 {
                        return None;
                    }
                    let group_path = parts[0].replace('.', "/");
                    let artifact = parts[1];
                    let version = parts[2];

                    let universal = self
                        .lib_dir
                        .join(&group_path)
                        .join(artifact)
                        .join(version)
                        .join(format!("{artifact}-{version}-universal.jar"));

                    if universal.exists() {
                        return Some(universal);
                    }

                    Some(
                        self.lib_dir
                            .join(group_path)
                            .join(artifact)
                            .join(version)
                            .join(format!("{artifact}-{version}.jar")),
                    )
                } else {
                    None
                }
            })
        };

        check(self.manifest)
            .or_else(|| self.parent.and_then(check))
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Return `group:artifact` from a full Maven coordinate (strips version + classifier).
fn maven_key(name: &str) -> String {
    let parts: Vec<&str> = name.splitn(3, ':').collect();
    match parts.as_slice() {
        [group, artifact, ..] => format!("{group}:{artifact}"),
        _ => name.to_string(),
    }
}

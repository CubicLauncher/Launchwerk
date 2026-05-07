// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Extracts native libraries (.so / .dll / .dylib) from their JAR files
//! into the per-version natives directory before launching.

use crate::error::Error;
use crate::models::VersionManifest;
use log::{debug, info, warn};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn extract_natives(
    manifest: &VersionManifest,
    lib_dir: &Path,
    natives_dir: &Path,
) -> Result<(), Error> {
    fs::create_dir_all(natives_dir)?;

    for lib in &manifest.libraries {
        if !lib.should_include() {
            continue;
        }
        // Obtener el artifact nativo (puede ser None si no es nativa)
        let native_artifact = match lib.native_artifact() {
            Some(art) => art,
            None => {
                // Si no tiene artifact nativo explícito, seguir con el antiguo método is_native
                if lib.is_native() {
                    // Usar el artifact principal (caso legacy)
                    let jar_path = lib_dir.join(lib.get_path());
                    if jar_path.exists() {
                        extract_jar(&jar_path, natives_dir)?;
                    }
                }
                continue;
            }
        };

        let jar_path = lib_dir.join(&native_artifact.path);
        if !jar_path.exists() {
            warn!("Native JAR not found, skipping: {}", jar_path.display());
            continue;
        }
        extract_jar(&jar_path, natives_dir)?;
    }
    Ok(())
}
fn extract_jar(jar_path: &Path, dest_dir: &Path) -> Result<(), Error> {
    let file = fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to open JAR {}: {e}", jar_path.display()),
        ))
    })?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        let name = entry.name().to_string();

        // Only extract native library files; skip META-INF and other resources.
        if !is_native_file(&name) {
            continue;
        }

        // Use only the filename, not any subdirectory inside the JAR.
        let file_name = Path::new(&name)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if file_name.is_empty() {
            continue;
        }

        let out_path = dest_dir.join(&file_name);

        // Skip if already extracted (size match is a quick sanity check).
        if out_path.exists() && out_path.metadata().map(|m| m.len()).unwrap_or(0) == entry.size() {
            debug!("Native already extracted: {file_name}");
            continue;
        }

        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        fs::write(&out_path, &buf)?;
        info!("Extracted native: {file_name} → {}", dest_dir.display());
    }

    Ok(())
}

fn is_native_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Skip META-INF, directories (end with /)
    if lower.starts_with("meta-inf") || lower.ends_with('/') {
        return false;
    }
    lower.ends_with(".so")
        || lower.ends_with(".dll")
        || lower.ends_with(".dylib")
        || lower.ends_with(".jnilib")
        // versioned .so files like libawt.so.1
        || lower.contains(".so.")
}

/// Collect the paths of all native JARs for this manifest.
/// Useful for logging / verification.
pub fn list_native_jars(manifest: &VersionManifest, lib_dir: &Path) -> Vec<PathBuf> {
    manifest
        .libraries
        .iter()
        .filter(|l| l.should_include() && l.is_native())
        .map(|l| lib_dir.join(l.get_path()))
        .collect()
}

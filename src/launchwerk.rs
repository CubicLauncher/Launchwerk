// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::Error;
use crate::handle::{InstanceHandle, InstanceInner};
use crate::launch_config::LaunchConfig;
use crate::models::VersionManifest;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Top-level manager for Minecraft instances.
///
/// # Example
/// ```no_run
/// # use claunch::{Launchwerk, LaunchConfig};
/// # use claunch::models::VersionManifest;
/// # use std::path::PathBuf;
/// # #[tokio::main] async fn main() {
/// let lw = Launchwerk::new(PathBuf::from("/home/user/.cubic/shared"));
/// let manifest = VersionManifest::from_file("versions/1.21/1.21.json").unwrap();
/// let config = LaunchConfig::default();
/// let handle = lw.prepare(manifest, config, PathBuf::from("/home/user/.cubic/instances/1.21"));
/// handle.launch().await.unwrap();
/// # }
/// ```
pub struct Launchwerk {
    /// Root shared directory – expected layout:
    ///   `<shared_dir>/libraries/`
    ///   `<shared_dir>/assets/`
    ///   `<shared_dir>/versions/<id>/<id>.jar`
    pub shared_dir: PathBuf,
    instances: DashMap<Uuid, Arc<InstanceInner>>,
}

impl Launchwerk {
    /// Create a new `Launchwerk` with the given shared directory.
    pub fn new(shared_dir: PathBuf) -> Self {
        Self {
            shared_dir,
            instances: DashMap::new(),
        }
    }

    /// Prepare a new instance. Returns an `InstanceHandle` that you can
    /// `.launch().await` when ready.
    ///
    /// * `manifest` – fully parsed `VersionManifest` (use `VersionManifest::from_file`).
    /// * `config`   – player name, Java path, RAM, resolution, etc.
    /// * `instance_dir` – per-instance directory (saves, mods, config).
    pub fn prepare(
        &self,
        manifest: VersionManifest,
        config: LaunchConfig,
        instance_dir: PathBuf,
    ) -> InstanceHandle {
        let handle = InstanceHandle::new(
            manifest,
            config,
            self.shared_dir.clone(),
            instance_dir,
        );

        self.instances
            .insert(handle.id(), Arc::clone(&handle.inner));

        handle
    }

    /// Retrieve an existing instance by UUID.
    /// Returns a fresh `InstanceHandle` (new broadcast receivers) backed by
    /// the same inner state, so you can watch output from a different task.
    pub fn get(&self, id: Uuid) -> Option<InstanceHandle> {
        self.instances.get(&id).map(|inner| InstanceHandle {
            stdout: inner.stdout_tx.subscribe(),
            stderr: inner.stderr_tx.subscribe(),
            inner: Arc::clone(&*inner),
        })
    }

    /// Remove an instance record from the registry.
    /// Does NOT kill a running process – call `handle.kill().await` first if needed.
    pub fn remove(&self, id: Uuid) {
        self.instances.remove(&id);
    }

    /// Number of tracked instances.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}

// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::Error;
use crate::launch_config::{LaunchConfig, QuickPlay};
use crate::models::{Argument, VersionManifest};
use crate::resolvers::ClasspathResolver;
use log::info;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

pub struct CommandBuilder<'a> {
    manifest: &'a VersionManifest,
    shared_dir: &'a Path,
    instance_dir: &'a Path,
    config: &'a LaunchConfig,
}

impl<'a> CommandBuilder<'a> {
    pub fn new(
        manifest: &'a VersionManifest,
        shared_dir: &'a Path,
        instance_dir: &'a Path,
        config: &'a LaunchConfig,
    ) -> Self {
        Self {
            manifest,
            shared_dir,
            instance_dir,
            config,
        }
    }
    pub fn verify_requirements(&self) -> Result<(), Error> {
        let lib_dir = self.shared_dir.join("libraries");
        let classpath = ClasspathResolver::new(self.manifest, None, &lib_dir).build();

        // 1. Java binary
        let java_path = &self.config.java_path;
        if !java_path.exists() {
            return Err(Error::MissingFile(format!(
                "Java binary not found: {}",
                java_path.display()
            )));
        }
        if !Self::is_executable(java_path) {
            return Err(Error::MissingFile(format!(
                "Java binary not executable: {}",
                java_path.display()
            )));
        }

        // 2. Classpath entries
        let separator = if cfg!(windows) { ';' } else { ':' };
        let mut missing_files = Vec::new();
        for entry in classpath.split(separator) {
            let p = Path::new(entry);
            if !p.exists() {
                missing_files.push(entry.to_string());
            }
        }
        if !missing_files.is_empty() {
            let msg = format!(
                "Missing classpath entries:\n  {}",
                missing_files.join("\n  ")
            );
            return Err(Error::MissingFile(msg));
        }

        // 3. Version JAR
        let version_jar = self
            .shared_dir
            .join("versions")
            .join(&self.manifest.id_raw)
            .join(format!("{}.jar", self.manifest.id_raw));
        if !version_jar.exists() {
            return Err(Error::MissingFile(format!(
                "Version JAR not found: {}",
                version_jar.display()
            )));
        }

        // 4. Verificar directorio de instancia
        if !self.instance_dir.exists() {
            return Err(Error::MissingFile(format!(
                "Instance directory does not exist: {}",
                self.instance_dir.display()
            )));
        }

        // 5. Verificar directorio de nativos (debe crearse ANTES de pasar -Djava.library.path)
        let natives_dir = self.shared_dir.join("natives").join(&self.manifest.id_raw);
        if !natives_dir.exists() {
            // Crear el directorio si no existe (aunque extract_natives debería hacerlo)
            std::fs::create_dir_all(&natives_dir)?;
        }

        Ok(())
    }

    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn is_executable(path: &Path) -> bool {
        path.exists() && path.extension().map_or(false, |ext| ext == "exe")
    }

    pub fn build(&self) -> Result<Vec<String>, Error> {
        let lib_dir = self.shared_dir.join("libraries");
        let assets_dir = self.shared_dir.join("assets");
        let natives_dir = self.shared_dir.join("natives").join(&self.manifest.id_raw);

        let classpath = ClasspathResolver::new(self.manifest, None, &lib_dir).build();
        if classpath.is_empty() {
            return Err(Error::EmptyClasspath);
        }
        self.verify_requirements()?;

        let uuid = Uuid::new_v4().to_string();
        let vars = self.build_vars(&assets_dir, &natives_dir, &uuid, &classpath);

        let mut cmd: Vec<String> = Vec::new();
        let java = self.config.java_path.to_string_lossy().to_string();
        cmd.push(java);
        self.add_jvm_flags(&mut cmd, &natives_dir, &vars);
        cmd.push("-cp".to_string());
        cmd.push(classpath);
        cmd.push(self.manifest.main_class.clone());
        self.add_game_args(&mut cmd, &vars);
        self.add_default_game_args(&mut cmd, &assets_dir);
        self.add_optional_args(&mut cmd);
        self.cleanup_unresolved(&mut cmd);

        Ok(cmd)
    }

    // ── JVM flags ─────────────────────────────────────────────────────────────

    fn add_jvm_flags(
        &self,
        cmd: &mut Vec<String>,
        natives_dir: &Path,
        vars: &HashMap<String, String>,
    ) {
        cmd.push(format!("-Djava.library.path={}", natives_dir.display()));
        cmd.push("-Dminecraft.launcher.brand=CubicLauncher".to_string());
        cmd.push("-Dminecraft.launcher.version=2.0".to_string());

        if self.config.cracked {
            info!("Offline (cracked) mode enabled");
            cmd.push("-Dminecraft.api.env=custom".to_string());
            for host in &["auth.host", "account.host", "session.host", "services.host"] {
                cmd.push(format!("-Dminecraft.api.{}=https://invalid.invalid", host));
            }
        }

        cmd.push(format!("-Xms{}", self.config.min_ram));
        cmd.push(format!("-Xmx{}", self.config.max_ram));

        // JVM args from manifest.
        // The manifest often contains ["-cp", "${classpath}"] as a pair – we
        // handle classpath ourselves, so skip the whole pair, not just each
        // token individually.
        if let Some(args) = self
            .manifest
            .arguments
            .as_ref()
            .and_then(|a| a.jvm.as_ref())
        {
            let mut skip_next = false;
            for arg in args {
                let tokens = arg.get_if_applies();

                // If the resolved token list contains -cp or ${classpath},
                // skip every token in this argument entry entirely.
                let has_cp = tokens
                    .iter()
                    .any(|t| t == "-cp" || t.contains("${classpath}"));
                if has_cp {
                    continue;
                }

                for s in tokens {
                    if skip_next {
                        skip_next = false;
                        continue;
                    }
                    // Also guard against a plain "-cp" token appearing alone.
                    if s == "-cp" {
                        skip_next = true; // skip the value that follows too
                        continue;
                    }
                    let s = replace_vars(&s, vars);
                    // After var substitution the classpath might now be a real
                    // path string – still skip it; we add it ourselves.
                    if s == "-cp" || s.contains("${classpath}") {
                        continue;
                    }
                    if !cmd.contains(&s) {
                        cmd.push(s);
                    }
                }
            }
        }
    }

    // ── Game arguments ────────────────────────────────────────────────────────

    fn add_game_args(&self, cmd: &mut Vec<String>, vars: &HashMap<String, String>) {
        // Modern JSON arguments
        if let Some(args) = self
            .manifest
            .arguments
            .as_ref()
            .and_then(|a| a.game.as_ref())
        {
            for arg in args {
                // Skip demo / quickplay if not configured
                if let Argument::Plain(s) = arg {
                    if self.should_skip_arg(s) {
                        continue;
                    }
                }
                for s in arg.get_if_applies() {
                    if !self.should_skip_arg(&s) {
                        cmd.push(replace_vars(&s, vars));
                    }
                }
            }
            return;
        }

        // Legacy minecraftArguments string (pre-1.13)
        if let Some(legacy) = &self.manifest.minecraft_arguments {
            for token in legacy.split_whitespace() {
                cmd.push(replace_vars(token, vars));
            }
        }
    }

    fn should_skip_arg(&self, arg: &str) -> bool {
        const DEMO_ARGS: &[&str] = &["--demo"];
        const QP_ARGS: &[&str] = &[
            "--quickPlaySingleplayer",
            "--quickPlayMultiplayer",
            "--quickPlayRealms",
            "--quickPlayPath",
        ];
        if DEMO_ARGS.contains(&arg) && !self.config.demo_mode {
            return true;
        }
        if QP_ARGS.contains(&arg) && self.config.quick_play.is_none() {
            return true;
        }
        false
    }

    fn add_default_game_args(&self, cmd: &mut Vec<String>, assets_dir: &Path) {
        let defaults: &[(&str, &dyn Fn() -> String)] = &[
            ("--width", &|| self.config.width.to_string()),
            ("--height", &|| self.config.height.to_string()),
            ("--username", &|| self.config.username.clone()),
            ("--version", &|| self.manifest.id_raw.clone()),
            ("--assetsDir", &|| assets_dir.display().to_string()),
            ("--assetIndex", &|| self.manifest.asset_index.id.clone()),
            ("--gameDir", &|| self.instance_dir.display().to_string()),
            ("--accessToken", &|| "0".to_string()),
            ("--userType", &|| "legacy".to_string()),
        ];

        for (flag, value_fn) in defaults {
            if !cmd.contains(&flag.to_string()) {
                let val = value_fn();
                if !val.is_empty() {
                    cmd.push(flag.to_string());
                    cmd.push(val);
                }
            }
        }
    }

    fn add_optional_args(&self, cmd: &mut Vec<String>) {
        if self.config.demo_mode && !cmd.contains(&"--demo".to_string()) {
            cmd.push("--demo".to_string());
        }

        if let Some(qp) = &self.config.quick_play {
            let (flag, value) = match qp {
                QuickPlay::Singleplayer(v) => ("--quickPlaySingleplayer", v),
                QuickPlay::Multiplayer(v) => ("--quickPlayMultiplayer", v),
                QuickPlay::Realms(v) => ("--quickPlayRealms", v),
            };
            if !cmd.contains(&flag.to_string()) {
                cmd.push(flag.to_string());
                cmd.push(value.clone());
            }
        }
    }

    // ── Utilities ─────────────────────────────────────────────────────────────

    fn build_vars(
        &self,
        assets_dir: &Path,
        natives_dir: &Path,
        uuid: &str,
        classpath: &str,
    ) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        vars.insert("auth_player_name".into(), self.config.username.clone());
        vars.insert("version_name".into(), self.manifest.id_raw.clone());
        vars.insert(
            "game_directory".into(),
            self.instance_dir.display().to_string(),
        );
        vars.insert("assets_root".into(), assets_dir.display().to_string());
        vars.insert(
            "assets_index_name".into(),
            self.manifest.asset_index.id.clone(),
        );
        vars.insert("auth_uuid".into(), uuid.to_string());
        vars.insert("auth_access_token".into(), "0".into());
        vars.insert("user_type".into(), "legacy".into());
        vars.insert("user_properties".into(), "{}".into());
        vars.insert("version_type".into(), "release".into());
        vars.insert(
            "natives_directory".into(),
            natives_dir.display().to_string(),
        );
        vars.insert(
            "library_directory".into(),
            self.shared_dir.join("libraries").display().to_string(),
        );
        vars.insert("classpath".into(), classpath.to_string());

        #[cfg(windows)]
        vars.insert("classpath_separator".into(), ";".into());
        #[cfg(not(windows))]
        vars.insert("classpath_separator".into(), ":".into());

        vars
    }

    fn cleanup_unresolved(&self, cmd: &mut Vec<String>) {
        let mut remove: Vec<usize> = Vec::new();
        for (i, arg) in cmd.iter().enumerate() {
            if arg.contains("${") {
                remove.push(i);
                // If the preceding arg was a --flag, remove it too.
                if i > 0 && cmd[i - 1].starts_with("--") && !cmd[i - 1].contains("${") {
                    remove.push(i - 1);
                }
            }
        }
        remove.sort_unstable();
        remove.dedup();
        remove.reverse();
        for idx in remove {
            info!("Removing unresolved placeholder: {}", cmd[idx]);
            cmd.remove(idx);
        }
    }
}

fn replace_vars(s: &str, vars: &HashMap<String, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("${{{k}}}"), v);
    }
    out.replace("${launcher_name}", "CubicLauncher")
        .replace("${launcher_version}", "2.0")
}

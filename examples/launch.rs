// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Uso:
//   BASE_DIR=/home/user/.cubic VERSION=1.21.4 JAVA=/usr/bin/java cargo run --example launch

use launchwerk::models::VersionManifest;
use launchwerk::{LaunchConfig, Launchwerk};
use std::env;
use std::path::PathBuf;
use tokio::sync::broadcast::error::RecvError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // ── Configuración desde variables de entorno ──────────────────────────
    let base_dir = env::var("BASE_DIR").unwrap_or_else(|_| "/home/santiagolxx/.cubic/".to_string());
    let version = env::var("VERSION").unwrap_or_else(|_| "26.2-snapshot-6".to_string());
    let java_path =
        env::var("JAVA").unwrap_or_else(|_| "/usr/lib/jvm/java-25-openjdk/bin/java".to_string());
    let username = env::var("USERNAME").unwrap_or_else(|_| "Player".to_string());

    let shared_dir = PathBuf::from(&base_dir).join("shared");
    let instance_dir = PathBuf::from(&base_dir).join("instances").join(&version);
    let version_json = shared_dir
        .join("versions")
        .join(&version)
        .join(format!("{version}.json"));

    println!("=== CLaunch v2 ===");
    println!("  shared_dir:   {}", shared_dir.display());
    println!("  instance_dir: {}", instance_dir.display());
    println!("  version_json: {}", version_json.display());
    println!("  java:         {java_path}");
    println!("  username:     {username}");
    println!();

    // ── Cargar manifest ───────────────────────────────────────────────────
    let manifest = VersionManifest::from_file(&version_json).unwrap_or_else(|e| {
        eprintln!("Error leyendo {}: {e}", version_json.display());
        std::process::exit(1);
    });

    println!(
        "Manifest cargado: {} (Java {})",
        manifest.id,
        manifest.java_major_version()
    );

    // ── Configurar launch ─────────────────────────────────────────────────
    let config = LaunchConfig::builder()
        .java_path(&java_path)
        .username(&username)
        .ram("512M", "2G")
        .resolution(854, 480)
        .cracked(true)
        .build();
    // DEBUG: imprimir classpath
    use launchwerk::resolvers::ClasspathResolver;
    let lib_dir = PathBuf::from(&base_dir).join("shared").join("libraries");
    let cp = ClasspathResolver::new(&manifest, None, &lib_dir).build();
    println!("=== CLASSPATH ===");
    for entry in cp.split(':') {
        println!("  {entry}");
    }
    println!("=== FIN CLASSPATH ===");
    // ── Preparar instancia ────────────────────────────────────────────────
    let lw = Launchwerk::new(shared_dir);
    let handle = lw.prepare(manifest, config, instance_dir);

    println!("Instancia preparada: {}", handle.id());
    println!("Loader: {}", handle.loader());

    // ── Suscribirse a stdout/stderr antes de lanzar ───────────────────────
    let mut stdout_rx = handle.subscribe_stdout();
    let mut stderr_rx = handle.subscribe_stderr();

    // Task que imprime stdout del juego
    tokio::spawn(async move {
        loop {
            match stdout_rx.recv().await {
                Ok(line) => println!("[MC] {line}"),
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => eprintln!("[stdout] {n} mensajes perdidos"),
            }
        }
    });

    // Task que imprime stderr del juego
    tokio::spawn(async move {
        loop {
            match stderr_rx.recv().await {
                Ok(line) => eprintln!("[MC:err] {line}"),
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => eprintln!("[stderr] {n} mensajes perdidos"),
            }
        }
    });

    // ── Lanzar y esperar ──────────────────────────────────────────────────
    handle.launch().await?;
    println!("Juego lanzado, esperando...");

    match handle.wait().await {
        Some(0) => println!("Juego cerrado correctamente."),
        Some(code) => eprintln!("Juego cerrado con código {code}."),
        None => eprintln!("El proceso no estaba corriendo."),
    }

    Ok(())
}

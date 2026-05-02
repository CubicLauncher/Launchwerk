use std::fs;

use launchwerk::VersionManifest;

fn main() {
    let bytes = fs::read("/home/santiagolxx/Schreibtisch/backup/programacao/cubiclauncher/launchwerk/tests/1_21_8_full.json").unwrap();

    let version: VersionManifest = serde_json::from_slice(&bytes).unwrap();
    println!("{:#?}", version)
}

use std::{fs, path::PathBuf};

use launchwerk::VersionManifest;

fn main() {
    let bytes = fs::read("/home/santiagolxx/Schreibtisch/backup/programacao/cubiclauncher/launchwerk/tests/1_21_8_full.json").unwrap();

    if let Some(version) = VersionManifest::from_bytes(&bytes) {
        let cp = version.get_classpath(PathBuf::from("/tmp/minecraft"));
        println!("{cp}")
    }
}

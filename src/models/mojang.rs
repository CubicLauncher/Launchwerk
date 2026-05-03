use crate::models::{MCVersion, deserialize_version};
use serde::Deserialize;
use std::env::consts::{ARCH, OS};

trait Evaluable {
    fn rules(&self) -> Option<&Vec<Rule>>;

    fn evaluate(&self) -> bool {
        match self.rules() {
            Some(rules) => rules.iter().all(|r| r.evaluate()),
            None => true,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Deserialize, Debug)]
struct OsRule {
    name: Option<String>,
    arch: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Rule {
    action: RuleAction,
    os: Option<OsRule>,
}

impl Rule {
    pub fn evaluate(&self) -> bool {
        let condition_matches = match &self.os {
            Some(os_rule) => {
                let os_matches = match os_rule.name.as_deref() {
                    Some("osx") => OS == "macos",
                    Some(name) => OS == name,
                    None => true,
                };
                let arch_matches = match os_rule.arch.as_deref() {
                    Some("x86") => ARCH == "x86" || ARCH == "x86_64",
                    Some(arch) => ARCH == arch,
                    None => true,
                };
                os_matches && arch_matches
            }
            None => false, // features false pq son boludeces, lo pondre luego
        };

        match self.action {
            RuleAction::Allow => condition_matches,
            RuleAction::Disallow => !condition_matches,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ArgumentValue {
    Single(String),
    Many(Vec<String>),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Argument {
    WithRule {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
    WithoutRule(String),
}

impl Argument {
    pub fn get_if_applies(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();
        match &self {
            Argument::WithRule { rules, value } => {
                if rules.iter().all(|r| r.evaluate()) {
                    match value {
                        ArgumentValue::Single(s) => args.push(s.clone()),
                        ArgumentValue::Many(s_vec) => {
                            args.extend(s_vec.iter().cloned());
                        }
                    }
                }
            }
            Argument::WithoutRule(s) => args.push(s.clone()),
        }
        args
    }
}

impl Evaluable for Argument {
    fn rules(&self) -> Option<&Vec<Rule>> {
        match self {
            Argument::WithRule { rules, .. } => Some(rules),
            Argument::WithoutRule(_) => None,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Library {
    name: String,
    rules: Option<Vec<Rule>>,
}

impl Evaluable for Library {
    fn rules(&self) -> Option<&Vec<Rule>> {
        self.rules.as_ref()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct JavaVersion {
    component: String,
    major_version: u8,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AssetIndex {
    id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VersionArgType {
    game: Option<Vec<Argument>>,
    jvm: Option<Vec<Argument>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifest {
    #[serde(deserialize_with = "deserialize_version")]
    id: MCVersion,
    main_class: String,
    arguments: VersionArgType,
    libraries: Vec<Library>,
    asset_index: AssetIndex,
    java_version: JavaVersion,
}

impl VersionManifest {
    pub fn from_bytes(bytes: &[u8]) -> Option<VersionManifest> {
        let version_m: Option<VersionManifest> = match serde_json::from_slice(bytes) {
            Ok(d) => Some(d),
            Err(_) => None,
        };
        version_m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_allow_linux() {
        let json = r#"{ "action": "allow", "os": { "name": "linux" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), true);
    }

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn test_allow_linux_arch_x86() {
        let json = r#"{ "action": "allow", "os": { "name": "linux", "arch": "x86" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), true);
    }

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    fn test_allow_linux_arch_aarch64() {
        let json = r#"{ "action": "allow", "os": { "name": "linux", "arch": "aarch" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), true);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_allow_windows() {
        let json = r#"{ "action": "allow", "os": { "name": "windows" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), true);
    }

    #[test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn test_allow_windows_arch_x86() {
        let json = r#"{ "action": "allow", "os": { "name": "windows", "arch": "x86" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), true);
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_disallow_linux() {
        let json = r#"{ "action": "disallow", "os": { "name": "linux" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), false);
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_disallow_windows() {
        let json = r#"{ "action": "disallow", "os": { "name": "windows"} }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), true);
    }

    #[test]
    #[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
    fn test_disallow_windows_with_arch() {
        let json = r#"{ "action": "disallow", "os": { "name": "windows", "arch": "x86" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.evaluate(), true);
    }

    #[test]
    fn test_parse_full_version_manifest() {
        let json = include_str!("../../tests/1_21_8_full.json");
        let manifest: VersionManifest = serde_json::from_str(json).unwrap();

        assert_eq!(manifest.main_class, "net.minecraft.client.main.Main");
        assert_eq!(manifest.java_version.major_version, 21);
        assert_eq!(manifest.asset_index.id, "26");
        assert!(manifest.libraries.len() > 0);
        assert!(manifest.arguments.jvm.is_some());
        assert!(manifest.arguments.game.is_some());
    }
}

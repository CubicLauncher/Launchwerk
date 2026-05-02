use serde::Deserialize;
use std::env::consts::{ARCH, OS};

#[derive(Deserialize)]
#[serde(untagged)]
enum ArgumentValue {
    Single(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
struct OsRule {
    name: Option<String>,
    arch: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Deserialize)]
struct Rule {
    action: RuleAction,
    os: Option<OsRule>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Argument {
    WithRule {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
    WithoutRule(String),
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
            None => true,
        };

        match self.action {
            RuleAction::Allow => condition_matches,
            RuleAction::Disallow => !condition_matches,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // test de linux
    //
    #[test]
    #[cfg(all(target_os = "linux"))]
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
    /// no tengo ningun dispositivo para testear aarch64 xd
    fn test_allow_linux_arch_aarch64() {
        let json = r#"{ "action": "allow", "os": { "name": "linux", "arch": "aarch" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();

        assert_eq!(rule.evaluate(), true);
    }
    #[test]
    #[cfg(all(target_os = "windows"))]
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
    #[cfg(all(not(target_os = "linux")))]
    fn test_disallow_linux() {
        let json = r#"{ "action": "disallow", "os": { "name": "linux" } }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();

        assert_eq!(rule.evaluate(), false);
    }
    #[test]
    #[cfg(all(not(target_os = "windows")))]
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
}

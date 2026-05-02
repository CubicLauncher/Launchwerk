use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
pub struct MCVersion {
    major: u8,
    minor: u8,
    patch: Option<u8>,
}

impl MCVersion {
    pub fn new(major: u8, minor: u8, patch: Option<u8>) -> MCVersion {
        MCVersion {
            major,
            minor,
            patch,
        }
    }
}

impl std::fmt::Display for MCVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            self.major,
            self.minor,
            self.patch.unwrap_or(0)
        )
    }
}

pub fn deserialize_version<'de, D>(deserializer: D) -> Result<MCVersion, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let parts: Vec<&str> = s.split('.').collect();

    Ok(MCVersion {
        major: parts[0].parse().map_err(serde::de::Error::custom)?,
        minor: parts[1].parse().map_err(serde::de::Error::custom)?,
        patch: parts.get(2).and_then(|p| p.parse().ok()),
    })
}

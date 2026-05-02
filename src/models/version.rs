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

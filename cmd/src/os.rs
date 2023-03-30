#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOS,
    Windows,
}

impl Os {
    pub fn current() -> Self {
        std::env::consts::OS.into()
    }
}

impl From<&str> for Os {
    fn from(s: &str) -> Self {
        match s {
            "linux" => Os::Linux,
            "macos" => Os::MacOS,
            "windows" => Os::Windows,
            _ => panic!("unknown os: {}", s),
        }
    }
}

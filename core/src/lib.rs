pub mod utils;

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OsVariant {
    Android,
    IOS,
}

impl OsVariant {
    /// Returns the string representation of the `OsVariant`.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Android => "Android",
            Self::IOS => "iOS",
        }
    }

    /// Returns the `User-Agent` string of the `OsVariant` in HTTP request.
    pub fn user_agent(&self) -> &str {
        match self {
            Self::Android => "UnityPlayer/2020.3.32f1 (UnityWebRequest/1.0, libcurl/7.80.0-DEV)",
            Self::IOS => "ProductName/5.2.000 CFNetwork/1333.0.4 Darwin/21.5.0",
        }
    }
}

impl std::fmt::Display for OsVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type of an entry in the manifest file.
///
/// The order of the members matters because it's deserialized from an array.
#[derive(Debug, serde::Deserialize)]
pub struct ManifestEntry {
    /// SHA1 hash of the file.
    pub hash: String,

    /// File name on the server.
    pub filename: String,

    /// File size.
    pub size: u64,
}

pub type Manifest = [std::collections::HashMap<String, ManifestEntry>; 1];

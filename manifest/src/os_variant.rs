use crate::ManifestError;
use clap::ValueEnum;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, ValueEnum)]
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

impl Display for OsVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for OsVariant {
    type Err = ManifestError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase() {
            s if s == "android" => Ok(Self::Android),
            s if s == "ios" => Ok(Self::IOS),
            _ => Err(ManifestError::UnknownVariant),
        }
    }
}

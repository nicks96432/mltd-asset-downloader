use clap::ValueEnum;
use std::fmt::Display;

#[derive(Debug, Clone, ValueEnum)]
pub enum ManifestFormat {
    Msgpack,

    #[cfg(feature = "json")]
    JSON,

    #[cfg(feature = "yaml")]
    YAML,
}

impl Display for ManifestFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Msgpack => write!(f, "msgpack"),
            Self::JSON => write!(f, "json"),
            Self::YAML => write!(f, "yaml"),
        }
    }
}

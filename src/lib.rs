pub mod utils;

use std::path::PathBuf;

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

    pub fn user_agent(&self) -> &str {
        match self {
            Self::Android => "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.5481.63 Mobile Safari/537.36",
            Self::IOS => "ProductName/5.2.000 CFNetwork/1333.0.4 Darwin/21.5.0",
        }
    }
}

impl std::fmt::Display for OsVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, clap::Parser)]
#[command(author, version, about, arg_required_else_help(true))]
pub struct Args {
    /// Keep the manifest file in the output directory
    #[arg(long, default_value_t = false)]
    pub keep_manifest: bool,

    /// The output path
    #[arg(short, long, value_name = "DIR", default_value_os_t = [".", "assets"].iter().collect())]
    pub output: PathBuf,

    /// The os variant to download
    #[arg(value_enum, value_name = "VARIANT")]
    pub os_variant: OsVariant,

    /// The number of threads to use
    #[arg(short = 'P', long, value_name = "CPUS", default_value_t = num_cpus::get())]
    pub parallel: usize,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ManifestEntry {
    pub hash: String,

    /// File name on the server.
    pub filename: String,

    /// File size.
    pub size: u64,
}

pub type Manifest = [std::collections::HashMap<String, ManifestEntry>; 1];

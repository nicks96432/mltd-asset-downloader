use std::path::PathBuf;

#[derive(Debug, clap::Args)]
#[command(author, version, about)]
pub struct ExtractorArgs {
    
    /// The output path
    #[arg(short, long, value_name = "DIR", default_value_os_t = [".", "output"].iter().collect())]
    output: PathBuf,

    /// The number of threads to use
    #[arg(short = 'P', long, value_name = "CPUS", default_value_t = num_cpus::get())]
    parallel: usize,
}

pub fn extractor(_args: &ExtractorArgs) {
    todo!()
}

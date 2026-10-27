use std::env::var;
use std::error::Error;

use vergen::{Cargo, Emitter, Rustc};
use vergen_gitcl::Gitcl;

fn main() -> Result<(), Box<dyn Error>> {
    let cargo = Cargo::all_cargo();
    let git2 = Gitcl::all_git();
    let rustc = Rustc::all_rustc();

    Emitter::default()
        .idempotent()
        .add_instructions(&cargo)?
        .add_instructions(&git2)?
        .add_instructions(&rustc)?
        .emit_and_set()?;

    let long_version = format!(
        "{} (enabled features: {}), built with rustc {}",
        var("VERGEN_GIT_DESCRIBE").unwrap_or("unknown version".to_string()),
        var("VERGEN_CARGO_FEATURES").unwrap_or("unknown features".to_string()),
        var("VERGEN_RUSTC_SEMVER").unwrap_or("unknown rustc version".to_string()),
    )
    .trim()
    .to_string();

    println!("cargo:rustc-env=MLTD_VERSION_LONG={}", long_version);

    Ok(())
}

use std::error::Error;

use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn Error>> {
    let git2 = Git2Builder::default().describe(true, true, None).build()?;
    Emitter::default().add_instructions(&git2)?.emit()?;

    Ok(())
}

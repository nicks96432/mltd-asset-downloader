use std::error::Error;
use std::process::exit;

pub fn error_exit(msg: &str, error: Option<&dyn Error>) -> ! {
    match error {
        Some(e) => log::error!("{}: {}", msg, e),
        None => log::error!("{}", msg),
    };

    exit(1);
}


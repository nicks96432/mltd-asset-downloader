use std::error::Error;
use std::process::exit;

pub fn error_exit(msg: &str, error: Option<&dyn Error>) -> ! {
    match error {
        Some(e) => log::error!("{}: {}", msg, e),
        None => log::error!("{}", msg),
    };

    exit(1);
}

use std::fs::metadata;
use std::io::ErrorKind;
use std::path::Path;

pub fn create_dir<P: AsRef<Path>>(path: P) {
    let e = match std::fs::create_dir(&path) {
        Ok(()) => return,
        Err(e) => e,
    };

    if e.kind() != ErrorKind::AlreadyExists {
        error_exit("unable to create output directory", Some(&e))
    }

    let is_dir = match metadata(&path) {
        Ok(m) => m.is_dir(),
        Err(e) => error_exit("cannot get matadata", Some(&e)),
    };

    if is_dir {
        return log::debug!("output directory already exists");
    }

    error_exit("output path is not a directory, exiting", None)
}

use env_logger::fmt::Formatter;
use log::Record;
use std::io::Result;
use std::io::Write;

pub fn log_formatter(buf: &mut Formatter, record: &Record) -> Result<()> {
    let color_code = match record.level() {
        log::Level::Error => 1, // red
        log::Level::Warn => 3,  // yellow
        log::Level::Info => 4,  // blue
        log::Level::Debug => 2, // green
        log::Level::Trace => 7, // white
    };
    let space = match record.level().as_str().len() {
        4 => " ",
        _ => "",
    };
    let level = record.level().as_str();
    let target = record.target();
    let timestamp = buf.timestamp();
    let body = record.args();

    writeln!(
        buf,
        "[\x1b[3{}m{}\x1b[0m]{} {} {} - {}",
        color_code, level, space, timestamp, target, body
    )
}

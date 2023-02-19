use crate::error_exit;
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

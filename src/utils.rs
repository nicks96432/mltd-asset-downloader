use std::io::Write;

pub fn error_exit(msg: &str, error: Option<&dyn std::error::Error>) -> ! {
    match error {
        Some(e) => log::error!("{}: {}", msg, e),
        None => log::error!("{}", msg),
    };

    std::process::exit(1);
}

pub fn trace_request(req: &ureq::Request) {
    let header_names = req.header_names();
    let iter = header_names.iter();

    log::trace!("{} {}", req.method(), req.url());

    iter.for_each(|h| log::trace!("{}: {}", h, req.header(h).unwrap_or("")));
}

pub fn trace_response(res: &ureq::Response) {
    let header_names = res.headers_names();
    let iter = header_names.iter();

    log::trace!(
        "{} {} {}",
        res.status(),
        res.status_text(),
        res.http_version()
    );

    iter.for_each(|h| log::trace!("{}: {}", h, res.header(h).unwrap_or("")));
}

pub fn create_dir<P: AsRef<std::path::Path>>(path: P) {
    let e = match std::fs::create_dir(&path) {
        Ok(()) => return,
        Err(e) => e,
    };

    if e.kind() != std::io::ErrorKind::AlreadyExists {
        error_exit("unable to create output directory", Some(&e))
    }

    let is_dir = match std::fs::metadata(&path) {
        Ok(m) => m.is_dir(),
        Err(e) => error_exit("cannot get matadata", Some(&e)),
    };

    if is_dir {
        return log::debug!("output directory already exists");
    }

    error_exit("output path is not a directory, exiting", None)
}

fn log_level_to_color(level: log::Level) -> i32 {
    match level {
        log::Level::Error => 1, // red
        log::Level::Warn => 3,  // yellow
        log::Level::Info => 4,  // blue
        log::Level::Debug => 2, // green
        log::Level::Trace => 7, // white
    }
}

pub fn init_logger(level: log::LevelFilter) {
    env_logger::Builder::new()
        .filter(Some("mltd_asset_downloader"), level)
        .format(|buf, record| {
            let color_code = log_level_to_color(record.level());
            let space = match record.level().as_str().len() {
                4 => " ",
                _ => "",
            };
            let level = record.level().as_str();
            let timestamp = buf.timestamp();
            let body = record.args();

            writeln!(
                buf,
                "[\x1b[3{}m{}\x1b[0m]{} {} - {}",
                color_code, level, space, timestamp, body
            )
        })
        .init()
}

#![cfg(feature = "request")]

use ureq::Error as UreqError;
use ureq::Error::Status;
use ureq::{Agent, Request, Response};

const ASSET_URL_BASE: &'static str = "https://td-assets.bn765.com";
const UNITY_VERSION: &'static str = "2020.3.32f1";

pub fn fetch_asset(agent: &Agent, path: &String) -> Result<Response, UreqError> {
    let url = format!("{}{}", ASSET_URL_BASE, path.as_str());
    let req = agent
        .get(url.as_str())
        .set("Accept", "*/*")
        .set("Accept-Encoding", "deflate, gzip")
        .set("X-Unity-Version", UNITY_VERSION);
    trace_request(&req);

    let result = req.call();
    if let Err(Status(_, ref res)) | Ok(ref res) = result {
        log::trace!("");
        trace_response(res);
    }

    result
}

pub fn trace_request(req: &Request) {
    let header_names = req.header_names();
    let iter = header_names.iter();

    log::trace!("{} {}", req.method(), req.url());

    iter.for_each(|h| log::trace!("{}: {}", h, req.header(h).unwrap_or("")));
}

pub fn trace_response(res: &Response) {
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

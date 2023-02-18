use ureq::{Agent, Error, Error::Status, Response};

const ASSET_URL_BASE: &'static str = "https://td-assets.bn765.com";
const UNITY_VERSION: &'static str = "2020.3.32f1";

fn trace_request(req: &ureq::Request) {
    let header_names = req.header_names();
    let iter = header_names.iter();

    log::trace!("{} {}", req.method(), req.url());

    iter.for_each(|h| log::trace!("{}: {}", h, req.header(h).unwrap_or("")));
}

fn trace_response(res: &ureq::Response) {
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

pub fn fetch_asset(agent: &Agent, path: &String) -> Result<Response, Error> {
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

pub fn get_version() -> Result<(String, u64), Box<dyn std::error::Error>> {
    let url = "https://api.matsurihi.me/api/mltd/v2/version/latest";
    let req = ureq::get(url).query("prettyPrint", "false");
    trace_request(&req);

    let res = req.call()?;
    log::trace!("");
    trace_response(&res);

    let json = res.into_json::<ureq::serde_json::Value>()?;

    let filename = match json["asset"]["indexName"].as_str() {
        Some(s) => s.to_owned(),
        None => {
            return Err(Box::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "cannot parse asset.indexName",
            )))
        }
    };
    let version = json["asset"]["version"].to_string().parse::<u64>()?;

    Ok((filename, version))
}

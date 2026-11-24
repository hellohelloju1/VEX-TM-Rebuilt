//! Bounded HTTP parsing and operator endpoint policy.
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::io::Read;
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
pub fn read(reader: &mut impl Read) -> Result<Request> {
    let started = std::time::Instant::now();
    let mut bytes = Vec::new();
    let end = loop {
        if started.elapsed() > std::time::Duration::from_secs(5) {
            bail!("Request timed out")
        }
        if bytes.len() > 16384 {
            bail!("Headers too large")
        };
        let mut buf = [0u8; 2048];
        let n = reader.read(&mut buf)?;
        if n == 0 {
            bail!("Incomplete request")
        };
        bytes.extend_from_slice(&buf[..n]);
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let text = std::str::from_utf8(&bytes[..end])?;
    let mut lines = text.split("\r\n");
    let mut first = lines.next().context("Request line")?.split_whitespace();
    let method = first.next().context("Method")?.to_owned();
    let path = first.next().context("Path")?.to_owned();
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once(':').context("Header")?;
        if headers
            .insert(k.to_ascii_lowercase(), v.trim().to_owned())
            .is_some()
        {
            bail!("Duplicate header")
        }
    }
    if headers.contains_key("transfer-encoding") {
        bail!("Chunked requests unsupported")
    };
    let size: usize = headers
        .get("content-length")
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(0);
    if size > 1_000_000 {
        bail!("Request too large")
    };
    let mut body = bytes[end..].to_vec();
    while body.len() < size {
        if started.elapsed() > std::time::Duration::from_secs(5) {
            bail!("Request timed out")
        }
        let mut buf = [0u8; 8192];
        let count = reader.read(&mut buf[..(size - body.len()).min(8192)])?;
        if count == 0 {
            bail!("Incomplete body")
        };
        body.extend_from_slice(&buf[..count]);
    }
    body.truncate(size);
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}
pub fn allowed_action(action: &str) -> bool {
    [
        "bootstrap",
        "snapshot",
        "send_to_fc",
        "field_setting",
        "field_bonus",
        "timer",
        "score",
        "submit",
        "reopen_match",
    ]
    .contains(&action)
}
pub fn valid_post(r: &Request) -> bool {
    r.method == "POST"
        && r.headers
            .get("content-type")
            .is_some_and(|v| v.split(';').next() == Some("application/json"))
        && r.headers.get("origin").is_none_or(|origin| {
            r.headers.get("host").is_some_and(|host| {
                origin == &format!("http://{host}") || origin == &format!("https://{host}")
            })
        })
}

pub fn authorized(password: Option<&str>, hash: &str) -> bool {
    !hash.is_empty() && password.is_some_and(|pw| crate::auth::verify(pw, hash))
}

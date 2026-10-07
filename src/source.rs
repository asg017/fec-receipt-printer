//! Turn what the user gave into an open filing: bytes already in memory (the
//! browser), or natively a path on disk, a URL, or a filing ID (`1926068` /
//! `FEC-1926068`) fetched from docquery.fec.gov.
//!
//! `Filing::from_reader` reads only the header and cover up front, so for a
//! URL only the start of the file is downloaded unless rows are read.

use std::io::{Cursor, Read};

use fec_parser::Filing;

pub type AnyFiling = Filing<Box<dyn Read + Send>>;

/// A whole `.fec` file already in memory, named like its file (for the ID).
pub fn from_bytes(bytes: Vec<u8>, name: &str) -> anyhow::Result<AnyFiling> {
    let len = bytes.len();
    Filing::from_reader(
        Box::new(Cursor::new(bytes)) as Box<dyn Read + Send>,
        stem(name),
        len,
    )
}

#[cfg(not(target_family = "wasm"))]
pub use native::open;

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::{fs::File, io::Read, path::Path};

    use anyhow::{Context, bail};
    use fec_parser::Filing;

    use super::{AnyFiling, stem};

    pub fn open(input: &str) -> anyhow::Result<AnyFiling> {
        let input = input.trim();
        let path = Path::new(input);
        if path.is_file() {
            let file = File::open(path).with_context(|| format!("opening {input}"))?;
            let len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
            return Filing::from_reader(Box::new(file) as Box<dyn Read + Send>, stem(input), len);
        }
        if input.starts_with("http://") || input.starts_with("https://") {
            return fetch(input);
        }
        let id = input.strip_prefix("FEC-").unwrap_or(input);
        if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) {
            return fetch(&format!("https://docquery.fec.gov/dcdev/posted/{id}.fec"));
        }
        bail!("{input:?} is not a file, a URL, or a filing ID like 1926068 / FEC-1926068")
    }

    fn fetch(url: &str) -> anyhow::Result<AnyFiling> {
        let response = ureq::get(url)
            .header(
                "User-Agent",
                concat!("fec-receipt/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .with_context(|| format!("fetching {url}"))?;
        let len = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok()?.parse().ok())
            .unwrap_or(0);
        let reader = response.into_body().into_reader();
        Filing::from_reader(Box::new(reader) as Box<dyn Read + Send>, stem(url), len)
            .with_context(|| format!("reading {url}"))
    }
}

/// `…/1926068.fec` → `1926068`; a test fixture's `F3XN_1926068.fec` → `1926068`.
fn stem(s: &str) -> String {
    let name = s.rsplit(['/', '\\']).next().unwrap_or(s);
    let stem = name.split('.').next().unwrap_or(name);
    match stem.rsplit_once('_') {
        Some((_, id)) if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) => id.to_owned(),
        _ => stem.to_owned(),
    }
}

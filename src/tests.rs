//! Every cover fixture from libfec's typed-covers branch, rendered both ways:
//! snapshot the text preview, and check nothing is wider than the paper.

use std::path::{Path, PathBuf};

use crate::{
    escpos, forms, preview,
    receipt::{Block, DOTS, Receipt},
    source,
};

fn fixtures() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(crate::DEFAULT_SAMPLES)
        .expect("libfec typed-covers fixtures")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "fec"))
        .collect();
    files.sort();
    files
}

fn render(path: &Path, full: bool) -> Receipt {
    let mut filing = source::open(path.to_str().unwrap()).unwrap();
    forms::render(&mut filing, forms::Opts { full })
}

fn assert_fits(name: &str, receipt: &Receipt) {
    for block in &receipt.blocks {
        match block {
            Block::Line(l) => {
                let text = l.text();
                assert!(
                    text.len() <= l.size.cols(),
                    "{name}: {text:?} is wider than {}",
                    l.size.cols()
                );
                assert!(
                    text.bytes().all(|b| (0x20..0x7f).contains(&b)),
                    "{name}: non-ASCII in {text:?}"
                );
            }
            Block::Raster(r) => assert_eq!(r.dots.len(), DOTS * r.height),
            Block::Feed(_) => {}
        }
    }
    assert!(
        receipt.warnings.is_empty(),
        "{name}: {:?}",
        receipt.warnings
    );
}

#[test]
fn fixtures_fit_and_snapshot() {
    for path in fixtures() {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        for full in [false, true] {
            let receipt = render(&path, full);
            assert_fits(&name, &receipt);
            let suffix = if full { "_full" } else { "" };
            insta::assert_snapshot!(format!("{name}{suffix}"), preview::text(&receipt, false));
        }
    }
}

#[test]
fn encodes_text_and_raster() {
    let receipt = render(&fixtures()[0], false);
    let bytes = escpos::encode(&receipt);
    assert!(bytes.starts_with(&[0x1b, b'@', 0x1b, b'3', escpos::LINE_DOTS]));
    // The QR goes out as a raster image, never the native QR command.
    assert!(bytes.windows(3).any(|w| w == [0x1d, b'v', b'0']));
    assert!(!bytes.windows(3).any(|w| w == [0x1d, b'(', b'k']));
}

/// Every filing in the local libfec cache (tens of thousands). Slow; run with
/// `cargo test -- --ignored`.
#[test]
#[ignore]
fn cached_filings_fit() {
    let dir = std::env::var("HOME").unwrap() + "/.cache/libfec/cache";
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let Ok(mut filing) = source::open(path.to_str().unwrap()) else {
            continue;
        };
        for full in [false, true] {
            let receipt = forms::render(&mut filing, forms::Opts { full });
            assert_fits(&path.display().to_string(), &receipt);
            // `full` needs the rows again for F24; reopen.
            filing = match source::open(path.to_str().unwrap()) {
                Ok(f) => f,
                Err(_) => break,
            };
        }
        checked += 1;
    }
    eprintln!("checked {checked} filings");
}

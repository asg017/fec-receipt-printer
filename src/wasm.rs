//! The browser build: a `.fec` file's bytes in, the receipt preview and the
//! printer's bytes out. The page (`web/index.html`) sends those bytes to the
//! printer itself, over WebUSB.

use wasm_bindgen::prelude::*;

use crate::{escpos, forms, preview, source};

#[wasm_bindgen]
pub struct Rendered {
    html: String,
    escpos: Vec<u8>,
    warnings: Vec<String>,
    length_mm: f64,
}

#[wasm_bindgen]
impl Rendered {
    /// The paper strip; style it with [`paper_css`].
    #[wasm_bindgen(getter)]
    pub fn html(&self) -> String {
        self.html.clone()
    }

    /// ESC/POS bytes to send to the printer.
    #[wasm_bindgen(getter)]
    pub fn escpos(&self) -> Vec<u8> {
        self.escpos.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> Vec<String> {
        self.warnings.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn length_mm(&self) -> f64 {
        self.length_mm
    }
}

/// Render a filing's receipt. `name` is the file name, for the filing ID.
#[wasm_bindgen]
pub fn render(bytes: Vec<u8>, name: &str, full: bool) -> Result<Rendered, JsError> {
    let mut filing =
        source::from_bytes(bytes, name).map_err(|e| JsError::new(&format!("{e:#}")))?;
    let receipt = forms::render(&mut filing, forms::Opts { full });
    Ok(Rendered {
        html: preview::html(&receipt),
        escpos: escpos::encode(&receipt),
        length_mm: preview::length_mm(&receipt),
        warnings: receipt.warnings,
    })
}

#[wasm_bindgen]
pub fn paper_css() -> String {
    preview::PAPER_CSS.to_owned()
}

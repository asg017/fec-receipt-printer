//! Print FEC filing covers on a 58mm thermal receipt printer.
//!
//! Everything from a filing's bytes to the printer's bytes is portable and
//! also builds for the browser (see [`wasm`]); fetching, USB and the debug
//! server are native only.

pub mod escpos;
pub mod forms;
pub mod preview;
pub mod receipt;
pub mod source;

#[cfg(not(target_family = "wasm"))]
pub mod serve;
#[cfg(not(target_family = "wasm"))]
pub mod usb;
#[cfg(target_family = "wasm")]
pub mod wasm;

#[cfg(test)]
mod tests;

/// Cover fixtures for every form, copied from libfec's typed-covers branch.
pub const DEFAULT_SAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/covers");

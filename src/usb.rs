//! Raw USB to the POS58. It is not a system printer (no CUPS queue), so the
//! bytes go straight to its bulk OUT endpoint — 0x03, not the 0x01 most
//! libraries default to, which silently prints nothing.

use std::time::Duration;

use anyhow::{Context, bail};

const VENDOR_ID: u16 = 0x0416;
const PRODUCT_ID: u16 = 0x5011;
const OUT_EP: u8 = 0x03;
const INTERFACE: u8 = 0;
const CHUNK: usize = 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

/// Whether the printer is plugged in (doesn't open it).
pub fn connected() -> bool {
    rusb::devices().is_ok_and(|list| {
        list.iter().any(|d| {
            d.device_descriptor()
                .is_ok_and(|desc| desc.vendor_id() == VENDOR_ID && desc.product_id() == PRODUCT_ID)
        })
    })
}

pub fn print(bytes: &[u8]) -> anyhow::Result<()> {
    let Some(handle) = rusb::open_device_with_vid_pid(VENDOR_ID, PRODUCT_ID) else {
        bail!("printer {VENDOR_ID:04x}:{PRODUCT_ID:04x} not found — is it plugged in and on?");
    };
    // Linux binds a kernel driver (usblp); macOS doesn't, and errors here.
    let _ = handle.set_auto_detach_kernel_driver(true);
    handle
        .claim_interface(INTERFACE)
        .context("claiming printer USB interface")?;
    let result = bytes.chunks(CHUNK).try_for_each(|chunk| {
        let mut sent = 0;
        while sent < chunk.len() {
            sent += handle
                .write_bulk(OUT_EP, &chunk[sent..], TIMEOUT)
                .context("writing to printer")?;
        }
        anyhow::Ok(())
    });
    let _ = handle.release_interface(INTERFACE);
    result
}

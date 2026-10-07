//! Receipt → ESC/POS bytes for the POS58.
//!
//! Only the handful of commands this printer is known to handle: init, line
//! spacing, bold, character size, raster images (`GS v 0` — not column mode,
//! which shears on this printer) and feed. No native QR (it prints the
//! command bytes as garbage) and no cut (tear-off only).

use crate::receipt::{Block, DOTS, Raster, Receipt, Size};

const ESC: u8 = 0x1b;
const GS: u8 = 0x1d;
const LF: u8 = 0x0a;

/// Dots between text baselines, set explicitly so the preview can match.
pub const LINE_DOTS: u8 = 30;

/// Rows per `GS v 0` command; small bands keep the printer's buffer happy.
const BAND_ROWS: usize = 128;

pub fn encode(receipt: &Receipt) -> Vec<u8> {
    let mut out = vec![ESC, b'@', ESC, b'3', LINE_DOTS];
    for block in &receipt.blocks {
        match block {
            Block::Line(line) => {
                let size = match line.size {
                    Size::Normal => 0x00,
                    Size::Tall => 0x01,
                    Size::Big => 0x11,
                };
                out.extend([GS, b'!', size]);
                for span in &line.spans {
                    out.extend([ESC, b'E', span.bold as u8]);
                    out.extend(span.text.bytes());
                }
                out.extend([ESC, b'E', 0, GS, b'!', 0, LF]);
            }
            Block::Raster(r) => raster(&mut out, r),
            Block::Feed(n) => out.extend([ESC, b'd', *n]),
        }
    }
    out
}

fn raster(out: &mut Vec<u8>, r: &Raster) {
    const ROW_BYTES: usize = DOTS / 8;
    for top in (0..r.height).step_by(BAND_ROWS) {
        let rows = BAND_ROWS.min(r.height - top);
        out.extend([
            GS,
            b'v',
            b'0',
            0,
            ROW_BYTES as u8,
            0,
            rows as u8,
            (rows >> 8) as u8,
        ]);
        for y in top..top + rows {
            for byte in 0..ROW_BYTES {
                let mut b = 0u8;
                for bit in 0..8 {
                    if r.get(byte * 8 + bit, y) {
                        b |= 0x80 >> bit;
                    }
                }
                out.push(b);
            }
        }
    }
}

/// `xxd`-style dump, 16 bytes a line, for the debug viewer.
pub fn hexdump(bytes: &[u8]) -> String {
    let mut s = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let text: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        s.push_str(&format!("{:06x}  {:<48} {text}\n", i * 16, hex.join(" ")));
    }
    s
}

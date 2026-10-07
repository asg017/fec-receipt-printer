//! Receipt → something to look at without printing: plain text for the
//! terminal and snapshot tests, and a true-scale HTML paper strip for the
//! debug viewer.

use crate::{
    escpos::LINE_DOTS,
    receipt::{Block, COLS, DOTS, Line, Receipt, Size},
};

/// Paper used, in mm (8 dots per mm at 203 dpi). An estimate: the printer
/// decides exactly how far a tall line advances.
pub fn length_mm(receipt: &Receipt) -> f64 {
    let dots: usize = receipt
        .blocks
        .iter()
        .map(|b| match b {
            Block::Line(l) if l.size == Size::Normal => LINE_DOTS as usize,
            Block::Line(_) => 2 * LINE_DOTS as usize,
            Block::Raster(r) => r.height,
            Block::Feed(n) => *n as usize * LINE_DOTS as usize,
        })
        .sum();
    dots as f64 / 8.0
}

/// The receipt as text inside a 32-column frame. Big text is spread out to
/// show its double width; tall lines are marked in the gutter; images are a
/// placeholder as tall as they'd print. `ansi` adds bold.
pub fn text(receipt: &Receipt, ansi: bool) -> String {
    let mut out = format!("  +{}+\n", "-".repeat(COLS));
    // `width` is the visible width of `body`, which may hold ANSI codes.
    let mut row = |gutter: &str, body: &str, width: usize| {
        let pad = " ".repeat(COLS.saturating_sub(width));
        out.push_str(&format!("{gutter:<2}|{body}{pad}|\n"));
    };
    for block in &receipt.blocks {
        match block {
            Block::Line(line) => {
                let (gutter, plain, styled) = line_text(line, ansi);
                row(gutter, &styled, plain.len());
            }
            Block::Raster(r) => {
                let rows = r.height.div_ceil(LINE_DOTS as usize);
                for i in 0..rows {
                    let label = if i == rows / 2 {
                        format!("[ image {}x{} ]", DOTS, r.height)
                    } else {
                        String::new()
                    };
                    row("", &format!("{label:^COLS$}"), COLS);
                }
            }
            Block::Feed(n) => (0..*n).for_each(|_| row("", "", 0)),
        }
    }
    out.push_str(&format!("  +{}+\n", "-".repeat(COLS)));
    if !receipt.warnings.is_empty() {
        out.push_str("\nwarnings:\n");
        for w in &receipt.warnings {
            out.push_str(&format!("  - {w}\n"));
        }
    }
    out
}

/// A line's gutter mark, plain text, and text with ANSI bold.
fn line_text(line: &Line, ansi: bool) -> (&'static str, String, String) {
    let (mut plain, mut styled) = (String::new(), String::new());
    for span in &line.spans {
        let text: String = match line.size {
            Size::Big => span.text.chars().flat_map(|c| [c, ' ']).collect(),
            _ => span.text.clone(),
        };
        plain.push_str(&text);
        if span.bold && ansi {
            styled.push_str(&format!("\x1b[1m{text}\x1b[0m"));
        } else {
            styled.push_str(&text);
        }
    }
    let gutter = match line.size {
        Size::Normal => "",
        Size::Tall | Size::Big => "2x",
    };
    (gutter, plain, styled)
}

/// The paper strip as HTML: a 384px-wide div (1 CSS px = 1 printer dot),
/// text on a 12px-per-character grid, images as SVG of the exact dots sent.
/// Styled by the `.paper` rules in [`PAPER_CSS`].
pub fn html(receipt: &Receipt) -> String {
    let mut out = String::from(r#"<div class="paper">"#);
    for block in &receipt.blocks {
        match block {
            Block::Line(line) => {
                let class = match line.size {
                    Size::Normal => "n",
                    Size::Tall => "t",
                    Size::Big => "b",
                };
                let over = if line.text().len() > line.size.cols() {
                    " over"
                } else {
                    ""
                };
                out.push_str(&format!(r#"<div class="l {class}{over}"><span>"#));
                for span in &line.spans {
                    let text = escape(&span.text);
                    if span.bold {
                        out.push_str(&format!("<b>{text}</b>"));
                    } else {
                        out.push_str(&text);
                    }
                }
                out.push_str("</span></div>");
            }
            Block::Raster(r) => {
                let mut path = String::new();
                for y in 0..r.height {
                    let mut x = 0;
                    while x < DOTS {
                        if r.get(x, y) {
                            let start = x;
                            while x < DOTS && r.get(x, y) {
                                x += 1;
                            }
                            path.push_str(&format!("M{start} {y}h{}v1h-{}z", x - start, x - start));
                        } else {
                            x += 1;
                        }
                    }
                }
                out.push_str(&format!(
                    r#"<svg class="img" width="{DOTS}" height="{h}" viewBox="0 0 {DOTS} {h}" shape-rendering="crispEdges"><path d="{path}"/></svg>"#,
                    h = r.height
                ));
            }
            Block::Feed(n) => {
                out.push_str(&format!(
                    r#"<div class="feed" style="height:{}px"></div>"#,
                    *n as usize * LINE_DOTS as usize
                ));
            }
        }
    }
    out.push_str("</div>");
    out
}

/// Styles for [`html`]'s paper strip. Every character gets a 12px cell
/// whatever the font (`letter-spacing` makes up the difference from `1ch`),
/// matching the printer's 12x24 font, so columns line up as on paper.
pub const PAPER_CSS: &str = r#"
.paper { width: 384px; box-sizing: content-box; padding: 24px 40px 0; /* 58mm paper, 48mm printable */ background: #fbfaf6; color: #1b1b1b;
  font: 20px/30px Menlo, ui-monospace, "SF Mono", monospace; position: relative;
  box-shadow: 0 1px 2px rgba(0,0,0,.25), 0 8px 24px rgba(0,0,0,.18);
  -webkit-mask: linear-gradient(#000 0 0) top/100% calc(100% - 8px) no-repeat,
    conic-gradient(from -45deg at bottom, #0000, #000 1deg 89deg, #0000 90deg) bottom/12px 8px repeat-x; }
.paper .l { height: 30px; white-space: pre; overflow: visible; }
.paper .l span { display: inline-block; letter-spacing: calc(12px - 1ch); transform-origin: 0 0; }
.paper .l b { font-weight: 800; }
.paper .t { height: 60px; }
.paper .t span { transform: scale(1, 2); }
.paper .b { height: 60px; }
.paper .b span { transform: scale(2, 2); }
.paper .img { display: block; fill: #1b1b1b; }
.paper .over { background: rgba(220, 38, 38, .25); }
.grid .paper::after { content: ""; position: absolute; inset: 0 40px; pointer-events: none;
  background: repeating-linear-gradient(90deg, rgba(37,99,235,.18) 0 1px, transparent 1px 12px); }
"#;

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

//! The receipt: a list of already-laid-out blocks that both the printer
//! encoder ([`crate::escpos`]) and the previews ([`crate::preview`]) read.
//!
//! Everything about fitting the paper happens here, once. A [`Line`] is at
//! most [`Size::cols`] characters of printable ASCII, already aligned with
//! spaces, and a [`Raster`] is exactly [`DOTS`] wide. The encoder and previews
//! never wrap, pad or measure anything themselves, so what the debug viewer
//! shows is what the paper gets.

/// Printable width of 58mm paper at 203 dpi.
pub const DOTS: usize = 384;
/// Characters per line in the printer's default 12x24 font.
pub const COLS: usize = DOTS / 12;

/// Blank lines fed after the last block so it clears the tear bar.
const TEAR_FEED: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    #[default]
    Normal,
    /// Twice as tall, same width: still 32 columns.
    Tall,
    /// Twice as tall and twice as wide: 16 columns.
    Big,
}

impl Size {
    pub fn cols(self) -> usize {
        match self {
            Size::Normal | Size::Tall => COLS,
            Size::Big => COLS / 2,
        }
    }
}

/// A run of text within a line, bold or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub size: Size,
    pub spans: Vec<Span>,
}

impl Line {
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

/// A 1-bit image, [`DOTS`] wide; `true` is a burned (black) dot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    pub height: usize,
    /// Row-major, `DOTS * height` dots.
    pub dots: Vec<bool>,
}

impl Raster {
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.dots[y * DOTS + x]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Line(Line),
    Raster(Raster),
    /// Blank lines of paper.
    Feed(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
}

#[derive(Debug, Default)]
pub struct Receipt {
    pub blocks: Vec<Block>,
    /// Anything the builder had to cut to make it fit, for the previews to flag.
    pub warnings: Vec<String>,
}

impl Receipt {
    pub fn new() -> Self {
        Self::default()
    }

    /// The finished receipt, with the tear-off feed appended.
    pub fn finish(mut self) -> Self {
        self.blocks.push(Block::Feed(TEAR_FEED));
        self
    }

    /// Number of text lines (tall lines count twice), a rough paper length.
    pub fn line_count(&self) -> usize {
        self.blocks
            .iter()
            .map(|b| match b {
                Block::Line(l) if l.size == Size::Normal => 1,
                Block::Line(_) => 2,
                Block::Raster(r) => r.height.div_ceil(24),
                Block::Feed(n) => *n as usize,
            })
            .sum()
    }

    fn push(&mut self, size: Size, spans: Vec<Span>) {
        let width: usize = spans.iter().map(|s| s.text.len()).sum();
        debug_assert!(width <= size.cols(), "line too wide: {spans:?}");
        self.blocks.push(Block::Line(Line { size, spans }));
    }

    fn wrapped(&mut self, text: &str, size: Size, bold: bool, align: Align) {
        for line in wrap(&ascii(text), size.cols()) {
            let line = match align {
                Align::Left => line,
                Align::Center => center(&line, size.cols()),
            };
            self.push(size, vec![span(line, bold)]);
        }
    }

    /// Plain text, wrapped.
    pub fn text(&mut self, text: &str) {
        self.wrapped(text, Size::Normal, false, Align::Left);
    }

    pub fn bold(&mut self, text: &str) {
        self.wrapped(text, Size::Normal, true, Align::Left);
    }

    pub fn center(&mut self, text: &str, bold: bool) {
        self.wrapped(text, Size::Normal, bold, Align::Center);
    }

    /// A big centered headline, 16 columns.
    pub fn big(&mut self, text: &str) {
        self.wrapped(text, Size::Big, true, Align::Center);
    }

    /// A tall bold line at full width, for names.
    pub fn tall(&mut self, text: &str) {
        self.wrapped(text, Size::Tall, true, Align::Left);
    }

    pub fn rule(&mut self, ch: char) {
        self.push(Size::Normal, vec![span(ch.to_string().repeat(COLS), false)]);
    }

    pub fn feed(&mut self, lines: u8) {
        self.blocks.push(Block::Feed(lines));
    }

    /// `Label   value`, the value wrapped under itself with a hanging indent.
    pub fn kv(&mut self, label: &str, value: &str) {
        const LABEL: usize = 8;
        let value = ascii(value);
        if value.is_empty() {
            return;
        }
        let label = fit(&ascii(label), LABEL - 1, &mut self.warnings);
        for (i, line) in wrap(&value, COLS - LABEL).into_iter().enumerate() {
            let head = if i == 0 { label.as_str() } else { "" };
            self.push(
                Size::Normal,
                vec![span(format!("{head:<LABEL$}"), false), span(line, false)],
            );
        }
    }

    /// `kv` for an optional value.
    pub fn kv_opt(&mut self, label: &str, value: Option<impl AsRef<str>>) {
        if let Some(v) = value {
            self.kv(label, v.as_ref());
        }
    }

    /// A label and one right-aligned amount.
    pub fn money(&mut self, label: &str, amount: f64, bold: bool) {
        self.amounts(label, &[amount], bold);
    }

    /// A label and two right-aligned amounts (Column A, Column B).
    pub fn money2(&mut self, label: &str, a: f64, b: f64, bold: bool) {
        self.amounts(label, &[a, b], bold);
    }

    /// Column headings over `money2` rows.
    pub fn money2_heading(&mut self, a: &str, b: &str) {
        let line = format!("{:>w$}{:>AMOUNT$}", a, b, w = COLS - AMOUNT);
        self.push(Size::Normal, vec![span(line, true)]);
    }

    /// The label takes whatever the amounts leave. A label that doesn't fit
    /// gets full-width lines of its own, with the amounts on the line below;
    /// two-amount rows always do, since barely any label fits beside them.
    fn amounts(&mut self, label: &str, amounts: &[f64], bold: bool) {
        // A lone amount has room to print in full; pairs shorten to fit.
        let cells: String = match amounts {
            [a] => format!("{:>AMOUNT$}", format!(" {}", money_in(*a, COLS / 2))),
            _ => amounts
                .iter()
                .map(|&a| format!("{:>AMOUNT$}", money(a)))
                .collect(),
        };
        let label = ascii(label);
        let room = COLS - cells.len() - 1;
        let last = if amounts.len() == 1 && label.len() <= room {
            label
        } else {
            for line in wrap(&label, COLS) {
                self.push(Size::Normal, vec![span(line, bold)]);
            }
            String::new()
        };
        let pad = COLS - last.len() - cells.len();
        self.push(
            Size::Normal,
            vec![
                span(format!("{last}{}", " ".repeat(pad)), bold),
                span(cells, bold),
            ],
        );
    }

    /// A QR code, centered.
    pub fn qr(&mut self, data: &str) {
        match qr_raster(data) {
            Some(r) => self.blocks.push(Block::Raster(r)),
            None => self
                .warnings
                .push(format!("QR too large for paper: {data}")),
        }
    }
}

/// Width of one amount cell, including its leading space.
const AMOUNT: usize = 11;

fn span(text: String, bold: bool) -> Span {
    Span { text, bold }
}

/// Fold to printable ASCII: `é` → `e`, `—` → `-`, anything else unprintable dropped.
pub fn ascii(s: &str) -> String {
    let folded = deunicode::deunicode(s);
    let mut out = String::with_capacity(folded.len());
    for ch in folded.chars() {
        match ch {
            ' '..='~' => out.push(ch),
            '\t' | '\n' | '\r' => out.push(' '),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Greedy word wrap to `width`, breaking words longer than a line. Always at
/// least one (possibly empty) line.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        let mut word = word;
        while word.len() > width {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let (head, tail) = word.split_at(width);
            lines.push(head.to_owned());
            word = tail;
        }
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.len() + 1 + word.len() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::replace(&mut cur, word.to_owned()));
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

fn center(s: &str, width: usize) -> String {
    let pad = width.saturating_sub(s.len()) / 2;
    format!("{}{s}", " ".repeat(pad))
}

fn fit(s: &str, width: usize, warnings: &mut Vec<String>) -> String {
    if s.len() <= width {
        s.to_owned()
    } else {
        warnings.push(format!("cut {s:?} to {width} columns"));
        s[..width].to_owned()
    }
}

/// Whole dollars with commas (`1,234,567`); amounts too wide for a
/// 10-character cell shorten to `123.4M` / `1.23B`.
pub fn money(amount: f64) -> String {
    money_in(amount, AMOUNT - 1)
}

/// [`money`], shortened only past `width` characters.
fn money_in(amount: f64, width: usize) -> String {
    let rounded = amount.round();
    let n = rounded.abs() as u64;
    let digits = n.to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let sign = if rounded < 0.0 { "-" } else { "" };
    let full = format!("{sign}{grouped}");
    if full.len() <= width {
        return full;
    }
    let abs = amount.abs();
    if abs >= 1e9 {
        format!("{sign}{:.2}B", abs / 1e9)
    } else {
        format!("{sign}{:.1}M", abs / 1e6)
    }
}

/// Render `data` as a QR code, 4 dots per module plus a quiet zone, centered
/// in a full-width raster. The printer's own QR command doesn't work, so QR
/// codes always go to paper as images.
fn qr_raster(data: &str) -> Option<Raster> {
    const SCALE: usize = 4;
    const QUIET: usize = 2;
    let code = qrcode::QrCode::with_error_correction_level(data, qrcode::EcLevel::M).ok()?;
    let n = code.width();
    let modules = code.to_colors();
    let side = (n + 2 * QUIET) * SCALE;
    if side > DOTS {
        return None;
    }
    let left = (DOTS - side) / 2;
    let mut dots = vec![false; DOTS * side];
    for my in 0..n {
        for mx in 0..n {
            if modules[my * n + mx] != qrcode::Color::Dark {
                continue;
            }
            for dy in 0..SCALE {
                let y = (my + QUIET) * SCALE + dy;
                let x0 = left + (mx + QUIET) * SCALE;
                dots[y * DOTS + x0..y * DOTS + x0 + SCALE].fill(true);
            }
        }
    }
    Some(Raster { height: side, dots })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_formats() {
        assert_eq!(money(0.0), "0");
        assert_eq!(money(1234.5), "1,235");
        assert_eq!(money(-98_765.0), "-98,765");
        assert_eq!(money(99_999_999.0), "99,999,999");
        assert_eq!(money(123_456_789.0), "123.5M");
        assert_eq!(money(2_345_678_901.0), "2.35B");
    }

    #[test]
    fn wrap_breaks_long_words() {
        assert_eq!(wrap("ab cdefgh i", 4), ["ab", "cdef", "gh i"]);
        assert_eq!(wrap("", 4), [""]);
    }

    #[test]
    fn ascii_folds() {
        assert_eq!(ascii("José  Núñez — PAC\t"), "Jose Nunez -- PAC");
    }

    #[test]
    fn money_rows_fit() {
        let mut r = Receipt::new();
        r.money(
            "A very long line label that must wrap somewhere",
            123_456_789.0,
            false,
        );
        r.money2("Receipts", 99_999_999.0, -99_999_999.0, true);
        for b in &r.blocks {
            if let Block::Line(l) = b {
                assert!(l.text().len() <= COLS, "{:?}", l.text());
            }
        }
    }
}

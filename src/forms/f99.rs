//! Form 99: miscellaneous text — a letter to the FEC.

use fec_parser::covers::Form99;

use super::*;

/// Lines of message text printed by default; `--full` prints it all.
const TEXT_LINES: usize = 24;

pub fn render(r: &mut Receipt, filing: &AnyFiling, f: &Form99, opts: Opts) {
    banner(r, filing);
    filer(r, &f.committee_name, &f.filer_committee_id, &f.address);
    r.rule('-');
    r.kv_opt("Subject", f.text_code_label().or(f.text_code.as_deref()));
    if f.pdf_attachment {
        r.text("PDF attached (on fec.gov)");
    }

    if let Some(text) = f.text.as_deref().filter(|t| !t.trim().is_empty()) {
        heading(r, "Message");
        let mut lines = Vec::new();
        for para in text.lines() {
            lines.extend(crate::receipt::wrap(
                &crate::receipt::ascii(para),
                crate::receipt::COLS,
            ));
        }
        // Collapse runs of blank lines: paper is precious.
        lines.dedup_by(|a, b| a.is_empty() && b.is_empty());
        // Cutting a line or two to print a "more" notice saves nothing.
        let shown = if opts.full || lines.len() <= TEXT_LINES + 2 {
            lines.len()
        } else {
            TEXT_LINES
        };
        for line in &lines[..shown] {
            r.text(line);
        }
        if shown < lines.len() {
            r.center(
                &format!("... {} more lines (--full)", lines.len() - shown),
                false,
            );
        }
    }

    footer(r, filing, Some(&f.treasurer), f.date_signed);
}

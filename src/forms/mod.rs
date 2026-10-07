//! Cover → receipt. One module per form we lay out specially; every other
//! form gets the shared header/filer/footer from [`generic`].
//!
//! Every receipt has the same frame: a banner (form, report, amendment), who
//! filed, form-specific body, then who signed it and a QR code to the filing
//! on fec.gov. The default body is the headline numbers only; `full` adds the
//! detailed summary (non-zero lines only, to save paper).

mod f24;
mod f3;
mod f3p;
mod f3x;
mod f99;
mod generic;

use fec_parser::covers::{Cover, PersonName};
use jiff::civil::Date;

use crate::{receipt::Receipt, source::AnyFiling};

#[derive(Debug, Clone, Copy, Default)]
pub struct Opts {
    /// Add the detailed summary page / every itemization / the whole text.
    pub full: bool,
}

pub fn render(filing: &mut AnyFiling, opts: Opts) -> Receipt {
    let mut r = Receipt::new();
    match filing.cover.cover_data.clone() {
        Some(Cover::Form3X(f)) => f3x::render(&mut r, filing, &f, opts),
        Some(Cover::Form3(f)) => f3::render(&mut r, filing, &f, opts),
        Some(Cover::Form3P(f)) => f3p::render(&mut r, filing, &f, opts),
        Some(Cover::Form24(f)) => f24::render(&mut r, filing, &f, opts),
        Some(Cover::Form99(f)) => f99::render(&mut r, filing, &f, opts),
        other => generic::render(&mut r, filing, other.as_ref()),
    }
    r.finish()
}

/// What the form is, in a few words that fit under the big form number.
pub fn form_title(form_type: &str) -> &'static str {
    match fec_parser::covers::base_form_type(form_type).as_str() {
        "F1" => "Statement of Organization",
        "F1M" => "Multicandidate Status",
        "F2" => "Statement of Candidacy",
        "F3" => "Congressional Campaign Report",
        "F3L" => "Lobbyist Bundling Report",
        "F3P" => "Presidential Campaign Report",
        "F3X" => "PAC / Party Report",
        "F4" => "Convention Committee Report",
        "F5" => "Independent Expenditures",
        "F6" => "48-Hour Contribution Notice",
        "F7" => "Communication Costs",
        "F9" => "Electioneering Communications",
        "F13" => "Inaugural Committee Donations",
        "F24" => "Independent Expenditures",
        "F99" => "Miscellaneous Text",
        _ => "FEC Filing",
    }
}

/// The top of every receipt:
///
/// ```text
/// ================================
///             FORM 3X
///        PAC / Party Report
///           FEC-1926068
///       ** AMENDMENT **
/// ================================
/// ```
pub fn banner(r: &mut Receipt, filing: &AnyFiling) {
    let form_type = filing.cover.form_type.as_str();
    let base = fec_parser::covers::base_form_type(form_type);
    r.rule('=');
    r.big(&format!("FORM {}", base.trim_start_matches('F')));
    r.center(form_title(form_type), false);
    r.center(&format!("FEC-{}", filing.filing_id), false);
    if fec_parser::covers::is_amendment_form_type(form_type) {
        r.center("** AMENDMENT **", true);
    }
    if form_type.ends_with(['T', 't']) {
        r.center("** TERMINATION **", true);
    }
    r.rule('=');
}

/// Name (tall), ID, and city/state.
pub fn filer(r: &mut Receipt, name: &str, id: &str, address: &fec_parser::covers::Address) {
    r.tall(name);
    let place = [address.city.as_deref(), address.state.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if place.is_empty() {
        r.text(id);
    } else {
        r.text(&format!("{id} - {place}"));
    }
}

/// Report type, covering period and election, as `kv` lines.
pub fn report_lines(
    r: &mut Receipt,
    report: Option<String>,
    from: Option<Date>,
    through: Option<Date>,
    election: Option<String>,
) {
    r.kv_opt("Report", report);
    r.kv_opt("Period", period(from, through));
    r.kv_opt("Elect", election);
}

/// `Q3 - October Quarterly`.
pub fn report_text(code: Option<&str>, label: Option<&str>) -> Option<String> {
    match (code, label) {
        (Some(code), Some(label)) => Some(format!("{code} - {label}")),
        (Some(code), None) => Some(code.to_owned()),
        _ => None,
    }
}

/// `General 2026, Nov 3, 2026, TX`.
pub fn election_text(
    code: Option<&str>,
    label: Option<&str>,
    date: Option<Date>,
    state: Option<&str>,
) -> Option<String> {
    let year = code
        .map(|c| c.trim_start_matches(|ch: char| ch.is_ascii_alphabetic()))
        // The date says the year already.
        .filter(|y| date.is_none_or(|d| d.year().to_string() != *y));
    let what = match (label, year) {
        (Some(l), Some(y)) if !y.is_empty() => Some(format!("{l} {y}")),
        (Some(l), _) => Some(l.to_owned()),
        (None, _) => code.map(str::to_owned),
    };
    let parts: Vec<String> = [what, date.map(date_text), state.map(str::to_owned)]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// `Jul 1 - Sep 30, 2025`, or with both years when they differ.
pub fn period(from: Option<Date>, through: Option<Date>) -> Option<String> {
    match (from, through) {
        (Some(a), Some(b)) if a.year() == b.year() => {
            Some(format!("{} - {}", a.strftime("%b %-d"), date_text(b)))
        }
        (Some(a), Some(b)) => Some(format!("{} - {}", date_text(a), date_text(b))),
        (Some(a), None) => Some(format!("from {}", date_text(a))),
        (None, Some(b)) => Some(format!("through {}", date_text(b))),
        (None, None) => None,
    }
}

/// `Oct 1, 2025`.
pub fn date_text(d: Date) -> String {
    d.strftime("%b %-d, %Y").to_string()
}

/// The bottom of every receipt: who signed, when, the software, and a QR code
/// to the filing's page on docquery.fec.gov.
pub fn footer(
    r: &mut Receipt,
    filing: &AnyFiling,
    signer: Option<&PersonName>,
    signed: Option<Date>,
) {
    r.rule('-');
    let signer = signer
        .map(|p| p.to_string())
        .filter(|s| !s.trim().is_empty());
    match (signer, signed) {
        (Some(name), date) => {
            r.kv("Signed", &name);
            r.kv_opt("", date.map(date_text));
        }
        (None, Some(date)) => r.kv("Signed", &date_text(date)),
        (None, None) => {}
    }
    if let Some(url) = fec_url(filing) {
        r.feed(1);
        r.qr(&url);
        r.center("scan for the filing on fec.gov", false);
    }
}

/// The filing's page on docquery.fec.gov, for filings with a numeric ID.
pub fn fec_url(filing: &AnyFiling) -> Option<String> {
    let id = &filing.filing_id;
    let filer = &filing.cover.filer_id;
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) && !filer.is_empty())
        .then(|| format!("https://docquery.fec.gov/cgi-bin/forms/{filer}/{id}/"))
}

/// A section heading inside the body.
pub fn heading(r: &mut Receipt, text: &str) {
    r.rule('-');
    r.bold(&text.to_uppercase());
}

/// Two-column rows of a detailed summary page, skipping all-zero lines.
pub fn detail_rows(r: &mut Receipt, rows: &[(&str, fec_parser::covers::DetailedSummaryRow)]) {
    for (label, row) in rows {
        if row.column_a != 0.0 || row.column_b != 0.0 {
            r.money2(label, row.column_a, row.column_b, false);
        }
    }
}

/// The headline cash flow every financial report leads with.
pub fn cash_flow(r: &mut Receipt, begin: f64, receipts: f64, disbursements: f64, end: f64) {
    r.money("Cash on hand, start", begin, false);
    r.money("+ Receipts", receipts, false);
    r.money("- Disbursements", disbursements, false);
    r.money("Cash on hand, end", end, true);
}

/// Debts lines, only when non-zero.
pub fn debts(r: &mut Receipt, owed_to: f64, owed_by: f64) {
    if owed_to != 0.0 {
        r.money("Debts owed to them", owed_to, false);
    }
    if owed_by != 0.0 {
        r.money("Debts they owe", owed_by, false);
    }
}

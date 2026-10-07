//! Form 24: 24/48-hour report of independent expenditures. The cover says
//! little, so this one reads the Schedule E rows after it. By default it
//! prints one total per candidate supported or opposed; `--full` lists every
//! expenditure: when, to whom, and what for.

use fec_parser::{covers::Form24, mappings::column_names_for_field};

use super::*;

struct Expenditure {
    amount: f64,
    /// `SUPPORT` / `OPPOSE` and the candidate, e.g. `OPPOSE Adelita Grijalva, AZ-07`.
    target: String,
    payee: String,
    purpose: String,
    date: String,
}

pub fn render(r: &mut Receipt, filing: &mut AnyFiling, f: &Form24, opts: Opts) {
    banner(r, filing);
    filer(r, &f.committee_name, &f.filer_committee_id, &f.address);
    r.rule('-');
    r.kv_opt("Report", f.report_type_label());
    r.kv_opt("Amends", f.original_amendment_date.map(date_text));

    let (items, error) = expenditures(filing);
    if !items.is_empty() {
        let total: f64 = items.iter().map(|e| e.amount).sum();
        heading(r, &format!("{} expenditures", items.len()));
        if opts.full {
            for (i, e) in items.iter().enumerate() {
                if i > 0 {
                    r.text("");
                }
                r.money(&e.target, e.amount, true);
                r.text(&format!("{} - {}", e.date, e.payee));
                if !e.purpose.is_empty() {
                    r.text(&e.purpose);
                }
            }
        } else {
            // One line per candidate, in order of first appearance.
            let mut targets: Vec<(&str, f64)> = Vec::new();
            for e in &items {
                match targets.iter_mut().find(|(t, _)| *t == e.target) {
                    Some((_, sum)) => *sum += e.amount,
                    None => targets.push((&e.target, e.amount)),
                }
            }
            for (target, sum) in targets {
                r.money(target, sum, false);
            }
        }
        r.rule('-');
        r.money("TOTAL", total, true);
    }
    if let Some(error) = error {
        r.warnings.push(error);
    }

    footer(r, filing, Some(&f.treasurer), f.date_signed);
}

/// Every Schedule E row in the filing, by column name for its FEC version.
fn expenditures(filing: &mut AnyFiling) -> (Vec<Expenditure>, Option<String>) {
    let version = filing.header.fec_version.clone();
    let mut out = Vec::new();
    while let Some(row) = filing.next_row() {
        let row = match row {
            Ok(row) => row,
            Err(e) => return (out, Some(format!("stopped reading rows: {e:?}"))),
        };
        if !row.row_type.starts_with("SE") {
            continue;
        }
        let Ok(names) = column_names_for_field(&row.row_type, &version) else {
            continue;
        };
        let get = |name: &str| -> String {
            names
                .iter()
                .position(|n| n == name)
                .and_then(|i| row.record.get(i))
                .unwrap_or("")
                .trim()
                .to_owned()
        };
        let first_of = |names: &[&str]| -> String {
            let joined = |n: &&str| {
                n.split('+')
                    .map(|c| get(c))
                    .filter(|v| !v.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            names
                .iter()
                .map(joined)
                .find(|v| !v.is_empty())
                .unwrap_or_default()
        };
        let payee = first_of(&[
            "payee_organization_name",
            "payee_first_name+payee_last_name",
            "payee_name",
        ]);
        let candidate = first_of(&["candidate_first_name+candidate_last_name", "candidate_name"]);
        let seat = match (get("candidate_state"), get("candidate_district")) {
            (st, d) if !st.is_empty() && !d.is_empty() && d != "00" => format!(", {st}-{d}"),
            (st, _) if !st.is_empty() => format!(", {st}"),
            _ => String::new(),
        };
        let verb = match get("support_oppose_code").to_uppercase().as_str() {
            "S" => "SUPPORT",
            "O" => "OPPOSE",
            _ => "RE:",
        };
        out.push(Expenditure {
            amount: get("expenditure_amount").parse().unwrap_or(0.0),
            target: format!(
                "{verb} {}{seat}",
                if candidate.is_empty() {
                    "?"
                } else {
                    &candidate
                }
            ),
            payee,
            purpose: get("expenditure_purpose_descrip"),
            date: fec_date(&first_of(&["dissemination_date", "disbursement_date"])),
        });
    }
    (out, None)
}

/// `20250710` → `Jul 10, 2025`; anything else unchanged.
fn fec_date(s: &str) -> String {
    jiff::civil::Date::strptime("%Y%m%d", s)
        .map(date_text)
        .unwrap_or_else(|_| s.to_owned())
}

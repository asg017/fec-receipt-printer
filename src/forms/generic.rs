//! Every other form: the shared frame, plus the few fields that say the most
//! about the common ones (who the candidate is, what kind of committee).

use fec_parser::covers::{Address, Cover};

use super::*;

pub fn render(r: &mut Receipt, filing: &AnyFiling, cover: Option<&Cover>) {
    banner(r, filing);
    let c = &filing.cover;
    let name = cover
        .and_then(Cover::filer_name)
        .unwrap_or_else(|| c.filer_name.clone());
    let address = match cover {
        Some(Cover::Form1(f)) => f.address.clone(),
        Some(Cover::Form2(f)) => f.candidate_address.clone(),
        Some(Cover::Form5(f)) => f.address.clone(),
        Some(Cover::Form6(f)) => f.address.clone(),
        _ => Address::default(),
    };
    filer(r, &name, &c.filer_id, &address);
    r.rule('-');

    match cover {
        Some(Cover::Form1(f)) => {
            r.kv_opt(
                "Type",
                f.committee_type_label().or(f.committee_type.as_deref()),
            );
            if let Some(cand) = &f.candidate {
                r.kv("For", &cand.name.to_string());
                r.kv_opt(
                    "Office",
                    seat(
                        cand.office_label(),
                        cand.state.as_deref(),
                        cand.district.as_deref(),
                    ),
                );
            }
            r.kv_opt("Party", f.party_code_label().or(f.party_code.as_deref()));
            // Usually the signer too, who the footer already names.
            if f.treasurer.name != f.signer && !f.signer.is_empty() {
                r.kv("Treas", &f.treasurer.name.to_string());
            }
            r.kv_opt("Web", f.committee_url.as_deref());
        }
        Some(Cover::Form2(f)) => {
            r.kv_opt(
                "Office",
                seat(
                    f.office_label(),
                    f.office_state.as_deref(),
                    f.district.as_deref(),
                ),
            );
            r.kv_opt("Party", f.party_code_label().or(f.party_code.as_deref()));
            r.kv_opt("Year", f.election_year.map(|y| y.to_string()));
            r.kv_opt("Cmte", f.principal_committee.name.as_deref());
        }
        Some(Cover::Form6(f)) => {
            r.kv("For", &f.candidate.name.to_string());
            r.kv_opt(
                "Office",
                seat(
                    f.candidate.office_label(),
                    f.candidate.state.as_deref(),
                    f.candidate.district.as_deref(),
                ),
            );
        }
        Some(Cover::Form5(f)) => {
            r.kv_opt("Report", f.report_type_label().or(f.report_code_label()));
            r.kv_opt(
                "Period",
                period(f.coverage_from_date, f.coverage_through_date),
            );
            r.money("Contributions", f.total_contributions, false);
            r.money("Independent exp.", f.total_independent_expenditures, true);
        }
        _ => {
            let label = c.report_code.as_deref().map(fec_parser::report_code_label);
            report_lines(
                r,
                report_text(
                    c.report_code.as_deref(),
                    label.filter(|l| !l.starts_with('[')),
                ),
                c.coverage_from_date,
                c.coverage_through_date,
                None,
            );
        }
    }

    footer(
        r,
        filing,
        cover.and_then(Cover::signer),
        cover.and_then(Cover::date_signed),
    );
}

/// `House, TX-07`.
fn seat(office: Option<&str>, state: Option<&str>, district: Option<&str>) -> Option<String> {
    let place = match (state, district) {
        (Some(s), Some(d)) if !d.trim().is_empty() && d != "00" => Some(format!("{s}-{d}")),
        (Some(s), _) if !s.trim().is_empty() => Some(s.to_owned()),
        _ => None,
    };
    match (office, place) {
        (Some(o), Some(p)) => Some(format!("{o}, {p}")),
        (Some(o), None) => Some(o.to_owned()),
        (None, p) => p,
    }
}

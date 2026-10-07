//! Form 3: House and Senate campaign report.

use fec_parser::covers::Form3;

use super::*;

pub fn render(r: &mut Receipt, filing: &AnyFiling, f: &Form3, opts: Opts) {
    banner(r, filing);
    filer(r, &f.committee_name, &f.filer_committee_id, &f.address);
    let district = match (f.election_state.as_deref(), f.election_district.as_deref()) {
        (Some(st), Some(d)) if !d.trim().is_empty() && d != "00" => Some(format!("{st}-{d}")),
        (Some(st), _) => Some(st.to_owned()),
        _ => None,
    };
    r.rule('-');
    r.kv_opt("Seat", district);
    report_lines(
        r,
        report_text(f.report_code.as_deref(), f.report_code_label()),
        f.coverage_from_date,
        f.coverage_through_date,
        election_text(
            f.election_code.as_deref(),
            f.election_code_label(),
            f.election_date,
            f.state_of_election.as_deref(),
        ),
    );

    let d = &f.detailed_summary;
    let c = &d.cash_summary;
    heading(r, "This period");
    cash_flow(
        r,
        c.line23_cash_on_hand_beginning,
        c.line24_total_receipts,
        c.line26_total_disbursements,
        c.line27_cash_on_hand_close,
    );
    debts(
        r,
        f.summary.line9_debts_owed_to_committee,
        f.summary.line10_debts_owed_by_committee,
    );

    if opts.full {
        let (rc, ds) = (&d.receipts, &d.disbursements);
        heading(r, "Receipts");
        r.money2_heading("PERIOD", "CYCLE");
        detail_rows(
            r,
            &[
                (
                    "11a Individuals, itemized",
                    rc.line11a_i_contributions_from_individuals_itemized,
                ),
                (
                    "11a Individuals, unitem.",
                    rc.line11a_ii_contributions_from_individuals_unitemized,
                ),
                (
                    "11b Party committees",
                    rc.line11b_political_party_committees,
                ),
                ("11c PACs", rc.line11c_other_political_committees_pacs),
                ("11d The candidate", rc.line11d_the_candidate),
                ("11e Total contributions", rc.line11e_total_contributions),
                ("12 Transfers in", rc.line12_transfers_from_authorized),
                ("13a Loans from candidate", rc.line13a_loans_from_candidate),
                ("13b Other loans", rc.line13b_other_loans),
                ("14 Offsets", rc.line14_offset_to_operating_expenditures),
                ("15 Other receipts", rc.line15_other_receipts),
                ("16 TOTAL RECEIPTS", rc.line16_total_receipts),
            ],
        );
        heading(r, "Disbursements");
        r.money2_heading("PERIOD", "CYCLE");
        detail_rows(
            r,
            &[
                ("17 Operating", ds.line17_operating_expenditures),
                ("18 Transfers out", ds.line18_transfers_to_authorized),
                ("19a Repaid candidate", ds.line19a_candidate_loan_repayments),
                ("19b Other loan repay.", ds.line19b_other_loan_repayments),
                ("20 Refunds", ds.line20d_total_refunds),
                ("21 Other", ds.line21_other_disbursements),
                ("22 TOTAL DISBURSEMENTS", ds.line22_total_disbursements),
            ],
        );
    }

    footer(r, filing, Some(&f.treasurer), f.date_signed);
}

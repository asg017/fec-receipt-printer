//! Form 3X: PAC and party committee report.

use fec_parser::covers::Form3X;

use super::*;

pub fn render(r: &mut Receipt, filing: &AnyFiling, f: &Form3X, opts: Opts) {
    banner(r, filing);
    filer(r, &f.committee_name, &f.filer_committee_id, &f.address);
    if f.qualified_committee {
        r.text("Multicandidate committee");
    }
    r.rule('-');
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

    let s = &f.summary;
    heading(r, "This period");
    cash_flow(
        r,
        s.line6b_cash_on_hand_beginning_period,
        s.line6c_total_receipts.column_a,
        s.line7_total_disbursements.column_a,
        s.line8_cash_on_hand_close_of_period.column_a,
    );
    debts(
        r,
        s.line9_debts_owed_to_committee,
        s.line10_debts_owed_by_committee,
    );

    if opts.full {
        let d = &f.detailed_summary;
        let (rc, ds) = (&d.receipts, &d.disbursements);
        heading(r, "Receipts");
        r.money2_heading("PERIOD", "YEAR");
        detail_rows(
            r,
            &[
                (
                    "11a Individuals, itemized",
                    rc.line11a_i_individuals_itemized,
                ),
                (
                    "11a Individuals, unitem.",
                    rc.line11a_ii_individuals_unitemized,
                ),
                (
                    "11b Party committees",
                    rc.line11b_political_party_committees,
                ),
                (
                    "11c Other committees",
                    rc.line11c_other_political_committees,
                ),
                ("11d Total contributions", rc.line11d_total_contributions),
                ("12 Transfers in", rc.line12_transfers_from_affiliated),
                ("13 Loans received", rc.line13_loans_received),
                ("14 Loan repayments", rc.line14_loan_repayments_received),
                ("15 Offsets", rc.line15_offsets_to_operating_expenditures),
                ("16 Refunds", rc.line16_refunds_of_federal_contributions),
                ("17 Other receipts", rc.line17_other_federal_receipts),
                (
                    "18 Non-federal transfers",
                    rc.line18c_total_nonfederal_transfers,
                ),
                ("19 TOTAL RECEIPTS", rc.line19_total_receipts),
            ],
        );
        heading(r, "Disbursements");
        r.money2_heading("PERIOD", "YEAR");
        detail_rows(
            r,
            &[
                ("21 Operating", ds.line21c_total_operating_expenditures),
                ("22 Transfers out", ds.line22_transfers_to_affiliated),
                (
                    "23 To candidates",
                    ds.line23_contributions_to_federal_candidates,
                ),
                ("24 Independent exp.", ds.line24_independent_expenditures),
                (
                    "25 Coordinated exp.",
                    ds.line25_coordinated_party_expenditures,
                ),
                ("26 Loan repayments", ds.line26_loan_repayments_made),
                ("27 Loans made", ds.line27_loans_made),
                ("28 Refunds", ds.line28d_total_contribution_refunds),
                ("29 Other", ds.line29_other_disbursements),
                ("30 Fed. election activity", ds.line30c_fea_total),
                ("31 TOTAL DISBURSEMENTS", ds.line31_total_disbursements),
            ],
        );
    }

    footer(r, filing, Some(&f.treasurer), f.date_signed);
}

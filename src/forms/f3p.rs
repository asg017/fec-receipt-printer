//! Form 3P: presidential campaign report.

use fec_parser::covers::Form3P;

use super::*;

pub fn render(r: &mut Receipt, filing: &AnyFiling, f: &Form3P, opts: Opts) {
    banner(r, filing);
    filer(r, &f.committee_name, &f.filer_committee_id, &f.address);
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
        s.line6_cash_on_hand_beginning_period,
        s.line7_total_receipts,
        s.line9_total_disbursements,
        s.line10_cash_on_hand_end_period,
    );
    debts(
        r,
        s.line11_debts_owed_to_committee,
        s.line12_debts_owed_by_committee,
    );

    if opts.full {
        let d = &f.detailed_summary;
        let (rc, ds) = (&d.receipts, &d.disbursements);
        heading(r, "Receipts");
        r.money2_heading("PERIOD", "CYCLE");
        detail_rows(
            r,
            &[
                ("16 Federal funds", rc.line16_federal_funds),
                (
                    "17a Individuals, itemized",
                    rc.line17a_i_contributions_from_individuals_itemized,
                ),
                (
                    "17a Individuals, unitem.",
                    rc.line17a_ii_contributions_from_individuals_unitemized,
                ),
                (
                    "17b Party committees",
                    rc.line17b_political_party_committees,
                ),
                (
                    "17c Other committees",
                    rc.line17c_other_political_committees,
                ),
                ("17d The candidate", rc.line17d_the_candidate),
                ("17e Total contributions", rc.line17e_total_contributions),
                (
                    "18 Transfers in",
                    rc.line18_transfers_from_other_authorized_committee,
                ),
                ("19 Loans", rc.line19c_total_loans),
                ("20 Offsets", rc.line20d_offsets_to_expenditures_total),
                ("21 Other receipts", rc.line21_other_receipts),
                ("22 TOTAL RECEIPTS", rc.line22_total_receipts),
            ],
        );
        heading(r, "Disbursements");
        r.money2_heading("PERIOD", "CYCLE");
        detail_rows(
            r,
            &[
                ("23 Operating", ds.line23_operating_expenditures),
                (
                    "24 Transfers out",
                    ds.line24_transfers_to_other_authorized_committees,
                ),
                ("25 Fundraising", ds.line25_fundraising_disbursements),
                (
                    "26 Legal & accounting",
                    ds.line26_exempt_legal_and_accounting_disbursements,
                ),
                ("27 Loan repayments", ds.line27c_loan_repayments_total),
                ("28 Refunds", ds.line28d_refunds_total),
                ("29 Other", ds.line29_other_disbursements),
                ("30 TOTAL DISBURSEMENTS", ds.line30_total_disbursements),
            ],
        );
    }

    footer(r, filing, Some(&f.treasurer), f.date_signed);
}

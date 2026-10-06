// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Recording a projection's runtime losses against its closed contract.
//!
//! Every projection declares, as a static [`LossLedger`] contract, each loss
//! it can incur; at run time it records an instance of one of those entries,
//! located at the value that lost. The entry's `from`, `to` and `note` are the
//! contract's, never re-spelled at the recording site, so a runtime ledger can
//! only ever carry what the contract declared. This module is that one step.

use std::borrow::Cow;

use purrdf_core::loss::check_ledger_sound;
use purrdf_core::{LossEntry, LossLedger, RdfLocation};
use purrdf_lex::json::record::ToJson;

use super::ProjectionError;
use super::source_rows::source_identifier;

/// Record into `ledger` the contract entry `code`, located at `location`.
///
/// # Panics
///
/// When `contract` declares no entry `code`: every runtime code is a constant
/// its projection's closed contract declares, so an undeclared one is a
/// programming error, not a data condition.
pub(crate) fn record_contract_loss(
    ledger: &mut LossLedger,
    contract: &LossLedger,
    code: &'static str,
    location: RdfLocation,
) {
    let template = contract
        .entries()
        .iter()
        .find(|entry| entry.code == code)
        .unwrap_or_else(|| panic!("runtime loss code `{code}` must exist in the closed contract"));
    ledger.record(LossEntry {
        code: Cow::Borrowed(code),
        from: template.from.clone(),
        to: template.to.clone(),
        note: template.note.clone(),
        location: Some(Box::new(location)),
    });
}

/// Record the contract entry `code` at the logical location `logical`, on
/// `subject`.
pub(crate) fn record_logical_loss(
    ledger: &mut LossLedger,
    contract: &LossLedger,
    code: &'static str,
    logical: &str,
    subject: impl Into<String>,
) {
    record_contract_loss(
        ledger,
        contract,
        code,
        RdfLocation::logical(logical).with_subject(subject),
    );
}

/// Record the contract entry `code` for one source row at the logical
/// location `logical`, the row named by its stable identifier under `prefix`.
///
/// # Errors
///
/// Returns [`source_identifier`]'s refusal.
pub(crate) fn record_row_loss(
    ledger: &mut LossLedger,
    contract: &LossLedger,
    code: &'static str,
    logical: &str,
    prefix: &str,
    row: &impl ToJson,
) -> Result<(), ProjectionError> {
    let subject = source_identifier(prefix, row)?;
    record_logical_loss(ledger, contract, code, logical, subject);
    Ok(())
}

/// Refuse a runtime ledger that records a loss the `from` → `to` pair's
/// contract does not declare.
///
/// # Errors
///
/// Returns an integrity error naming the undeclared loss.
pub(crate) fn ensure_sound(
    ledger: &LossLedger,
    from: &str,
    to: &str,
) -> Result<(), ProjectionError> {
    check_ledger_sound(ledger, from, to).map_err(ProjectionError::integrity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> LossLedger {
        LossLedger::contract(vec![LossEntry {
            code: Cow::Borrowed("example-dropped"),
            from: Cow::Borrowed("rdf-1.2-dataset"),
            to: Cow::Borrowed("example"),
            note: Cow::Borrowed("An example value is dropped."),
            location: None,
        }])
    }

    #[test]
    fn a_recorded_loss_carries_the_contract_entry_at_its_location() {
        let contract = contract();
        let mut ledger = LossLedger::new();
        record_logical_loss(
            &mut ledger,
            &contract,
            "example-dropped",
            "example:quad",
            "q1",
        );
        let [entry] = ledger.entries() else {
            panic!("exactly one entry");
        };
        assert_eq!(entry.code, "example-dropped");
        assert_eq!(entry.to, "example");
        assert_eq!(entry.note, "An example value is dropped.");
        assert_eq!(
            entry.location.as_deref(),
            Some(&RdfLocation::logical("example:quad").with_subject("q1"))
        );
    }

    #[test]
    fn a_row_loss_names_the_row_by_its_stable_identifier() {
        let contract = contract();
        let mut ledger = LossLedger::new();
        record_row_loss(
            &mut ledger,
            &contract,
            "example-dropped",
            "example:quad",
            "quad",
            &"row",
        )
        .expect("identifier");
        let subject = source_identifier("quad", &"row").expect("identifier");
        assert_eq!(
            ledger.entries()[0].location.as_deref(),
            Some(&RdfLocation::logical("example:quad").with_subject(subject))
        );
    }

    #[test]
    #[should_panic(expected = "runtime loss code `undeclared` must exist in the closed contract")]
    fn an_undeclared_code_is_a_programming_error() {
        record_logical_loss(
            &mut LossLedger::new(),
            &contract(),
            "undeclared",
            "example:quad",
            "q1",
        );
    }
}

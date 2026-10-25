// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A downstream projection selects its loss contract without global registration.

use purrdf_core::loss::{
    LossDefinition, LossEntry, LossLedger, LossProfile, LossSoundnessError, check_ledger_sound,
    check_ledger_sound_in,
};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

/// Dynamic caller vocabulary must remain owned after registration input disposal.
fn definition(code: &str, description: &str) -> LossDefinition {
    LossDefinition {
        from: "source-model".to_owned().into(),
        to: "target-json".to_owned().into(),
        code: code.to_owned().into(),
        description: description.to_owned().into(),
    }
}

/// Soundness and rendering use the same explicit profile and original pair.
fn downstream_projection_checks_soundness_and_intent() {
    let definitions = [
        definition(
            "regex-post-check",
            "Regex constraints require a post-generation check",
        ),
        definition(
            "ownership-post-check",
            "Ownership requires a post-generation check",
        ),
    ];
    let profile = LossProfile::builtin().extend(definitions.clone()).unwrap();
    let reverse = LossProfile::builtin()
        .extend(definitions.into_iter().rev())
        .unwrap();
    let mut ledger = LossLedger::new();
    for code in ["regex-post-check", "ownership-post-check"] {
        ledger.record(LossEntry {
            code: code.to_owned().into(),
            from: "source-model".to_owned().into(),
            to: "target-json".to_owned().into(),
            note: "The particular input needs an enforced check"
                .to_owned()
                .into(),
            location: None,
        });
    }
    check_ledger_sound_in(&profile, &ledger, "source-model", "target-json").unwrap();
    assert!(check_ledger_sound(&ledger, "source-model", "target-json").is_err());
    let builtin_json = ledger.render_json();
    assert_eq!(builtin_json.matches("\"intentional\": false").count(), 2);
    assert_eq!(
        ledger.render_json_in(&profile),
        builtin_json.replace("\"intentional\": false", "\"intentional\": true")
    );
    assert_eq!(
        ledger.render_json_in(&profile),
        ledger.render_json_in(&reverse)
    );

    ledger.record(LossEntry {
        code: "unregistered-third-code".to_owned().into(),
        from: "source-model".to_owned().into(),
        to: "target-json".to_owned().into(),
        note: "An undeclared loss must fail soundness".to_owned().into(),
        location: None,
    });
    let error =
        check_ledger_sound_in(&profile, &ledger, "source-model", "target-json").unwrap_err();
    assert_eq!(
        error,
        LossSoundnessError::UnregisteredCode {
            code: "unregistered-third-code",
            from: "source-model",
            to: "target-json",
        }
    );
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "loss-unregistered-code");
    assert!(matches!(
        fields.field("code"),
        ErrorValue::Code("unregistered-third-code")
    ));
    assert_eq!(
        ledger
            .render_json_in(&profile)
            .matches("\"intentional\": false")
            .count(),
        1
    );

    ledger.record(LossEntry {
        code: "regex-post-check".into(),
        from: "wrong-source".into(),
        to: "target-json".into(),
        note: "A known code must not disguise the wrong conversion".into(),
        location: None,
    });
    // Restore the unregistered-code neighbor; the later pair mismatch then wins.
    let profile = profile
        .extend([definition(
            "unregistered-third-code",
            "Additional checked loss",
        )])
        .unwrap();
    let error =
        check_ledger_sound_in(&profile, &ledger, "source-model", "target-json").unwrap_err();
    assert!(matches!(
        error,
        LossSoundnessError::PairMismatch {
            code: "regex-post-check",
            actual_from: "wrong-source",
            actual_to: "target-json",
            expected_from: "source-model",
            expected_to: "target-json",
        }
    ));
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "loss-pair-mismatch");
    assert!(matches!(
        fields.field("actual_from"),
        ErrorValue::Code("wrong-source")
    ));
    assert!(matches!(
        fields.field("expected_from"),
        ErrorValue::Code("source-model")
    ));
    assert_eq!(
        ledger
            .render_json_in(&profile)
            .matches("\"intentional\": false")
            .count(),
        1
    );
    check_ledger_sound_in(&profile, &LossLedger::new(), "unknown", "unknown").unwrap();
}

/// Code collisions preserve pair-scoped builtin meanings and caller isolation.
fn conflicting_definitions_are_refused_without_global_mutation() {
    let builtin = LossProfile::builtin();
    let okf = builtin
        .description("rdf-1.2-dataset", "okf", "named-graph-dropped")
        .unwrap();
    let syntax = builtin
        .description("trig", "turtle", "named-graph-dropped")
        .unwrap();
    assert_ne!(okf, syntax);
    let same = LossDefinition {
        from: "rdf-1.2-dataset".into(),
        to: "okf".into(),
        code: "named-graph-dropped".into(),
        description: okf.to_owned().into(),
    };
    assert_eq!(builtin.clone().extend([same.clone()]).unwrap(), builtin);
    let mut conflicting = same;
    conflicting.description = syntax.to_owned().into();
    let pointer = conflicting.description.as_ptr();
    let error = builtin
        .clone()
        .extend([definition("new-code", "New checked loss"), conflicting])
        .unwrap_err();
    assert_eq!(error.definition.description.as_ptr(), pointer);
    assert_eq!(error.definition.description, syntax);
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "loss-profile-conflict");
    let rejected = fields.child("definition");
    assert!(matches!(
        rejected.field("code"),
        ErrorValue::Code("named-graph-dropped")
    ));
    assert!(matches!(rejected.field("description"), ErrorValue::Text(value) if value == syntax));
    assert!(
        builtin
            .description("source-model", "target-json", "new-code")
            .is_none()
    );
    assert_eq!(LossProfile::builtin(), builtin);

    let wrong = definition(
        "named-graph-dropped",
        "This different meaning must not shadow a builtin code",
    );
    assert!(builtin.clone().extend([wrong]).is_err());
    let reused = definition("named-graph-dropped", syntax);
    let extension = builtin.clone().extend([reused]).unwrap();
    assert_eq!(
        extension.description("source-model", "target-json", "named-graph-dropped"),
        Some(syntax)
    );
    assert_eq!(
        builtin.description("source-model", "target-json", "named-graph-dropped"),
        None
    );

    let profile = builtin
        .extend([definition("custom-code", "Original caller meaning")])
        .unwrap();
    assert!(
        profile
            .clone()
            .extend([definition("custom-code", "Conflicting caller meaning")])
            .is_err()
    );
    assert_eq!(
        profile.description("source-model", "target-json", "custom-code"),
        Some("Original caller meaning")
    );
}

purrdf_testkit::harness_main!(
    downstream_projection_checks_soundness_and_intent,
    conflicting_definitions_are_refused_without_global_mutation,
);

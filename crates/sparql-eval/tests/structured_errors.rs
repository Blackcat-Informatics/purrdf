// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrow original evaluator payloads through the generic consumer, on every host.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{ResourceDimension, StopCause, TrippedGovernor};
use purrdf_hash::error::ErrorValue;
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{
    AdmittedResolvedBindings, EvalError, NativeDiagnostic, NativeDiagnosticKind, RemoteError,
    RetainedDiagnostic, ServiceBuildError, ServiceCapability, ServiceDenial, UnsupportedKind,
    UserFunctionRefusal, WorkspaceCapability,
};
use purrdf_testkit::error_capture::{CallerRefusal, Capture};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn erased_function_causes_keep_original_fields_and_identity_without_rendering() {
    let error =
        EvalError::FunctionAdmission(UserFunctionRefusal::new(CallerRefusal { limit: u128::MAX }));
    let clone = error.clone();
    assert_eq!(error, clone);
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let capture = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    let cause = capture.child("cause").child("cause");
    assert_eq!(cause.code, "caller-refused");
    assert!(matches!(
        cause.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    let source = std::error::Error::source(&error).unwrap();
    assert_eq!(
        source.downcast_ref::<CallerRefusal>().unwrap().limit,
        u128::MAX
    );
    drop(error);
    assert_eq!(
        Capture::read(&clone).child("cause").child("cause").code,
        "caller-refused"
    );
}

fn parsed_errors_and_optional_classifications_keep_their_native_shapes() {
    let error = EvalError::from(SparqlParser::new().parse_query("%").unwrap_err());
    let capture = Capture::read(&error);
    assert_eq!(capture.code, "sparql-eval-parse");
    let lexical = capture.child("cause").child("reason");
    assert!(matches!(
        lexical.field("character"),
        ErrorValue::Character('%')
    ));
    for kind in [None, Some(UnsupportedKind::CustomFunction)] {
        let error = EvalError::Unsupported {
            what: "original construct".into(),
            kind,
        };
        let capture = Capture::read(&error);
        assert!(matches!(
            capture.field("what"),
            ErrorValue::Text("original construct")
        ));
        if kind.is_some() {
            assert_eq!(
                capture.child("kind").code,
                "sparql-unsupported-custom-function"
            );
        } else {
            assert!(matches!(capture.field("kind"), ErrorValue::Absent));
        }
    }
}

fn service_categories_preserve_external_messages_and_governor_quantities() {
    let endpoint = "https://example.org/service";
    let refusal = RemoteError::Denied(ServiceDenial::new(
        endpoint,
        ServiceCapability::Credentials,
        "caller policy",
    ));
    let fields = Capture::read(&refusal).child("cause");
    assert!(matches!(fields.field("endpoint"), ErrorValue::Iri(value) if value == endpoint));
    assert!(matches!(
        fields.field("detail"),
        ErrorValue::Message("caller policy")
    ));
    assert_eq!(
        fields.child("withheld").code,
        "service-capability-credentials"
    );
    for error in [
        RemoteError::Transport("host transport".into()),
        RemoteError::Decode("host decode".into()),
        RemoteError::Unconfigured("host routing".into()),
    ] {
        assert!(matches!(
            Capture::read(&error).field("message"),
            ErrorValue::Message(_)
        ));
    }
    for error in [
        RemoteError::Governed(TrippedGovernor::Budget {
            dimension: ResourceDimension::Bytes,
            limit: u64::MAX,
            consumed: u64::MAX - 1,
        }),
        RemoteError::GovernedAfterCompletion(TrippedGovernor::Refused {
            dimension: ResourceDimension::RemoteRequests,
            limit: u64::MAX - 1,
            estimate: u64::MAX,
        }),
    ] {
        let governor = Capture::read(&error).child("governor");
        let count = if governor.code == "governor-budget" {
            "consumed"
        } else {
            "estimate"
        };
        assert!(
            matches!(governor.field(count), ErrorValue::Unsigned(value) if value >= u128::from(u64::MAX - 1))
        );
        assert!(
            matches!(governor.field("limit"), ErrorValue::Unsigned(value) if value >= u128::from(u64::MAX - 1))
        );
        assert_ne!(governor.child("dimension").code, "");
    }
    let stopped = RemoteError::Governed(TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    });
    assert_eq!(
        Capture::read(&stopped)
            .child("governor")
            .child("cause")
            .code,
        "stop-cancelled"
    );
}

fn xpath_actual_refusals_keep_profile_offsets_and_full_width_resource_counts() {
    use purrdf_core::xsd_regex::xpath::{self, Limits, Profile, Resource};
    let flags = xpath::compile(Profile::Xpath31, "a", "z", Limits::default()).unwrap_err();
    let error = EvalError::XPathRegex(flags);
    let cause = Capture::read(&error).child("cause");
    assert_eq!(cause.code, "xpath-invalid-flags");
    assert!(matches!(cause.field("flag"), ErrorValue::Character('z')));
    assert!(matches!(cause.field("offset"), ErrorValue::ByteCount(0)));
    assert_eq!(cause.child("profile").code, "xpath-profile-3.1-2017-03-21");
    let refusal = Limits::default()
        .with(Resource::PatternBytes, 0)
        .admit(Resource::PatternBytes, u128::MAX)
        .unwrap_err();
    let capture = Capture::read(&refusal);
    assert!(matches!(
        capture.field("required"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    assert!(matches!(capture.field("limit"), ErrorValue::Unsigned(0)));
    let source = xpath::compile(
        Profile::Xpath20,
        "a",
        "",
        Limits::default().with(Resource::PatternBytes, 0),
    )
    .unwrap_err();
    assert_eq!(
        Capture::read(&source).child("cause").child("resource").code,
        "xpath-resource-pattern-bytes"
    );
}

fn admitted_diagnostics_borrow_original_text_and_clone_the_same_owner() {
    let workspace = WorkspaceCapability::default();
    let diagnostic = NativeDiagnostic::render(
        NativeDiagnosticKind::Internal,
        "original native message",
        &workspace,
    )
    .unwrap();
    let original = diagnostic.message().as_ptr();
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let clone = diagnostic.clone();
    let capture = Capture::read_with_buffer(&clone, events);
    assert_eq!(window.close().allocations, 0);
    assert!(matches!(capture.field("text"), ErrorValue::Text(text) if text.as_ptr() == original));
    assert_eq!(capture.child("kind").code, "sparql-native-internal");
    drop(diagnostic);
    assert_eq!(clone.message(), "original native message");
    let retained =
        RetainedDiagnostic::render("original-code", &"original detail", &workspace).unwrap();
    let error = EvalError::RetainedDiagnostic(retained);
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .is_some()
    );
    assert_eq!(
        Capture::read(&error).child("cause").child("cause").code,
        "rdf-diagnostic"
    );
}

fn actual_service_builder_preserves_original_failure_under_its_admission() {
    let workspace = WorkspaceCapability::default();
    let error = AdmittedResolvedBindings::try_build(&workspace, |memory| {
        Err(ServiceBuildError::Remote(RemoteError::HostDenied {
            endpoint: memory.string("https://example.org/service")?,
            message: memory.string("original host policy")?,
        }))
    })
    .unwrap_err();
    let admitted = Capture::read(&error).child("cause");
    assert_eq!(admitted.code, "service-admitted-error");
    let original = admitted.child("cause");
    assert_eq!(original.code, "service-host-denied");
    assert!(matches!(
        original.field("message"),
        ErrorValue::Message("original host policy")
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .is_some()
    );
}

fn admitted_service_native_causes_survive_without_rendering_or_inspection_allocation() {
    let workspace = WorkspaceCapability::resident();
    let failure = AdmittedResolvedBindings::try_build(&workspace, |memory| {
        let cause = EvalError::FunctionAdmission(UserFunctionRefusal::new(CallerRefusal {
            limit: u128::MAX,
        }));
        Err(ServiceBuildError::Remote(RemoteError::Evaluation(
            memory.boxed(cause)?,
        )))
    })
    .unwrap_err();
    let events = Vec::with_capacity(2);
    let window = CurrentThreadWindow::open();
    let capture = Capture::read_with_buffer(&failure, events);
    assert_eq!(window.close().allocations, 0);
    let cause = capture
        .child("cause")
        .child("cause")
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(cause.code, "caller-refused");
    assert!(matches!(
        cause.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    assert!(
        std::error::Error::source(&failure)
            .unwrap()
            .source()
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<EvalError>()
            .is_some()
    );
}

fn actual_protocol_errors_keep_parser_form_iri_and_operation_fields() {
    use purrdf_sparql_eval::protocol::{
        OperationKind, ProtocolError, ProtocolReason, ProtocolRequest,
    };
    const {
        assert!(size_of::<ProtocolError>() < 128);
    }
    let request =
        ProtocolRequest::parse("POST", Some("application/sparql-query"), None, b"%").unwrap();
    let error = request.result_kind().unwrap_err();
    assert_eq!(error.status(), 400);
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let capture = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    let lexical = capture.child("cause").child("cause");
    assert_eq!(lexical.code, "sparql-parse-lex");
    assert!(matches!(lexical.field("at"), ErrorValue::ByteCount(0)));
    assert!(matches!(
        lexical.child("reason").field("character"),
        ErrorValue::Character('%')
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .unwrap()
            .is::<purrdf_sparql_algebra::ParseError>()
    );

    let error = ProtocolRequest::parse("GET", None, Some("query=%"), b"").unwrap_err();
    let form = Capture::read(&error).child("reason");
    assert!(matches!(form.field("input"), ErrorValue::Text("%")));
    assert!(matches!(
        form.child("cause").field("offset"),
        ErrorValue::ByteCount(0)
    ));

    let error = ProtocolRequest::parse(
        "POST",
        Some("application/x-www-form-urlencoded"),
        None,
        b"x\xff",
    )
    .unwrap_err();
    assert!(matches!(
        Capture::read(&error).child("reason").field("valid_up_to"),
        ErrorValue::ByteCount(1)
    ));

    let error = ProtocolRequest::parse("GET", None, Some("query=a&query=b"), b"").unwrap_err();
    let repeated = Capture::read(&error).child("reason");
    assert!(matches!(repeated.field("count"), ErrorValue::Unsigned(2)));
    assert_eq!(repeated.child("kind").code, "sparql-operation-query");

    let error = ProtocolRequest::parse(
        "GET",
        None,
        Some("query=ASK+%7B%7D&default-graph-uri=http%3A%2F%2Fexample.org%2Fa+b"),
        b"",
    )
    .unwrap_err();
    let iri = Capture::read(&error);
    assert!(matches!(
        iri.field("value"),
        ErrorValue::Iri("http://example.org/a b")
    ));
    let original = purrdf_iri::is_absolute("http://example.org/a b").unwrap_err();
    assert_eq!(
        iri.child("reason").child("cause").code,
        original.diagnostic_code()
    );

    let request =
        ProtocolRequest::parse("POST", Some("application/sparql-query"), None, b"BLAH").unwrap();
    assert!(matches!(
        Capture::read(&request.result_kind().unwrap_err())
            .child("cause")
            .field("word"),
        ErrorValue::Text("BLAH")
    ));
    for cause in [
        ProtocolReason::Storage(purrdf_lex::allocation::StorageError::AllocationFailed),
        purrdf_sparql_algebra::ParseError::Storage(
            purrdf_lex::allocation::StorageError::AllocationFailed,
        )
        .into(),
        EvalError::Internal("native environment fault".into()).into(),
    ] {
        let error = ProtocolError::MalformedOperation {
            kind: OperationKind::Query,
            cause,
        };
        assert_eq!(error.status(), 500);
        assert!(std::error::Error::source(&error).is_some());
    }
}

fn control_and_load_errors_keep_native_causes_and_genuine_messages() {
    use purrdf_sparql_eval::{LoadError, QueryControlFailure};
    for error in [
        QueryControlFailure::Unpriced("original producer"),
        QueryControlFailure::LayoutOverflow,
        QueryControlFailure::DiagnosticFormatting,
        QueryControlFailure::WorkspaceStopped,
    ] {
        assert_ne!(Capture::read(&error).code, "");
    }
    for error in [
        LoadError::Transport("host transport".into()),
        LoadError::Decode("host decode".into()),
        LoadError::HostDenied("host policy".into()),
        LoadError::Fault("host fault".into()),
    ] {
        assert!(matches!(
            Capture::read(&error).field("message"),
            ErrorValue::Message(_)
        ));
    }
    let denied = LoadError::Denied(ServiceDenial::new(
        "https://example.org/load",
        ServiceCapability::Network,
        "caller policy",
    ));
    assert_eq!(
        Capture::read(&denied).child("cause").child("withheld").code,
        "service-capability-network"
    );
    let stopped = LoadError::Governed(TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    });
    assert_eq!(
        Capture::read(&stopped)
            .child("governor")
            .child("cause")
            .code,
        "stop-cancelled"
    );
}

fn native_protocol_resource_refusals_remain_allocation_free() {
    use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};
    use purrdf_sparql_eval::protocol::{OperationKind, ProtocolError, ProtocolReason};
    let capacity = CapacityError::new(CapacitySpace::GlobalTerms, u128::MAX, u128::MAX - 1);
    let window = CurrentThreadWindow::open();
    let parsed = ProtocolReason::from(purrdf_sparql_algebra::ParseError::Storage(
        StorageError::Capacity(capacity),
    ));
    let stopped = ProtocolReason::from(EvalError::WorkspaceStopped);
    let allocator = ProtocolReason::from(EvalError::AllocationFailed {
        construct: "original native allocation",
    });
    let exhausted = ProtocolReason::from(EvalError::Capacity(capacity));
    assert_eq!(window.close().allocations, 0);
    for original in [
        Capture::read(&parsed).child("cause").child("cause"),
        Capture::read(&exhausted).child("cause"),
    ] {
        assert!(matches!(
            original.field("attempted"),
            ErrorValue::Unsigned(u128::MAX)
        ));
        assert!(
            matches!(original.field("limit"), ErrorValue::Unsigned(value) if value == u128::MAX - 1)
        );
    }
    assert!(matches!(
        Capture::read(&allocator).field("construct"),
        ErrorValue::Text("original native allocation")
    ));
    for cause in [parsed, stopped, allocator, exhausted] {
        let error = ProtocolError::MalformedOperation {
            kind: OperationKind::Query,
            cause,
        };
        assert_eq!(error.status(), 500);
    }
}

purrdf_testkit::harness_main!(
    erased_function_causes_keep_original_fields_and_identity_without_rendering,
    parsed_errors_and_optional_classifications_keep_their_native_shapes,
    service_categories_preserve_external_messages_and_governor_quantities,
    xpath_actual_refusals_keep_profile_offsets_and_full_width_resource_counts,
    admitted_diagnostics_borrow_original_text_and_clone_the_same_owner,
    actual_service_builder_preserves_original_failure_under_its_admission,
    admitted_service_native_causes_survive_without_rendering_or_inspection_allocation,
    actual_protocol_errors_keep_parser_form_iri_and_operation_fields,
    control_and_load_errors_keep_native_causes_and_genuine_messages,
    native_protocol_resource_refusals_remain_allocation_free,
);

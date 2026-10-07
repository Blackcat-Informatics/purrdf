// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Count units, comparable identities and actual acknowledged control I/O.

use purrdf_testkit::bench::Estimates;
use purrdf_testkit::bench::counter::{CountContext, CountReport, CountUnit, InstructionSample};

fn context() -> CountContext {
    CountContext {
        unit: CountUnit::Instructions,
        work: 5,
        threads: 4,
        fixture: "fixture-query-v1".to_owned(),
        boundary: "public-query-five-results-retained".to_owned(),
    }
}

#[test]
fn count_reports_preserve_raw_totals_and_explicit_units() {
    let report = CountReport::from_samples(context(), (100..110).collect(), 17).unwrap();
    let text = report.to_json().unwrap();
    assert!(text.contains("\"schema\": 2"));
    assert!(text.contains("\"unit\": \"instructions\""));
    assert_eq!(CountReport::from_json(&text).unwrap(), report);
    assert!(
        Estimates::from_json(&text).is_err(),
        "counts must not enter the ns reader"
    );
    let legacy = report.estimates().to_json().unwrap();
    assert!(legacy.contains("\"schema\": 1"));
    assert!(legacy.contains("\"unit\": \"ns\""));
    assert!(CountReport::from_json(&legacy).is_err());
    assert_eq!(&Estimates::from_json(&legacy).unwrap(), report.estimates());
    for unit in [
        CountUnit::AllocationCalls,
        CountUnit::RequestedBytes,
        CountUnit::RetainedBytes,
        CountUnit::PeakBytes,
    ] {
        let mut identity = context();
        identity.unit = unit;
        identity.work = 1;
        identity.boundary = "public-query-one-result-retained".to_owned();
        let raw = if unit == CountUnit::RetainedBytes {
            vec![-7; 10]
        } else {
            vec![7; 10]
        };
        let report = CountReport::from_samples(identity, raw, 19).unwrap();
        assert_eq!(
            CountReport::from_json(&report.to_json().unwrap()).unwrap(),
            report
        );
    }
}

#[test]
fn comparisons_refuse_different_units_work_workers_fixtures_and_boundaries() {
    let base = CountReport::from_samples(context(), vec![100; 10], 1).unwrap();
    let new = CountReport::from_samples(context(), vec![110; 10], 2).unwrap();
    let change = new.compare("base", &base, 3).unwrap();
    assert!((change.point - 0.1).abs() < 1e-12);
    for changed in [
        CountContext {
            unit: CountUnit::AllocationCalls,
            ..context()
        },
        CountContext {
            work: 1,
            ..context()
        },
        CountContext {
            threads: 32,
            ..context()
        },
        CountContext {
            fixture: "other-fixture".to_owned(),
            ..context()
        },
        CountContext {
            boundary: "other-boundary".to_owned(),
            ..context()
        },
    ] {
        let other = CountReport::from_samples(changed, vec![100; 10], 4).unwrap();
        assert!(new.compare("other", &other, 5).is_err());
    }
    let identity = CountContext {
        unit: CountUnit::RetainedBytes,
        ..context()
    };
    let negative = CountReport::from_samples(identity.clone(), vec![-10; 10], 6).unwrap();
    let less_negative = CountReport::from_samples(identity, vec![-5; 10], 6).unwrap();
    assert!(less_negative.compare("negative", &negative, 6).is_err());
    let zero = CountReport::from_samples(context(), vec![0; 10], 6).unwrap();
    assert!(new.compare("zero", &zero, 6).is_err());
}

#[test]
fn malformed_count_records_fail_instead_of_changing_the_measurement() {
    let text = CountReport::from_samples(context(), vec![100; 10], 6)
        .unwrap()
        .to_json()
        .unwrap();
    for bad in [
        text.replace("\"instructions\"", "\"cycles\""),
        text.replace("\"threads\": 4", "\"threads\": 0"),
        text.replace(
            "\"iterations_per_sample\": 5",
            "\"iterations_per_sample\": 0",
        ),
        text.replace("\"raw_samples\": [100", "\"raw_samples\": [101"),
        text.replace("\"median\": 20", "\"median\": 21"),
        text.replace("\"mad\": 0", "\"mad\": 1"),
        text.replace("\"ci_low\": 20", "\"ci_low\": 19"),
        text.replace("\"fixture\": \"fixture-query-v1\"", "\"fixture\": \"\""),
        text.replace("\"schema\": 2", "\"schema\": 2, \"schema\": 2"),
    ] {
        assert!(CountReport::from_json(&bad).is_err(), "accepted {bad}");
    }
    assert!(CountReport::from_samples(context(), vec![-1; 10], 7).is_err());
    assert!(CountReport::from_samples(context(), Vec::new(), 7).is_err());
}

#[test]
fn exact_counter_coverage_refuses_partial_or_rounded_multiplexing() {
    let csv = "12345,,instructions:u,900,100.00,,\n";
    let verbose = "instructions:u: 12345 900 900\n";
    let sample = InstructionSample::from_perf(csv, verbose).unwrap();
    assert_eq!(
        InstructionSample::from_perf(
            &format!("{verbose}{csv}"),
            "Events enabled\nEvents disabled\n"
        )
        .unwrap(),
        sample,
        "perf --output keeps its exact tuple in the output file"
    );
    assert_eq!(sample.instructions, 12345);
    assert_eq!(sample.enabled, sample.running);
    for (csv, verbose) in [
        ("<not counted>,,instructions:u,900,100.00,,\n", verbose),
        ("<not supported>,,instructions:u,900,100.00,,\n", verbose),
        (csv, "instructions:u: 12345 901 900\n"),
        (csv, "instructions:u: 12346 900 900\n"),
        (csv, "instructions:u: 12345 900\n"),
        (
            "instructions:u: 12345 900 900\n12345,,instructions:u,900,100.00,,\n",
            verbose,
        ),
        (
            "instructions:u: 12346 900 900\n12345,,instructions:u,900,100.00,,\n",
            verbose,
        ),
        (
            "instructions:u: 12346 900 900\n12345,,instructions:u,900,100.00,,\n",
            "",
        ),
        (csv, ""),
        ("12345,,instructions:u,900,99.99,,\n", verbose),
        (
            "12345,,instructions:u,900,100.00,,\n12345,,instructions:u,900,100.00,,\n",
            verbose,
        ),
    ] {
        assert!(InstructionSample::from_perf(csv, verbose).is_err());
    }
    // Separate invocations are totals, not an accumulated counter masquerading
    // as samples: the parser preserves each newly collected value independently.
    let next = InstructionSample::from_perf(
        "23456,,instructions:u,800,100.00,,\n",
        "instructions:u: 23456 800 800\n",
    )
    .unwrap();
    assert_eq!(next.instructions, 23456);
}

#[cfg(target_os = "linux")]
#[test]
fn control_commands_verify_the_actual_acknowledgement_bytes() {
    use purrdf_testkit::bench::counter::{CounterCommand, CounterControl};
    let dir = purrdf_testkit::temp_dir!().unwrap();
    let command = dir.path().join("commands");
    let ack = dir.path().join("ack");
    std::fs::write(&command, b"").unwrap();
    std::fs::write(&ack, b"ack\n\0ack\n\0").unwrap();
    let mut control = CounterControl::open(&command, &ack).unwrap();
    control.command(CounterCommand::Enable).unwrap();
    control.command(CounterCommand::Disable).unwrap();
    assert_eq!(std::fs::read(&command).unwrap(), b"enable\ndisable\n");
    assert!(
        control.command(CounterCommand::Enable).is_err(),
        "short acknowledgement must fail"
    );
    std::fs::write(&ack, b"bad\n\0").unwrap();
    assert!(
        CounterControl::open(&command, &ack)
            .unwrap()
            .command(CounterCommand::Enable)
            .is_err()
    );
    assert!(CounterControl::open(&command, &dir.path().join("missing")).is_err());
}

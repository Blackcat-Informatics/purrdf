use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_conformance_kit::outcome::{grade, Observation, OutcomeKind};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn packaged_support() {
    let caller = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize().unwrap();
    assert_eq!(purrdf_testkit::workspace_root!(), caller);
    purrdf_testkit::assert_golden!("release.txt", "packaged downstream support\n");
    let scratch = purrdf_testkit::temp_dir!().unwrap();
    let scratch_path = scratch.path().to_owned();
    std::fs::write(scratch.path().join("caller.txt"), b"downstream").unwrap();
    assert_eq!(std::fs::read(scratch.path().join("caller.txt")).unwrap(), b"downstream");
    drop(scratch);
    assert!(!scratch_path.exists());
    assert_eq!(purrdf_testkit::jsonschema_metaschemas::all().count(), 17);
    assert!(purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12[0].1.contains("$schema"));
    let window = CurrentThreadWindow::open();
    let values = std::hint::black_box(vec![42_u64; 48]);
    let measurement = window.close();
    assert!(measurement.allocations >= 1);
    assert!(measurement.requested_bytes >= 384);
    assert_eq!(values.len(), 48);
    let expected = Observation { kind: OutcomeKind::Compared, reason: None, message: String::new() };
    grade(&expected, &expected).unwrap();
    let mismatch = Observation { kind: OutcomeKind::Mismatch, reason: None, message: "different result".into() };
    assert!(grade(&expected, &mismatch).is_err());
}

purrdf_testkit::harness_main!(packaged_support);

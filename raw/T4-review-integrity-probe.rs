use purrdf_gts::verify::{verify_file_with_options, VerifyOptions};
use purrdf_gts::writer::Writer;

fn main() {
    let options = VerifyOptions::default().require_signatures(false);
    let intact = Writer::new("purrdf.gts").into_bytes();
    let mut failures = 0;
    for profile in ["purrdf.gts", "evidence", "opaque"] {
        let mut writer = Writer::new(profile);
        writer.add_blob(b"unsigned", None, None);
        let bytes = writer.into_bytes();
        let graph = purrdf_gts::reader::read(&bytes, true, None);
        let expected_policy = purrdf_gts::policy::evaluate_profile_policy(&graph, Some(&options.trust_policy), None);
        let result = verify_file_with_options(&bytes, &options);
        println!("profile={profile}: ok={}, errors={:?}, findings={:?}", result.ok, result.errors, result.profile_findings);
        println!("profile={profile}: direct_policy={expected_policy:?}");
        if profile == "purrdf.gts" { assert!(result.ok); }
        else if result.ok { failures += 1; }
    }
    for (name, input) in [
        ("clean_header", intact.as_slice()),
        ("empty", &[][..]),
        ("torn_first_item", &[0x5f][..]),
        ("invalid_first_item", &[0xff][..]),
    ] {
        let result = verify_file_with_options(input, &options);
        println!("{name}: ok={}, errors={:?}, diagnostics={:?}", result.ok, result.errors, result.diagnostics);
        if name == "clean_header" { assert!(result.ok); }
        else if result.ok { failures += 1; }
    }
    assert_eq!(failures, 0, "unsigned opt-in accepted invalid file inputs");
}

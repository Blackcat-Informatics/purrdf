use std::fs;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2);
    let before = fs::read_to_string(&args[0]).unwrap();
    let after = fs::read_to_string(&args[1]).unwrap();
    let a: Vec<_> = before.lines().collect();
    let b: Vec<_> = after.lines().collect();
    assert_eq!(a.len(), b.len(), "line count changed");
    let mut case = "";
    let mut regime = "";
    let mut report = false;
    let mut changes = 0;
    let mut cases = 0;
    for (old, new) in a.iter().zip(&b) {
        if let Some(name) = old.strip_prefix("@case ") {
            case = name;
            cases += 1;
        }
        if let Some(name) = old.strip_prefix("@regime ") {
            regime = name;
        }
        if old.starts_with('@') {
            report = *old == "@report";
        }
        if old == new {
            continue;
        }
        assert!(report, "changed outside report: {case}: {old:?} -> {new:?}");
        assert!(
            ["contract-hash ", "budget join-steps "].iter().any(|prefix| {
                old.starts_with(prefix) && new.starts_with(prefix)
            }),
            "unexpected delta: {case}: {old:?} -> {new:?}"
        );
        println!("{case}\t{regime}\t{old}\t{new}");
        changes += 1;
    }
    assert_eq!(cases, 9);
    println!("PASS: all nine case/input/program/closure/proof/tally/non-work-budget bytes unchanged; {changes} hash/work lines differ");
}

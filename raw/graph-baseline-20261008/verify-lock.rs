// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use std::collections::BTreeMap;
use std::fs;

fn registry_records(path: &str) -> BTreeMap<(String, String), (String, String)> {
    let text = fs::read_to_string(path).expect("read retained Cargo lock");
    let mut records = BTreeMap::new();
    for block in text.split("[[package]]").skip(1) {
        let field = |key: &str| block.lines().find_map(|line| {
            line.strip_prefix(key).map(|value| value.trim_matches('"').to_owned())
        });
        let Some(source) = field("source = ") else { continue };
        if !source.starts_with("registry+") { continue }
        let name = field("name = ").expect("package name");
        let version = field("version = ").expect("package version");
        let checksum = field("checksum = ").expect("registry checksum");
        assert!(records.insert((name, version), (source, checksum)).is_none());
    }
    records
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let baseline = registry_records(&args[1]);
    let probe = registry_records(&args[2]);
    assert!(!probe.is_empty());
    for (package, record) in &probe {
        assert_eq!(baseline.get(package), Some(record), "dependency differs: {package:?}");
    }
    println!("{} registry packages match current-main lock versions, sources and checksums", probe.len());
}

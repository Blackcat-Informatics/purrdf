// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Extract only catalogue-parity diagnostics from a complete CI matrix.
//! This changes no report status and makes no passing-gate claim.

use std::collections::BTreeSet;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let sites: usize = args.first().expect("site count required").parse().unwrap();
    assert_eq!(args.len(), 8, "expected seven CONFIG=LOG arguments");
    let mut configurations = BTreeSet::new();
    let mut producer = None;
    let mut updates = BTreeSet::new();
    let mut output = Vec::new();
    for arg in &args[1..] {
        let (config, path) = arg.split_once('=').expect("CONFIG=LOG required");
        assert!(
            configurations.insert(config.to_owned()),
            "repeated configuration"
        );
        let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"));
        let compiler = text
            .lines()
            .find_map(|line| {
                let start = line.find("rustc 1.")?;
                let tail = &line[start..];
                let end = tail.find(')')?;
                Some(tail[..=end].to_owned())
            })
            .expect("actual producer version required");
        if let Some(expected) = &producer {
            assert_eq!(&compiler, expected, "mixed producer compiler: {path}");
        } else {
            producer = Some(compiler);
        }
        let mut expected = None;
        let mut found = 0;
        for line in text.lines() {
            if let Some((_, failure)) = line.split_once("FAIL: ") {
                assert!(expected.is_none(), "multiple failures: {path}");
                let count = failure
                    .trim()
                    .strip_suffix(" problem(s):")
                    .unwrap_or_else(|| panic!("non-parity gate failure: {line}"));
                expected = Some(count.parse::<usize>().expect("problem count"));
            }
            let Some((_, row)) = line.split_once("  - row `") else {
                continue;
            };
            let (site, row) = row.split_once("` `").expect("site/config diagnostic");
            let (actual_config, row) = row
                .split_once("` reads '")
                .expect("configuration diagnostic");
            assert_eq!(actual_config, config, "configuration differs: {path}");
            let (old, row) = row
                .split_once("', measured '")
                .expect("old/measured diagnostic");
            let (new, _) = row
                .rsplit_once("' (run `python3 scripts/check-simd-asm.py --write-doc`)")
                .expect("complete native generator diagnostic");
            assert_ne!(old, new);
            assert!(
                updates.insert((site.to_owned(), config.to_owned())),
                "repeated cell"
            );
            assert!(
                [site, config, old, new]
                    .iter()
                    .all(|value| !value.contains(['\t', '\r', '\n']))
            );
            output.push(format!("{site}\t{config}\t{old}\t{new}"));
            found += 1;
        }
        if let Some(expected) = expected {
            assert!(expected > 0);
            assert_eq!(found, expected, "unaccounted gate problem in {path}");
        } else {
            assert_eq!(found, 0);
            assert!(
                text.contains(&format!(
                    "OK: {sites} site(s) measured on 1 configurations (partial configuration shard)"
                )),
                "missing successful terminal result: {path}"
            );
        }
    }
    println!("site\tconfig\tprevious\tmeasured");
    output.sort();
    for line in &output {
        println!("{line}");
    }
    eprintln!(
        "producer={}; configurations={}; parity_updates={}; original failed gates remain failed",
        producer.unwrap(),
        configurations.len(),
        output.len()
    );
}

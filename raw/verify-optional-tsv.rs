// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{BufRead, BufReader};

fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1).expect("TSV path");
    let mut lines = BufReader::new(std::fs::File::open(path)?).lines();
    assert_eq!(lines.next().expect("header")?, "?s\t?v");
    let mut seen = vec![false; 200_000];
    let mut count = 0;
    for line in lines {
        let line = line?;
        let (subject, value) = line.split_once('\t').expect("two bound cells");
        let index: usize = subject.strip_prefix("<http://example.org/s").and_then(|v| v.strip_suffix('>')).expect("exact subject IRI").parse().expect("subject integer suffix");
        assert!(index < seen.len() && !seen[index], "each subject exactly once");
        assert_eq!(value, format!("\"{index}\"^^<http://www.w3.org/2001/XMLSchema#integer>"), "exact integer value for subject {index}");
        seen[index] = true;
        count += 1;
    }
    assert_eq!(count, 200_000);
    assert!(seen.into_iter().all(|v| v));
    println!("PASS: all 200000 subject/integer pairs, both cells bound and exact, no duplicates");
    Ok(())
}

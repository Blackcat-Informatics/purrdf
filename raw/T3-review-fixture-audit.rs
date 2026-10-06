// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Read-only independent extraction from the primary document.
use std::fs;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = fs::read_to_string(&args[1]).unwrap();
    let section = text.split("Figure 9: ML-DSA-65-ES256").nth(1).unwrap()
        .split("Figure 10: ML-DSA-65-Ed25519").next().unwrap();
    let stripped = section.lines().filter(|line| {
        !line.contains("Prabel, et al.") && !line.contains("Internet-Draft")
    }).collect::<Vec<_>>().join("\n");
    let mut literals = Vec::new();
    let mut tail = stripped.as_str();
    while let Some(start) = tail.find("h'") {
        tail = &tail[start + 2..];
        let end = tail.find('\'').unwrap();
        let value: String = tail[..end].chars().filter(|c| !c.is_whitespace()).collect();
        assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(value.len() % 2, 0);
        literals.push(value);
        tail = &tail[end + 1..];
    }
    let lengths: Vec<_> = literals.iter().map(|s| s.len() / 2).collect();
    assert_eq!(lengths, [32,32,8,1984,64,8,0,29,127,8,29,3373]);
    assert_eq!(literals[2], literals[5]);
    assert_eq!(literals[2], literals[9]);
    assert_eq!(literals[7], literals[10]);
    assert_eq!(literals[4], literals[0].clone() + &literals[1]);
    let fixture = fs::read_to_string(&args[2]).unwrap();
    let rows: Vec<_> = fixture.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
    assert_eq!(rows.len(), 1);
    let fields: Vec<_> = rows[0].split('\t').collect();
    assert_eq!(fields.len(), 8);
    for (field, index) in fields.iter().zip([0,1,2,3,4,7,8,11]) {
        assert_eq!(*field, literals[index], "primary literal {index}");
        println!("PASS: source literal {index}, {} exact bytes", field.len()/2);
    }
    println!("PASS: 12 primary literal lengths {:?}; all 8 retained fixture fields exact", lengths);
}

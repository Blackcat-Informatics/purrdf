// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One-time primary ACVP input/output import; no implementation is an oracle.
use purrdf_lex::json::{self, Value};
use purrdf_testkit::vectors::Recorder;
use std::{env, fs, path::Path};

fn read(path: &Path) -> Value {
    json::read(&fs::read_to_string(path).unwrap()).unwrap()
}
fn groups(document: &Value) -> &[Value] {
    document.get("testGroups").unwrap().as_array().unwrap()
}
fn number(value: &Value, name: &str) -> u64 {
    value.get(name).unwrap().as_u64().unwrap()
}
fn text(value: &Value, name: &str) -> String {
    let hex = value.get(name).unwrap().as_str().unwrap();
    assert!(purrdf_hash::hex::decode(hex).is_ok());
    if hex.is_empty() {
        "\\0".to_owned()
    } else {
        hex.to_ascii_lowercase()
    }
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3);
    let raw = Path::new(&args[1]);
    let destination = Path::new(&args[2]);
    for (operation, selections) in [
        ("keyGen", vec![2]),
        ("sigGen", vec![3, 15]),
        ("sigVer", vec![3]),
    ] {
        let prompt = read(&raw.join(format!("ML-DSA-{operation}-FIPS204-prompt.json")));
        let expected = read(&raw.join(format!("ML-DSA-{operation}-FIPS204-expectedResults.json")));
        let mut recorder = Recorder::new();
        recorder
            .comment("NIST ACVP FIPS 204 official answers, ML-DSA-65. See PROVENANCE.md.")
            .unwrap();
        recorder
            .header(
                "oracle",
                "usnistgov/ACVP-Server 975de31eb83d87039ec88934fdc47d8c312b892d",
            )
            .unwrap();
        for group in groups(&prompt)
            .iter()
            .filter(|group| selections.contains(&number(group, "tgId")))
        {
            assert_eq!(
                group.get("parameterSet").unwrap().as_str(),
                Some("ML-DSA-65")
            );
            if operation != "keyGen" {
                assert_eq!(
                    group.get("signatureInterface").unwrap().as_str(),
                    Some("external")
                );
                assert_eq!(group.get("preHash").unwrap().as_str(), Some("pure"));
            }
            let gid = number(group, "tgId");
            let answers = groups(&expected)
                .iter()
                .find(|group| number(group, "tgId") == gid)
                .unwrap();
            for input in group.get("tests").unwrap().as_array().unwrap() {
                let id = number(input, "tcId");
                let answer = answers
                    .get("tests")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|answer| number(answer, "tcId") == id)
                    .unwrap();
                let mut fields = vec![gid.to_string(), id.to_string()];
                match operation {
                    "keyGen" => {
                        fields.extend([text(input, "seed"), text(answer, "pk"), text(answer, "sk")])
                    }
                    "sigGen" => {
                        fields.extend([
                            text(input, "sk"),
                            text(input, "message"),
                            text(input, "context"),
                        ]);
                        fields.push(if gid == 3 {
                            "00".repeat(32)
                        } else {
                            text(input, "rnd")
                        });
                        fields.push(text(answer, "signature"));
                    }
                    "sigVer" => {
                        fields.extend([
                            text(input, "pk"),
                            text(input, "message"),
                            text(input, "context"),
                            text(input, "signature"),
                        ]);
                        fields.push(match answer.get("testPassed").unwrap() {
                            Value::Bool(passed) => passed.to_string(),
                            _ => panic!("expected Boolean"),
                        });
                    }
                    _ => unreachable!(),
                }
                recorder.record(&fields).unwrap();
            }
        }
        let file = destination.join(format!("{operation}.txt"));
        fs::write(&file, recorder.render()).unwrap();
        println!("{}", file.display());
    }
}

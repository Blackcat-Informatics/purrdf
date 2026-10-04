// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generate or check separately distributed canonical lexicon data artifacts.
//! Run `cargo run -p purrdf-text --example gen_lexicons -- --check` to verify.

use purrdf_hash::{blake3, hex};
use purrdf_lex::json::{self, Object, Value};
use purrdf_text::segment::{Dictionary, PROFILE_ID};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write as _;
use std::path::Path;

const REVISION: &str = "21d1eb0f306e1141c10931e914dfc038c06121da";
const SOURCES: [(&str, usize); 5] = [
    ("cjdict", 315_964),
    ("thaidict", 26_383),
    ("laodict", 30_550),
    ("khmerdict", 81_028),
    ("burmesedict", 41_120),
];

fn object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(entries.into_iter().collect::<Object>())
}
fn publish(path: &Path, bytes: &[u8], check: bool) -> Result<(), Box<dyn Error>> {
    if check {
        if std::fs::read(path)? != bytes {
            return Err(format!("generated lexicon drift: {}", path.display()).into());
        }
    } else {
        std::fs::write(path, bytes)?;
    }
    Ok(())
}
fn generate(
    root: &Path,
    name: &str,
    expected: usize,
    check: bool,
) -> Result<Value, Box<dyn Error>> {
    let source = std::fs::read(root.join("icu-78.3").join(format!("{name}.txt")))?;
    let text = std::str::from_utf8(&source)?.trim_start_matches('\u{feff}');
    let notice = text
        .lines()
        .take_while(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    publish(
        &root.join("notices").join(format!("{name}.txt")),
        notice.as_bytes(),
        check,
    )?;
    let mut entries: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    let mut count = 0usize;
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (word, cost) = if name == "cjdict" {
            let (word, cost) = line
                .split_once('\t')
                .ok_or("CJK dictionary line has no cost")?;
            (word, cost.parse::<u32>()?)
        } else {
            (line, 1)
        };
        let key = Dictionary::canonical_key(word)?;
        entries
            .entry(key)
            .or_default()
            .push((word.to_owned(), cost));
        count += 1;
    }
    if count != expected {
        return Err(format!("{name}: expected {expected} raw entries, found {count}").into());
    }
    let collisions: Vec<Value> = entries
        .iter()
        .filter(|(_, variants)| variants.len() > 1)
        .map(|(key, variants)| {
            object([
                ("canonical", Value::from(key)),
                (
                    "cost",
                    Value::from(
                        variants
                            .iter()
                            .map(|(_, cost)| *cost)
                            .min()
                            .expect("nonempty variants"),
                    ),
                ),
                (
                    "sources",
                    Value::Array(
                        variants
                            .iter()
                            .map(|(word, cost)| {
                                object([("word", Value::from(word)), ("cost", Value::from(*cost))])
                            })
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let collision_groups = collisions.len();
    let collision_text = json::write_pretty(&Value::Array(collisions)) + "\n";
    publish(
        &root
            .join("artifacts")
            .join(format!("{name}.collisions.json")),
        collision_text.as_bytes(),
        check,
    )?;
    let dictionary = Dictionary::with_costs(entries.into_iter().map(|(word, variants)| {
        (
            word,
            variants
                .into_iter()
                .map(|(_, cost)| cost)
                .min()
                .expect("nonempty variants"),
        )
    }))?;
    let bytes = dictionary.artifact_bytes();
    let identity = dictionary.artifact_fingerprint();
    let loaded = Dictionary::from_artifact(identity, &bytes)?;
    if loaded != dictionary {
        return Err("artifact round trip changed vocabulary".into());
    }
    let physical = hex::encode(&identity);
    let filename = format!("{name}-{physical}.cbor");
    publish(&root.join("artifacts").join(&filename), &bytes, check)?;
    println!(
        "{name}: {count} -> {} entries, {collision_groups} collision groups, {} bytes, {physical}",
        dictionary.len(),
        bytes.len()
    );
    Ok(object([
        ("name", Value::from(name)),
        ("source", Value::from(format!("icu-78.3/{name}.txt"))),
        (
            "source_url",
            Value::from(format!(
                "https://raw.githubusercontent.com/unicode-org/icu/{REVISION}/icu4c/source/data/brkitr/dictionaries/{name}.txt"
            )),
        ),
        (
            "source_blake3",
            Value::from(hex::encode(blake3::hash(&source).as_bytes())),
        ),
        ("source_entries", Value::from(count)),
        ("canonical_entries", Value::from(dictionary.len())),
        ("collapsed_entries", Value::from(count - dictionary.len())),
        ("collision_groups", Value::from(collision_groups)),
        ("collisions", Value::from(format!("{name}.collisions.json"))),
        ("artifact", Value::from(filename)),
        ("artifact_bytes", Value::from(bytes.len())),
        ("physical_blake3", Value::from(physical)),
        (
            "semantic_blake3",
            Value::from(hex::encode(&dictionary.fingerprint())),
        ),
        (
            "notices",
            Value::Array(vec![
                Value::from("../notices/Unicode-3.0.txt"),
                Value::from(format!("../notices/{name}.txt")),
            ]),
        ),
    ]))
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 || args.first().is_some_and(|arg| arg != "--check") {
        return Err("usage: gen_lexicons [--check]".into());
    }
    let check = !args.is_empty();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("lexicons");
    let mut artifacts = Vec::new();
    for (name, count) in SOURCES {
        artifacts.push(generate(&root, name, count, check)?);
    }
    let mut identifiers = String::from(
        "// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>\n// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0\n\n//! Generated artifact identities; regenerate with `gen_lexicons`.\n\n",
    );
    for artifact in &artifacts {
        let name = artifact["name"]
            .as_str()
            .ok_or("artifact name")?
            .to_ascii_uppercase();
        for (suffix, key) in [
            ("PHYSICAL", "physical_blake3"),
            ("SEMANTIC", "semantic_blake3"),
        ] {
            let hash = artifact[key].as_str().ok_or("artifact hash")?;
            writeln!(identifiers, "/// Pinned {name} {suffix} BLAKE3 identity.\npub const {name}_{suffix}: &str =\n    \"{hash}\";")
                .expect("String writes cannot fail");
        }
    }
    publish(
        &root
            .parent()
            .ok_or("crate root")?
            .join("src/segment/baseline.rs"),
        identifiers.as_bytes(),
        check,
    )?;
    let manifest = object([
        ("format", Value::from("purrdf-lexicon-bundle/v1")),
        ("icu_revision", Value::from(REVISION)),
        ("law", Value::from(PROFILE_ID)),
        (
            "normalization_collisions",
            Value::from("minimum-cost; all raw variants recorded"),
        ),
        ("artifacts", Value::Array(artifacts)),
    ]);
    publish(
        &root.join("artifacts/manifest.json"),
        (json::write_pretty(&manifest) + "\n").as_bytes(),
        check,
    )
}

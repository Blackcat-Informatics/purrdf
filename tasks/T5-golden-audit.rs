// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Read-only audit of regenerated original entailment goldens against HEAD.
use std::{collections::BTreeMap, error::Error, fs, process::Command};
fn normalized(text: &str) -> Result<(String, Vec<(String, u64)>), Box<dyn Error>> {
    let mut normalized = String::new();
    let mut regime = String::new();
    let mut budgets = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if line == "#" && lines.peek() == Some(&"# CURRENT CONNECTED RULE EVALUATION TRANSITION — calculus v2.") {
            let mut closed = false;
            for comment in lines.by_ref() {
                if !comment.starts_with('#') { return Err("non-comment in transition header".into()); }
                if comment == "# END CONNECTED RULE EVALUATION TRANSITION." { closed = true; break; }
            }
            if !closed { return Err("unterminated transition header".into()); }
            continue;
        }
        if let Some(name) = line.strip_prefix("=== regime ").and_then(|x| x.strip_suffix(" ===")) { regime = name.to_owned(); }
        if line.starts_with("  contract-hash: ") {
            normalized.push_str("  contract-hash: <versioned-contract>\n");
        } else if let Some(rest) = line.strip_prefix("  budget: join-steps=") {
            let (steps, remainder) = rest.split_once(' ').ok_or("malformed budget line")?;
            budgets.push((regime.clone(), steps.parse()?));
            normalized.push_str(&format!("  budget: join-steps=<work> {remainder}\n"));
        } else { normalized.push_str(line); normalized.push('\n'); }
    }
    Ok((normalized, budgets))
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut files: Vec<_> = fs::read_dir("crates/entail/tests/goldens")?.map(|x| x.map(|e| e.path())).collect::<Result<_, _>>()?;
    files.sort();
    let mut summaries: BTreeMap<String, (u64, u64, usize, usize)> = BTreeMap::new();
    let mut count = 0;
    for path in files {
        if path.extension().and_then(|x| x.to_str()) != Some("golden") { continue; }
        let baseline = Command::new("git").args(["show", &format!("HEAD:{}", path.display())]).output()?;
        if !baseline.status.success() { return Err(format!("baseline missing: {}", path.display()).into()); }
        let (old, old_budgets) = normalized(&String::from_utf8(baseline.stdout)?)?;
        let (new, new_budgets) = normalized(&fs::read_to_string(&path)?)?;
        if old != new { return Err(format!("unexpected non-contract/non-work payload change: {}", path.display()).into()); }
        if old_budgets.len() != new_budgets.len() { return Err("budget section mismatch".into()); }
        for ((old_regime, old_steps), (new_regime, new_steps)) in old_budgets.into_iter().zip(new_budgets) {
            if old_regime != new_regime { return Err("regime order mismatch".into()); }
            let expected_delta = match old_regime.as_str() { "D" => 32, "OWL-RL" => 43, _ => 0 };
            if i128::from(new_steps) - i128::from(old_steps) != expected_delta {
                println!("EXTRA {} {old_regime}: {old_steps} -> {new_steps}, delta={}", path.display(), i128::from(new_steps) - i128::from(old_steps));
            }
            let summary = summaries.entry(old_regime).or_default();
            summary.0 += old_steps; summary.1 += new_steps;
            summary.2 += usize::from(new_steps > old_steps); summary.3 += usize::from(new_steps < old_steps);
        }
        count += 1;
    }
    assert_eq!(count, 142);
    println!("PASS {count} goldens: only contract-hash, join-steps and generated transition comment block differ; every other byte unchanged");
    for (regime, (old, new, up, down)) in summaries { println!("{regime}: old_steps={old} new_steps={new} increased_sections={up} decreased_sections={down}"); }
    Ok(())
}

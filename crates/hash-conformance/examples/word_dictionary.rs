// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Offline deterministic EFF subset selection. Arguments: source, cl100k,
//! o200k, output directory. No network, model or runtime tokenizer dependency.

#[path = "word_dictionary/bpe.rs"]
mod bpe;

use std::error::Error;
use std::fmt::Write as _;
use std::path::Path;

use bpe::Bpe;
use purrdf_hash::{
    blake3, hex,
    word_id::{self, Reference},
};

type Failure = Box<dyn Error>;

const SOURCE: &str = "5d0101780829f897a3a2526c11d04913e899648ee9d8ff14489e7ea9193e302c";
const CL100K: &str = "65afa6a4947e558c7e52ca1dc682285a6aff053a60ce9ba951bf154da0cb980f";
const O200K: &str = "77500c1d2b9e0f606d2ab99ff39e9ebaed9221b4531c6d83a503c36def0b8be8";

struct Word {
    text: String,
    cost: [usize; 2],
    neighbours: [Vec<usize>; 2],
    degree: [usize; 2],
    retained: bool,
}

// Byte Levenshtein distance, capped at three. ASCII source words have at
// most nine letters. Insertions, deletions and substitutions cost one;
// transposition is two edits, not a separate Damerau operation.
fn distance(left: &[u8], right: &[u8]) -> usize {
    if left.len().abs_diff(right.len()) > 2 {
        return 3;
    }
    let mut previous = [0; 10];
    for (index, cell) in previous.iter_mut().enumerate() {
        *cell = index;
    }
    for (row, a) in left.iter().enumerate() {
        let mut next = [0; 10];
        next[0] = row + 1;
        for (column, b) in right.iter().enumerate() {
            next[column + 1] = (previous[column + 1] + 1)
                .min(next[column] + 1)
                .min(previous[column] + usize::from(a != b));
        }
        previous = next;
    }
    previous[right.len()].min(3)
}

fn main() -> Result<(), Failure> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err("expected EFF source, cl100k ranks, o200k ranks, output directory".into());
    }
    let source = std::fs::read(&args[0])?;
    if hex::encode(blake3::hash(&source).as_bytes()) != SOURCE {
        return Err("EFF source digest mismatch".into());
    }
    let tokenizers = [
        Bpe::read(Path::new(&args[1]), CL100K)?,
        Bpe::read(Path::new(&args[2]), O200K)?,
    ];
    for (tokenizer, name) in tokenizers.iter().zip(["cl100k", "o200k"]) {
        tokenizer.source_vectors(name)?;
    }
    let mut words = Vec::new();
    let mut exclusions = String::from("word\treason\n");
    let mut source_count = 0;
    for line in std::str::from_utf8(&source)?.lines() {
        let (_, word) = line.split_once('\t').ok_or("invalid EFF line")?;
        source_count += 1;
        if !(3..=9).contains(&word.len()) || !word.bytes().all(|byte| byte.is_ascii_lowercase()) {
            writeln!(
                exclusions,
                "{word}\tnot lowercase ASCII letters of length three through nine"
            )?;
            continue;
        }
        let piece = format!(" {word}");
        words.push(Word {
            text: word.to_owned(),
            cost: [
                tokenizers[0].piece(piece.as_bytes())?.len(),
                tokenizers[1].piece(piece.as_bytes())?.len(),
            ],
            neighbours: [Vec::new(), Vec::new()],
            degree: [0; 2],
            retained: true,
        });
    }
    if source_count != 7776
        || words
            .windows(2)
            .any(|pair| pair[0].text >= pair[1].text || pair[1].text.starts_with(&pair[0].text))
    {
        return Err("EFF count/order/uniqueness/prefix invariant failed".into());
    }
    for left in 0..words.len() {
        for right in left + 1..words.len() {
            let distance = distance(words[left].text.as_bytes(), words[right].text.as_bytes());
            if (1..=2).contains(&distance) {
                words[left].neighbours[distance - 1].push(right);
                words[right].neighbours[distance - 1].push(left);
            }
        }
    }
    for word in &mut words {
        word.degree = word.neighbours.each_ref().map(Vec::len);
    }
    let original: [usize; 2] =
        std::array::from_fn(|kind| words.iter().map(|word| word.degree[kind]).sum::<usize>() / 2);
    let mut decisions =
        String::from("step\tword\tdistance_one_degree\tdistance_two_degree\tcl100k\to200k\n");
    for step in 0..words.len() - 4096 {
        let remove = words
            .iter()
            .enumerate()
            .filter(|(_, word)| word.retained)
            .max_by_key(|(_, word)| {
                (
                    word.degree[0],
                    word.degree[1],
                    word.cost[0].max(word.cost[1]),
                    word.cost.iter().sum::<usize>(),
                    word.text.len(),
                    &word.text,
                )
            })
            .map(|(index, _)| index)
            .ok_or("selection exhausted")?;
        let word = &words[remove];
        writeln!(
            decisions,
            "{}\t{}\t{}\t{}\t{}\t{}",
            step + 1,
            word.text,
            word.degree[0],
            word.degree[1],
            word.cost[0],
            word.cost[1]
        )?;
        words[remove].retained = false;
        for kind in 0..2 {
            for ordinal in 0..words[remove].neighbours[kind].len() {
                let neighbour = words[remove].neighbours[kind][ordinal];
                if words[neighbour].retained {
                    words[neighbour].degree[kind] -= 1;
                }
            }
        }
    }
    let selected: Vec<_> = words.iter().filter(|word| word.retained).collect();
    let mut alphabet = String::new();
    let mut costs = String::from("word\tcl100k\to200k\tdistance_one_degree\tdistance_two_degree\n");
    for word in &selected {
        writeln!(alphabet, "{}", word.text)?;
        writeln!(
            costs,
            "{}\t{}\t{}\t{}\t{}",
            word.text, word.cost[0], word.cost[1], word.degree[0], word.degree[1]
        )?;
    }
    let mut report = format!(
        "source_words=7776\ncandidate_words={}\nselected_words=4096\nalphabet_blake3={}\noriginal_distance_one_pairs={}\noriginal_distance_two_pairs={}\n",
        words.len(),
        hex::encode(blake3::hash(alphabet.as_bytes()).as_bytes()),
        original[0],
        original[1]
    );
    for kind in 0..2 {
        writeln!(
            report,
            "selected_distance_{}_pairs={}",
            kind + 1,
            selected.iter().map(|word| word.degree[kind]).sum::<usize>() / 2
        )?;
    }
    for (index, name) in ["cl100k", "o200k"].iter().enumerate() {
        let total = selected.iter().map(|word| word.cost[index]).sum::<usize>();
        let maximum = selected
            .iter()
            .map(|word| word.cost[index])
            .max()
            .ok_or("empty alphabet")?;
        writeln!(
            report,
            "{name}_space_word_token_sum={total}\n{name}_space_word_token_max={maximum}"
        )?;
    }
    let output = Path::new(&args[3]);
    std::fs::create_dir_all(output)?;
    std::fs::write(output.join("alphabet.txt"), alphabet)?;
    std::fs::write(output.join("selection.tsv"), decisions)?;
    std::fs::write(output.join("exclusions.tsv"), exclusions)?;
    std::fs::write(output.join("word-costs.tsv"), costs)?;
    std::fs::write(output.join("selection-summary.txt"), report)?;
    if selected
        .iter()
        .enumerate()
        .any(|(index, word)| word_id::dictionary_word(index as u16) != Some(word.text.as_str()))
    {
        return Err("selection differs from the frozen runtime alphabet; a changed alphabet requires a new named scheme".into());
    }
    let mut measurements = String::from(
        "payload_words\tform\treferences\tmean_bytes\tcl100k_mean\tcl100k_min\tcl100k_max\to200k_mean\to200k_min\to200k_max\n",
    );
    for payload in 1..=22 {
        let mut total_bytes = 0;
        let mut totals = [0_usize; 2];
        let mut minima = [usize::MAX; 2];
        let mut maxima = [0_usize; 2];
        for index in 0_u64..4096 {
            let digest = hex::Digest32::new(*blake3::hash(&index.to_le_bytes()).as_bytes());
            let reference = Reference::from_digest(digest, payload)?;
            let text = reference.to_text()?;
            total_bytes += text.len();
            for (kind, tokenizer) in tokenizers.iter().enumerate() {
                let count = tokenizer.reference(&text)?.len();
                totals[kind] += count;
                minima[kind] = minima[kind].min(count);
                maxima[kind] = maxima[kind].max(count);
            }
        }
        writeln!(
            measurements,
            "{payload}\t{}\t4096\t{:.6}\t{:.6}\t{}\t{}\t{:.6}\t{}\t{}",
            if payload == 22 { "b3w1" } else { "b3p1" },
            total_bytes as f64 / 4096.0,
            totals[0] as f64 / 4096.0,
            minima[0],
            maxima[0],
            totals[1] as f64 / 4096.0,
            minima[1],
            maxima[1]
        )?;
    }
    std::fs::write(output.join("reference-token-costs.tsv"), measurements)?;
    Ok(())
}

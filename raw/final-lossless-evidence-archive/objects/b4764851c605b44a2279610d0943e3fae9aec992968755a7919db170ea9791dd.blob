// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Read emitted IR/assembly without rewriting operations, offsets or symbols.
use std::{env, fs, path::Path};

fn selected(name: &str) -> bool {
    ["eval_node", "eval_evaluated", "eval_lateral", "eval_application", "eval_substituted",
     "eval_correlated", "eval_filter", "eval_extend", "eval_project", "eval_dedup",
     "eval_slice", "eval_union", "eval_graph", "eval_group", "eval_aggregate", "eval_join",
     "eval_minus", "clone_tree", "validate", "from_algebra", "walk_expressions",
     "GraphPattern", "NodeRef", "qualification_parse", "query_impl",
     "PyQuadStore::query", "PyQuadStore>::query", "presentation::settled", "SparqlParser::parse",
     "parser::machine::Parser", "parser::Parser<", "parse_query_split", "aggregate_numeric_cost", "aggregate_numeric_tail_cost", "fold_numeric", "sum_chain", "SumChain", "charge_exact_numeric"]
        .iter().any(|needle| name.contains(needle))
}

fn inspect(path: &Path, destination: &Path, ordinal: &mut usize) {
    let source = fs::read_to_string(path).unwrap();
    let assembly = fs::read_to_string(path.with_extension("s")).unwrap();
    let lines: Vec<_> = source.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        if !line.starts_with("define ") { continue; }
        let name = lines[..index].iter().rev().find(|line| line.starts_with("; ") && !line.starts_with("; Function Attrs:")).unwrap_or(line);
        if !selected(name) { continue; }
        let end = index + lines[index..].iter().position(|line| *line == "}").unwrap();
        let body = &lines[index..=end];
        let mut opcodes = std::collections::BTreeMap::new();
        for line in body {
            let line = line.trim();
            let instruction = line.split_once(" = ").map_or(line, |(_, rhs)| rhs);
            let opcode = instruction.split_whitespace().next().unwrap_or("");
            if !opcode.is_empty() && !opcode.starts_with(';') && !opcode.ends_with(':') {
                *opcodes.entry(opcode).or_insert(0usize) += 1;
            }
        }
        let at = line.find('@').unwrap() + 1;
        let symbol = &line[at..line[at..].find('(').unwrap()+at];
        let symbol = symbol.trim_matches('"');
        let label = format!("{symbol}:");
        let asm_lines: Vec<_> = assembly.lines().collect();
        let asm_start = asm_lines.iter().position(|line| *line == label);
        *ordinal += 1;
        println!("SECTION {} {}:{} {}\nABI {}\nOPCODES {:?}", *ordinal, path.display(), index+1, name, line, opcodes);
        for line in body {
            if line.contains("alloca ") || line.contains(" call ") || line.trim_start().starts_with("invoke ") {
                println!("IR {line}");
            }
        }
        let mut output = format!("{}:{}\n{}\n", path.display(), index+1, name);
        output.push_str(&body.join("\n"));
        if let Some(start) = asm_start {
            let end = start + asm_lines[start..].iter().position(|line| line.starts_with(".Lfunc_end")).unwrap();
            output.push_str("\n\nASSEMBLY\n");
            output.push_str(&asm_lines[start..=end].join("\n"));
            println!("ASSEMBLY {}:{}", path.with_extension("s").display(), start+1);
            for line in asm_lines[start..=end].iter().take(30) { println!("ASM {line}"); }
        } else { println!("ASSEMBLY no emitted function label"); }
        fs::write(destination.join(format!("section-{:04}.txt", *ordinal)), output).unwrap();
    }
}

fn main() {
    let arguments: Vec<_> = env::args().collect();
    assert_eq!(arguments.len(), 3, "IR directory and output directory");
    let destination = Path::new(&arguments[2]);
    fs::create_dir_all(destination).unwrap();
    let mut paths: Vec<_> = fs::read_dir(&arguments[1]).unwrap().map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ll")).collect();
    paths.sort();
    assert!(!paths.is_empty(), "actual emitted IR required");
    let mut count = 0;
    for path in paths { inspect(&path, destination, &mut count); }
    assert!(count > 0, "named attributable functions required");
    println!("named sections {count}");
}

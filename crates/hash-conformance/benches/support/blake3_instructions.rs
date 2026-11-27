// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed-work retired instruction comparison; an unavailable PMU refuses.

use purrdf_hash::Backend as _;
use purrdf_hash::blake3::{self, Backend, DeriveKeyContext, Hash};
use purrdf_testkit::bench::counter::{CounterCommand, CounterControl, collect_instructions};
use std::{env, hint::black_box, path::Path};

const MESSAGES: usize = 16_384;
const WIDTH: usize = 16;
const LENGTHS: &[usize] = &[
    16, 32, 63, 64, 65, 100, 128, 256, 512, 1024, 1025, 2048, 2049, 3072, 3073, 4096,
];
const CONTEXT: &str = "PurRDF BLAKE3 benchmark application context";

fn data(len: usize) -> Vec<u8> {
    (0..len).map(|i| i.wrapping_mul(29) as u8).collect()
}

fn vary(i: usize) -> u8 {
    (i as u8).wrapping_add((i >> 8) as u8)
}

fn individual(input: &[u8], backend: Option<Backend>) -> Hash {
    if let Some(path) = backend {
        path.hash(input).expect("forced backend available")
    } else {
        blake3::hash(input)
    }
}

fn single(mut input: Vec<u8>, digest: impl Fn(&[u8]) -> [u8; 32]) -> u32 {
    let mut witness = 0_u32;
    for i in 0..MESSAGES {
        input[0] = vary(i);
        let byte = black_box(digest(black_box(input.as_slice())))[0];
        witness = witness.wrapping_add(u32::from(byte));
    }
    witness
}

fn grouped(mut rows: [Vec<u8>; WIDTH], backend: Option<Backend>, lanes: bool) -> u32 {
    let mut witness = 0_u32;
    let mut output = [Hash::ZERO; WIDTH];
    for start in (0..MESSAGES).step_by(WIDTH) {
        for (lane, row) in rows.iter_mut().enumerate() {
            row[0] = vary(start + lane);
        }
        let inputs = core::array::from_fn::<_, WIDTH, _>(|lane| rows[lane].as_slice());
        let inputs = black_box(&inputs);
        if lanes {
            if let Some(path) = backend {
                path.hash_batch(inputs, &mut output)
                    .expect("available")
                    .expect("valid batch");
            } else {
                blake3::hash_batch(inputs, &mut output).expect("valid batch");
            }
        } else {
            for (&input, slot) in inputs.iter().zip(&mut output) {
                *slot = individual(input, backend);
            }
        }
        for digest in black_box(&output) {
            witness = witness.wrapping_add(u32::from(digest.as_bytes()[0]));
        }
    }
    witness
}

fn sample(kind: &str, name: &str, len: usize) {
    let backend = Backend::all_available().find(|b| b.name() == name);
    assert!(name == "default" || backend.is_some(), "{name}");
    let one = data(len);
    let rows = core::array::from_fn(|_| data(len));
    let context = DeriveKeyContext::new(CONTEXT);
    let mut counter = CounterControl::required().expect("native retired-instruction counter");
    counter
        .command(CounterCommand::Enable)
        .expect("start counter");
    let witness = match (kind, backend) {
        ("single", None) => single(one, |data| *blake3::hash(data).as_bytes()),
        ("single", Some(path)) => {
            single(one, |data| *path.hash(data).expect("available").as_bytes())
        }
        ("derived", None) => single(one, |data| context.derive(data)),
        ("grouped" | "batch", _) => grouped(rows, backend, kind == "batch"),
        _ => panic!("unknown BLAKE3 sample kind {kind}"),
    };
    counter
        .command(CounterCommand::Disable)
        .expect("stop counter");
    println!("{witness}");
}

fn collect(directory: &Path) {
    let exe = env::current_exe().expect("benchmark executable");
    std::fs::create_dir(directory).expect("new output directory");
    println!("length,backend,kind,instructions_0,instructions_1,instructions_2,messages,witness");
    for &len in LENGTHS {
        let mut plain = None;
        for name in std::iter::once("default").chain(Backend::all_available().map(Backend::name)) {
            let mut kinds = vec!["single", "grouped", "batch"];
            if name == "default" {
                kinds.push("derived");
            }
            for kind in kinds {
                let row = collect_row(&exe, directory, kind, name, len);
                if kind != "derived" {
                    assert_eq!(&row.1, plain.get_or_insert_with(|| row.1.clone()));
                }
                let [a, b, c] = row.0;
                println!(
                    "{len},{name},{kind},{a},{b},{c},{MESSAGES},{}",
                    row.1.trim()
                );
            }
        }
    }
}

fn collect_row(
    exe: &Path,
    directory: &Path,
    kind: &str,
    name: &str,
    len: usize,
) -> ([u64; 3], String) {
    let mut counts = [0; 3];
    let mut expected = None;
    for (index, count) in counts.iter_mut().enumerate() {
        let dir = directory.join(format!("{len}-{name}-{kind}-{index}"));
        let args = [
            "--blake3-instruction-sample".into(),
            kind.into(),
            name.into(),
            len.to_string().into(),
        ];
        *count = collect_instructions(exe, &args, &dir)
            .expect("complete counter receipt")
            .instructions;
        let witness = std::fs::read_to_string(dir.join("workload.stdout")).expect("sample witness");
        assert_eq!(&witness, expected.get_or_insert_with(|| witness.clone()));
    }
    (counts, expected.expect("three retained samples"))
}

pub(super) fn dispatch() -> bool {
    let args: Vec<_> = env::args().skip(1).filter(|arg| arg != "--bench").collect();
    match args.as_slice() {
        [command, output] if command == "--blake3-instructions" => collect(Path::new(output)),
        [command, kind, name, len] if command == "--blake3-instruction-sample" => {
            sample(kind, name, len.parse().expect("message length"));
        }
        _ => return false,
    }
    true
}

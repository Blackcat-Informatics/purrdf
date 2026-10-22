// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{hint::black_box, path::Path, str::FromStr};
use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_testkit::bench::counter::{CountContext, CountReport, CountUnit, CounterCommand, CounterControl, collect_instructions};
use purrdf_testkit::rng::splitmix64_next;
use purrdf_xsd::{XsdDatatype, XsdValue, exact::{Decimal, DivisionPolicy, Integer, Rounding}};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const OPS: usize = 1024;
const ROWS: [&str; 7] = ["integer_add", "integer_mul", "decimal_add", "decimal_mul", "decimal_div", "parse_render", "to_f64"];

struct Inputs {
    integers: Vec<(Integer, Integer)>,
    decimals: Vec<(Decimal, Decimal)>,
    current_integers: Vec<(XsdValue, XsdValue)>,
    current_decimals: Vec<(XsdValue, XsdValue)>,
    bounded_integers: Vec<(bounded_xsd::XsdValue, bounded_xsd::XsdValue)>,
    bounded_decimals: Vec<(bounded_xsd::XsdValue, bounded_xsd::XsdValue)>,
    lexical: Vec<(String, String)>,
}

impl Inputs {
    fn new() -> Self {
        let mut seed = 0x5EED_E8AC;
        let small: Vec<_> = (0..OPS).map(|_| (i128::from(splitmix64_next(&mut seed) as i64 >> 8), i128::from(splitmix64_next(&mut seed) as i64 >> 8) | 1)).collect();
        let mut seed = 0xDEC1_3A15;
        let decimal = |seed: &mut u64| {
            let coefficient = (splitmix64_next(seed) % 1_000_000_000_000) as i64 - 500_000_000_000;
            let scale = (splitmix64_next(seed) % 7) as usize;
            let digits = format!("{:0>width$}", coefficient.unsigned_abs(), width = scale + 1);
            let sign = if coefficient < 0 { "-" } else { "" };
            let split = digits.len() - scale;
            if scale == 0 { format!("{sign}{digits}") } else { format!("{sign}{}.{}", &digits[..split], &digits[split..]) }
        };
        let lexical: Vec<_> = (0..OPS).map(|_| (decimal(&mut seed), decimal(&mut seed))).collect();
        let integers = small.iter().map(|&(a,b)| (Integer::from(a),Integer::from(b))).collect();
        let decimals: Vec<_> = lexical.iter().map(|(a,b)| (Decimal::from_str(a).unwrap(),Decimal::from_str(b).unwrap())).collect();
        assert!(decimals.iter().all(|(_,b)| !b.is_zero()));
        let current_integers = small.iter().map(|&(a,b)| (XsdValue::Integer {value:a.into(),datatype:XsdDatatype::Integer}, XsdValue::Integer {value:b.into(),datatype:XsdDatatype::Integer})).collect();
        let current_decimals = decimals.iter().map(|(a,b)| (XsdValue::Decimal(a.clone()),XsdValue::Decimal(b.clone()))).collect();
        let bounded_integers = small.iter().map(|&(a,b)| (bounded_xsd::XsdValue::Integer {value:a,datatype:bounded_xsd::XsdDatatype::Integer}, bounded_xsd::XsdValue::Integer {value:b,datatype:bounded_xsd::XsdDatatype::Integer})).collect();
        let bounded_decimals = lexical.iter().map(|(a,b)| (bounded_xsd::XsdValue::Decimal(bounded_xsd::numeric::parse_decimal(a).unwrap()),bounded_xsd::XsdValue::Decimal(bounded_xsd::numeric::parse_decimal(b).unwrap()))).collect();
        Self {integers,decimals,current_integers,current_decimals,bounded_integers,bounded_decimals,lexical}
    }
}

enum Output {
    Integer(Integer),
    Decimal(Decimal),
    Current(XsdValue),
    Bounded(bounded_xsd::XsdValue),
    Lexical(String),
    Float(f64),
}

impl Output {
    fn canonical(&self) -> String {
        match self {
            Self::Integer(v) => v.to_string(),
            Self::Decimal(v) => v.canonical_lexical(),
            Self::Current(XsdValue::Integer{value,..}) => value.to_string(),
            Self::Current(XsdValue::Decimal(v)) => v.canonical_lexical(),
            Self::Bounded(bounded_xsd::XsdValue::Integer{value,..}) => value.to_string(),
            Self::Bounded(bounded_xsd::XsdValue::Decimal(v)) => Decimal::from_str(&v.canonical_lexical()).unwrap().canonical_lexical(),
            Self::Lexical(v) => Decimal::from_str(v).unwrap().canonical_lexical(),
            Self::Float(v) => format!("f64:{:016x}",v.to_bits()),
            _ => panic!("unexpected numeric result family"),
        }
    }
}

type Kernel = fn(&Inputs, usize) -> Output;

fn select(family: &str, row: &str) -> Kernel {
    match (family, row) {
        ("exact", "integer_add") => |input, i| Output::Integer(black_box(&input.integers[i].0) + black_box(&input.integers[i].1)),
        ("exact", "integer_mul") => |input, i| Output::Integer(black_box(&input.integers[i].0) * black_box(&input.integers[i].1)),
        ("exact", "decimal_add") => |input, i| Output::Decimal(black_box(&input.decimals[i].0) + black_box(&input.decimals[i].1)),
        ("exact", "decimal_mul") => |input, i| Output::Decimal(black_box(&input.decimals[i].0).try_mul(black_box(&input.decimals[i].1)).unwrap()),
        ("exact", "decimal_div") => |input, i| Output::Decimal(black_box(&input.decimals[i].0).div(black_box(&input.decimals[i].1), DivisionPolicy::scale(18, Rounding::TowardZero)).unwrap()),
        ("exact_half_even", "decimal_div") => |input, i| Output::Decimal(black_box(&input.decimals[i].0).div(black_box(&input.decimals[i].1), DivisionPolicy::default()).unwrap()),
        ("exact", "parse_render") => |input, i| Output::Lexical(Decimal::from_str(black_box(&input.lexical[i].0)).unwrap().canonical_lexical()),
        ("exact", "to_f64") => |input, i| Output::Float(black_box(&input.decimals[i].0).to_f64()),
        ("value", "integer_add") => |input, i| Output::Current(purrdf_xsd::numeric_add(black_box(&input.current_integers[i].0), black_box(&input.current_integers[i].1)).unwrap()),
        ("value", "integer_mul") => |input, i| Output::Current(purrdf_xsd::numeric_mul(black_box(&input.current_integers[i].0), black_box(&input.current_integers[i].1)).unwrap()),
        ("value", "decimal_add") => |input, i| Output::Current(purrdf_xsd::numeric_add(black_box(&input.current_decimals[i].0), black_box(&input.current_decimals[i].1)).unwrap()),
        ("value", "decimal_mul") => |input, i| Output::Current(purrdf_xsd::numeric_mul(black_box(&input.current_decimals[i].0), black_box(&input.current_decimals[i].1)).unwrap()),
        ("value", "decimal_div") => |input, i| Output::Current(purrdf_xsd::numeric_div(black_box(&input.current_decimals[i].0), black_box(&input.current_decimals[i].1)).unwrap()),
        ("value", "parse_render") => |input, i| Output::Lexical(purrdf_xsd::numeric::parse_decimal(black_box(&input.lexical[i].0)).unwrap().canonical_lexical()),
        ("value", "to_f64") => |input, i| match black_box(&input.current_decimals[i].0) { purrdf_xsd::XsdValue::Decimal(v) => Output::Float(v.to_f64()), _ => unreachable!() },
        ("bounded", "integer_add") => |input, i| Output::Bounded(bounded_xsd::numeric_add(black_box(&input.bounded_integers[i].0), black_box(&input.bounded_integers[i].1)).unwrap()),
        ("bounded", "integer_mul") => |input, i| Output::Bounded(bounded_xsd::numeric_mul(black_box(&input.bounded_integers[i].0), black_box(&input.bounded_integers[i].1)).unwrap()),
        ("bounded", "decimal_add") => |input, i| Output::Bounded(bounded_xsd::numeric_add(black_box(&input.bounded_decimals[i].0), black_box(&input.bounded_decimals[i].1)).unwrap()),
        ("bounded", "decimal_mul") => |input, i| Output::Bounded(bounded_xsd::numeric_mul(black_box(&input.bounded_decimals[i].0), black_box(&input.bounded_decimals[i].1)).unwrap()),
        ("bounded", "decimal_div") => |input, i| Output::Bounded(bounded_xsd::numeric_div(black_box(&input.bounded_decimals[i].0), black_box(&input.bounded_decimals[i].1)).unwrap()),
        ("bounded", "parse_render") => |input, i| Output::Lexical(bounded_xsd::numeric::parse_decimal(black_box(&input.lexical[i].0)).unwrap().canonical_lexical()),
        ("bounded", "to_f64") => |input, i| match black_box(&input.bounded_decimals[i].0) { bounded_xsd::XsdValue::Decimal(v) => Output::Float(v.to_f64()), _ => unreachable!() },
        _ => panic!("unknown workload {family}/{row}"),
    }
}

fn run(input: &Inputs, kernel: Kernel, output: &mut Vec<Output>) {
    assert!(output.is_empty() && output.capacity() >= OPS);
    for i in 0..OPS {
        output.push(black_box(kernel(input, i)));
    }
}

fn sample(family: &str, row: &str, mode: &str) {
    assert!(ROWS.contains(&row));
    let input = Inputs::new();
    // Select once before warming/measuring: no family/string dispatch in the window.
    let kernel = black_box(select(family, row));
    let mut expected = Vec::with_capacity(OPS);
    // The historical API truncates; current public division rounds half-even.
    // Validate each against the matching law, without changing production law.
    let reference = if row == "decimal_div" && matches!(family,"value"|"exact_half_even") { "exact_half_even" } else { "bounded" };
    run(&input,select(reference,row),&mut expected);
    let expected: Vec<_> = expected.iter().map(Output::canonical).collect();
    let mut output = Vec::with_capacity(OPS);
    for _ in 0..3 {
        run(&input,kernel,&mut output);
        assert_eq!(output.iter().map(Output::canonical).collect::<Vec<_>>(),expected);
        output.clear();
    }
    match mode {
        "instructions" => {
            let mut control = CounterControl::required().unwrap();
            control.command(CounterCommand::Enable).unwrap();
            run(&input,kernel,&mut output);
            control.command(CounterCommand::Disable).unwrap();
        }
        "allocations" => {
            let window = WholeProcessWindow::open();
            run(&input,kernel,&mut output);
            let m = window.close();
            println!("ALLOC {} {} {} {}",m.allocations,m.requested_bytes,m.retained_bytes,m.peak_working_bytes);
        }
        _ => panic!("unknown measurement mode"),
    }
    assert_eq!(output.len(),OPS);
    for (i,(actual,expected)) in output.iter().zip(expected).enumerate() {
        let actual = actual.canonical();
        assert_eq!(actual,expected,"full answer {family}/{row}/{i}");
        println!("ANSWER {i} {actual}");
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("sample") if args.len() == 5 => sample(&args[2],&args[3],&args[4]),
        Some("collect") if args.len() == 3 => {
            let root = Path::new(&args[2]);
            std::fs::create_dir(root).unwrap();
            let executable = std::env::current_exe().unwrap();
            for row in ROWS {
                for family in ["bounded","exact","value"] {
                    let mut samples = Vec::new();
                    for sample in 0..10 {
                        let directory = root.join(format!("{row}-{family}-{sample}"));
                        let args = ["sample",family,row,"instructions"].map(Into::into);
                        let result = collect_instructions(&executable,&args,&directory).unwrap();
                        // Completed special-file endpoints have no replay data;
                        // retain every regular protocol/output receipt.
                        std::fs::remove_file(directory.join("control.fifo")).unwrap();
                        std::fs::remove_file(directory.join("ack.fifo")).unwrap();
                        samples.push(i64::try_from(result.instructions).unwrap());
                        println!("COUNT {row} {family} {sample} {} {} {}",result.instructions,result.enabled,result.running);
                    }
                    let law = if row == "decimal_div" && family == "value" { "half-even18" } else if row == "decimal_div" { "toward-zero18" } else { "exact" };
                    let context = CountContext { unit:CountUnit::Instructions, work:OPS as u64, threads:1, fixture:format!("small-seeds-5EED_E8AC-DEC1_3A15-{row}-{law}"), boundary:"1024-typed-common-enum-results-retained-preallocated-destination-excludes-validation-and-drop".to_owned() };
                    let report = CountReport::from_samples(context,samples,0x443).unwrap();
                    std::fs::write(root.join(format!("{row}-{family}.json")),report.to_json().unwrap()).unwrap();
                }
            }
        }
        _ => panic!("usage: sample FAMILY ROW instructions|allocations; collect NEW_DIRECTORY"),
    }
}

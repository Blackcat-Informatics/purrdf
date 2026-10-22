// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{hint::black_box, path::Path, str::FromStr};
use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_testkit::{exact::Rational, rng::splitmix64_next};
use purrdf_testkit::bench::counter::{CountContext,CountReport,CountUnit,CounterCommand,CounterControl,collect_instructions};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const CASES: [&str;22] = ["growth/40/positive","growth/40/negative","growth/400/positive","growth/400/negative","growth/4000/positive","growth/4000/negative","growth/40000/positive","growth/40000/negative","finite/40/positive","finite/40/negative","finite/400/positive","finite/400/negative","finite/4000/positive","finite/4000/negative","finite/40000/positive","finite/40000/negative","underflow/positive","underflow/negative","subnormal/positive","subnormal/negative","maxfinite/positive","maxfinite/negative"];

struct Input {
    before: reserved_xsd::exact::Decimal,
    after: purrdf_xsd::exact::Decimal,
    expected: u64,
    calls: usize,
}

fn digits(seed: &mut u64,len: usize) -> String {
    let mut text = String::with_capacity(len);
    while text.len()<len {
        let d = u8::try_from(splitmix64_next(seed)%10).unwrap();
        if text.is_empty() && d==0 {continue;}
        text.push(char::from(b'0'+d));
    }
    text
}

fn input(case: &str) -> Input {
    assert!(CASES.contains(&case));
    let negative = case.ends_with("negative");
    let sign = if negative {1_u64<<63} else {0};
    let coefficient = if negative {-1_i128} else {1};
    if case.starts_with("underflow/") || case.starts_with("subnormal/") {
        let (coefficient,scale,expected) = if case.starts_with("subnormal/") {(3*coefficient,324,sign|1)} else {(coefficient,u32::MAX,sign)};
        return Input {before:reserved_xsd::exact::Decimal::new(reserved_xsd::exact::Integer::from(coefficient),scale),after:purrdf_xsd::exact::Decimal::new(purrdf_xsd::exact::Integer::from(coefficient),scale),expected,calls:1024};
    }
    if case.starts_with("maxfinite/") {
        let value = if negative {-f64::MAX} else {f64::MAX};
        return Input {before:reserved_xsd::exact::Decimal::from_f64(value).unwrap(),after:purrdf_xsd::exact::Decimal::from_f64(value).unwrap(),expected:value.to_bits(),calls:1024};
    }
    let width: usize = case.split('/').nth(1).unwrap().parse().unwrap();
    let mut seed = 0x6E0_57A7E;
    let mut selected = None;
    for n in [40,400,4000,40000] {
        // Consume precisely the original growth bench's a, b, wide, decimal
        // and divisor draws, retaining its actual decimal input verbatim.
        for len in [n,n,2*n] {black_box(digits(&mut seed,len));}
        let text = format!("{}.{}",digits(&mut seed,n/2),digits(&mut seed,n/2));
        black_box(digits(&mut seed,n/2));
        if n==width {selected=Some(text);break;}
    }
    let original = selected.unwrap();
    let finite = case.starts_with("finite/");
    let text = if finite {format!("0.{}",original.replace('.',""))} else {original};
    let text = if negative {format!("-{text}")} else {text};
    let expected = if finite {
        // The full positive fraction lies between these exact 38-digit
        // decimal endpoints. Equal rounded endpoints certify every suffix;
        // this avoids rebuilding a quadratic 40000-digit oracle per sample.
        let fraction = text.trim_start_matches('-').strip_prefix("0.").unwrap();
        let prefix = fraction[..38].parse::<i128>().unwrap();
        let a=Rational::from_decimal(prefix*coefficient,38).to_f64().to_bits();
        let b=Rational::from_decimal((prefix+1)*coefficient,38).to_f64().to_bits();
        assert_eq!(a,b,"prefix endpoints do not certify unique rounding for {case}");a
    } else if width>=4000 {f64::INFINITY.to_bits()|sign}
    else {Rational::parse(&text).unwrap().to_f64().to_bits()};
    Input {before:reserved_xsd::exact::Decimal::from_str(&text).unwrap(),after:purrdf_xsd::exact::Decimal::from_str(&text).unwrap(),expected,calls:if finite && width>=4000 {32} else {1024}}
}

type Kernel=fn(&Input)->f64;
fn before(v:&Input)->f64 {black_box(&v.before).to_f64()}
fn after(v:&Input)->f64 {black_box(&v.after).to_f64()}

fn sample(family:&str,case:&str,mode:&str) {
    let input=input(case);
    let kernel:Kernel=black_box(match family {"before"=>before,"after"=>after,_=>panic!("unknown source")});
    for _ in 0..3 {assert_eq!(kernel(&input).to_bits(),input.expected,"warm {case}");}
    let mut output=Vec::with_capacity(input.calls);
    let mut run=||for _ in 0..input.calls {output.push(black_box(kernel(&input)));};
    match mode {
        "instructions"=>{let mut control=CounterControl::required().unwrap();control.command(CounterCommand::Enable).unwrap();run();control.command(CounterCommand::Disable).unwrap();}
        "allocations"=>{let window=WholeProcessWindow::open();run();let m=window.close();println!("ALLOC {} {} {} {}",m.allocations,m.requested_bytes,m.retained_bytes,m.peak_working_bytes);}
        _=>panic!("unknown measurement mode"),
    }
    assert_eq!(output.len(),input.calls);
    for (i,v) in output.iter().enumerate() {assert_eq!(v.to_bits(),input.expected,"{case}/{i}");println!("ANSWER {i} {:016x}",v.to_bits());}
}

fn main() {
    let args:Vec<_>=std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("sample") if args.len()==5=>sample(&args[2],&args[3],&args[4]),
        Some("collect") if args.len()==3=>{
            let root=Path::new(&args[2]);std::fs::create_dir(root).unwrap();
            let executable=std::env::current_exe().unwrap();
            for case in CASES {
                let mut reports=Vec::new();
                for family in ["before","after"] {
                    let mut samples=Vec::new();
                    for n in 0..10 {
                        let directory=root.join(format!("{}-{family}-{n}",case.replace('/',"-")));
                        let command=["sample",family,case,"instructions"].map(Into::into);
                        let count=collect_instructions(&executable,&command,&directory).unwrap();
                        std::fs::remove_file(directory.join("control.fifo")).unwrap();std::fs::remove_file(directory.join("ack.fifo")).unwrap();
                        samples.push(i64::try_from(count.instructions).unwrap());
                    }
                    let calls=if case.starts_with("finite/4000/") || case.starts_with("finite/40000/") {32} else {1024};
                    let context=CountContext {unit:CountUnit::Instructions,work:calls,threads:1,fixture:format!("decimal-binary64-{case}-nearest-even-v1"),boundary:"prepared-source-1024-or32-retained-float-results-validation-and-drop-excluded".into()};
                    let report=CountReport::from_samples(context,samples,0x443675).unwrap();
                    std::fs::write(root.join(format!("{}-{family}.json",case.replace('/',"-"))),report.to_json().unwrap()).unwrap();reports.push(report);
                }
                let change=reports[1].compare("source-4777f7dc0",&reports[0],0x443675).unwrap();
                println!("WIDE {case} {:.5} {:.5} {:.6}% [{:.6}%,{:.6}%] {:?}",reports[0].estimates().median,reports[1].estimates().median,change.point*100.0,change.ci_low*100.0,change.ci_high*100.0,change.verdict);
            }
        }
        _=>panic!("usage: sample before|after CASE instructions|allocations; collect NEW_DIRECTORY"),
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use std::{fs, hint::black_box, path::Path, time::Instant};
use purrdf_gts::{model::{Quad, Term}, reader::{self, StreamingSink}, writer::Writer};
#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;
#[derive(Default)]
struct Sink(usize);
impl StreamingSink for Sink {
    fn quad(&mut self, _:u64, _:Quad) { self.0 += 1; }
}
fn generate(root: &Path) {
    fs::create_dir_all(root).unwrap();
    for (name, subjects) in [("ordinary_unique", 50000usize), ("ordinary_repeated", 100usize)] {
        let mut writer=Writer::new("purrdf.gts");
        let mut terms=vec![Term::iri("https://example.org/p"),Term::iri("https://example.org/o")];
        terms.extend((0..subjects).map(|i|Term::iri(&format!("https://example.org/s{i}"))));
        let rows: Vec<Quad>=(0..50000usize).map(|i|(2+i%subjects,0,1,None)).collect();
        writer.add_terms(&terms); writer.add_quads(&rows);
        let bytes=writer.into_bytes();
        fs::write(root.join(format!("{name}.gts")),bytes).unwrap();
    }
    let input=fs::read(root.join("ordinary_unique.gts")).unwrap();
    let params=purrdf_gts::compact::CompactionParams {
        timestamp:"2026-10-06T00:00:00Z",seal_original:false,
        plan:purrdf_gts::compact::DictPlan::undicted(),content_digest:None,
        packaging_signer:(purrdf_ed25519::SigningKey::from_bytes(&[7;32]),"pack".to_string()),
    };
    let packed=purrdf_gts::compact::compact_streamable(&input,params).unwrap();
    fs::write(root.join("genuine_pack.gts"),packed).unwrap();
}
fn run(bytes:&[u8], mode:&str)->usize {
    if mode=="eager" {
        let graph=reader::read(bytes,true,None);
        assert!(graph.diagnostics.is_empty(),"{:?}",graph.diagnostics);
        black_box(graph.quads.len())
    } else {
        let mut sink=Sink::default();
        let result=reader::read_to_sink(bytes,true,None,&mut sink);
        assert!(result.diagnostics.is_empty(),"{:?}",result.diagnostics);
        black_box(sink.0)
    }
}
fn main() {
    let args: Vec<String>=std::env::args().collect();
    let root=Path::new(args.get(2).expect("command corpus_dir"));
    if args[1]=="generate" { generate(root); return; }
    println!("variant={}",args[1]);
    for name in ["ordinary_unique","ordinary_repeated","genuine_pack"] {
        let bytes=fs::read(root.join(format!("{name}.gts"))).unwrap();
        println!("input={name} bytes={} digest={:02x?}",bytes.len(),purrdf_gts::wire::blake3_256(&bytes));
        for mode in ["eager","evented"] {
            for _ in 0..2 {black_box(run(&bytes,mode));}
            let window=purrdf_alloc_probe::WholeProcessWindow::open();
            let rows=run(&bytes,mode);
            let alloc=window.close();
            let mut times=Vec::new();
            for _ in 0..11 {
                let start=Instant::now();
                assert_eq!(run(&bytes,mode),rows);
                times.push(start.elapsed().as_nanos());
            }
            times.sort_unstable();
            println!("input={name} mode={mode} rows={rows} allocations={} requested={} retained={} peak={} median_ns={} min_ns={} max_ns={}",
                alloc.allocations,alloc.requested_bytes,alloc.retained_bytes,alloc.peak_working_bytes,times[5],times[0],times[10]);
        }
    }
}


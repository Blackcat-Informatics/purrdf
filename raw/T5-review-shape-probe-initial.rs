use purrdf_gts::{compact::{self, DictPlan}, fixture::fixed_key, model::Term, reader::read, stream, writer::Writer, verify::{Keyring,verify_file_with_keyring}};
use purrdf_iri::vocab::rdf::TYPE;
use purrdf_rdf::gts_certify::{compact_and_certify,verify_compaction};
fn main() {
 let mut failures=0;
 let mut keys=Keyring::default(); keys.insert(b"author",fixed_key(1).verifying_key()); keys.insert(b"pack",fixed_key(2).verifying_key());
 let mut source=Writer::new("purrdf.gts"); source.sign_with(fixed_key(1),"author");
 source.add_terms(&[Term::iri("https://example.org/s"),Term::iri("https://example.org/p"),Term::literal("content",None)]); source.add_quads(&[(0,1,2,None)]);
 let source=source.into_bytes();
 let pack=compact_and_certify(&source,DictPlan::undicted(),"2026-01-01T00:00:00Z",false,(fixed_key(2),"pack".into())).unwrap().0;
 let graph=read(&pack,true,None);
 for variant in 0..3 {
  let mut terms=graph.terms.clone(); let mut quads=graph.quads.clone();
  let root=quads.iter().position(|(_,p,_,_)|terms[*p].iri_value()==Some(stream::DETACHED_SIGNATURE_ROOT)).unwrap();
  let (s,p,o,g)=quads[root];
  if variant==1 {
   let bad=terms.len(); terms.push(Term::literal("00".repeat(32),None)); quads.push((s,p,bad,g));
  } else if variant==2 { terms[o]=Term::blank(terms[o].value.as_ref().unwrap().clone()); }
  let mut writer=Writer::with_layout("purrdf.gts",Some("streamable"));
  writer.add_terms(&terms); writer.add_quads(&quads); writer.sign_with(fixed_key(2),"pack");writer.add_index();
  let bytes=writer.into_bytes();
  let folded=read(&bytes,true,None); let report=verify_compaction(&source,&bytes,&keys).unwrap();
  println!("root variant {variant}: diagnostics={:?}; report={report:?}",folded.diagnostics);
  if variant==0 { assert!(report.all_ok(),"valid rebuilt pack"); }
  else if report.all_ok() { failures+=1; }
 }
 let mut raw=Writer::with_layout("purrdf.gts",Some("streamable"));
 raw.add_terms(&[Term::blank("ordinary-content"),Term::iri(TYPE),Term::iri(stream::COMPACTION),Term::iri("https://example.org/p"),Term::literal("ordinary content with reserved rdf:type",None)]);
 raw.add_quads(&[(0,1,2,None),(0,3,4,None)]);raw.sign_with(fixed_key(1),"author");raw.add_index();
 let raw=raw.into_bytes();let folded=read(&raw,true,None);let pairs=compact::detached_signature_pairs(&folded).unwrap();
 println!("authored Compaction lookalike: diagnostics={:?}; signatures={:?}; pairs={}; crypto={:?}",folded.diagnostics,folded.signatures.iter().map(|s|s.packaging).collect::<Vec<_>>(),pairs.len(),verify_file_with_keyring(&raw,&keys));
 assert_eq!(folded.diagnostics,[]);
 if pairs.len()!=folded.signatures.len(){failures+=1;}
 assert_eq!(failures,0,"required refusal and original-authorship preservation failures");
}

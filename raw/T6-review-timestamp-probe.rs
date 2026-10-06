// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use purrdf_gts::{compact::{self, DictPlan}, fixture::fixed_key, model::{Term, Signature}, reader::{read, read_to_sink, StreamingSink}, stream, wire, writer::Writer, verify::{Keyring,verify_file_with_keyring}};
use purrdf_iri::vocab::rdf::TYPE;
use purrdf_rdf::gts_certify::{compact_and_certify,verify_compaction,content_projection};
#[derive(Default)]
struct Seen(Vec<Signature>);
impl StreamingSink for Seen {
 fn signature(&mut self, _: u64, signature: &Signature) { self.0.push(signature.clone()); }
}
fn main() {
 let mut keys=Keyring::default();
 keys.insert(b"author",fixed_key(1).verifying_key());
 keys.insert(b"pack",fixed_key(2).verifying_key());
 for timestamp in ["2026-01-01T00:00:00Z","2026-01-01T00:00:00+00:00","2026-01-01T00:00:00+01:00","2026-01-01T00:00:00","not-a-time", "2026-02-30T00:00:00Z"] {
  let terms=[Term::blank("authored-content"),Term::iri(TYPE),Term::iri(stream::COMPACTION),Term::iri(stream::AGENT),Term::iri(stream::TIMESTAMP),Term::iri(stream::SOURCE_HEAD),Term::iri(purrdf_xsd::datatype::XSD_DATE_TIME),Term::literal("ordinary application",None),Term::literal(timestamp,Some(6)),Term::literal(wire::digest_label(&[0;32]),None)];
  let quads=[(0,1,2,None),(0,3,7,None),(0,4,8,None),(0,5,9,None)];
  let mut writer=Writer::with_layout("purrdf.gts",Some("streamable"));
  writer.add_terms(&terms); writer.add_quads(&quads); writer.sign_with(fixed_key(1),"author"); writer.add_index();
  let input=writer.into_bytes(); let folded=read(&input,true,None); let original=(folded.signatures[0].frame_id.clone(),folded.signatures[0].cose.clone().unwrap());
  let pairs=compact::detached_signature_pairs(&folded).unwrap();
  let mut seen=Seen::default(); let streamed=read_to_sink(&input,true,None,&mut seen);
  assert_eq!(seen.0,folded.signatures); assert!(streamed.diagnostics.is_empty());
  let packed=compact_and_certify(&input,DictPlan::undicted(),"2026-01-01T00:00:00Z",false,(fixed_key(2),"pack".into())).unwrap().0;
  let post_pairs=compact::detached_signature_pairs(&read(&packed,true,None)).unwrap();
  let report=verify_compaction(&input,&packed,&keys).unwrap();
  println!("timestamp={timestamp:?} xsd_parse={:?} source_diagnostics={} crypto_ok={} packaging={} pre_pairs={} content_quads={} original_preserved={} post_pairs={} report={report:?}",purrdf_xsd::temporal::parse_datetime(timestamp),folded.diagnostics.len(),verify_file_with_keyring(&input,&keys).ok,folded.signatures[0].packaging,pairs.len(),content_projection(&folded).quads.len(),post_pairs.contains(&original),post_pairs.len());
 }
}

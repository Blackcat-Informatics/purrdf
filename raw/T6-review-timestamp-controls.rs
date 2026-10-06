// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use purrdf_gts::{compact::{compact_streamable,CompactionParams,DictPlan},fixture::fixed_key,writer::Writer};
fn main() {
 let mut writer=Writer::new("purrdf.gts"); writer.sign_with(fixed_key(1),"author"); writer.add_blob(b"content",None,None); let source=writer.into_bytes();
 for timestamp in ["2026-01-01T00:00:00Z","2026-01-01T00:00:00+00:00","2026-01-01T00:00:00-00:00","12026-01-01T00:00:00Z","-0001-01-01T00:00:00Z","2026-01-01T24:00:00Z","2026-01-01T00:00:00+01:00","2026-01-01T00:00:00","not-a-time","2026-02-30T00:00:00Z"] {
  let result=compact_streamable(&source,CompactionParams{timestamp,seal_original:false,plan:DictPlan::undicted(),content_digest:None,packaging_signer:(fixed_key(2),"pack".into())});
  println!("producer_timestamp={timestamp:?} xsd_parse={:?} compact_streamable={:?}",purrdf_xsd::temporal::parse_datetime(timestamp),result.as_ref().map(Vec::len));
 }
}

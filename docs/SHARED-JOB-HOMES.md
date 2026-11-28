<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- Generated from helpers-ledger.toml. Do not edit. -->

# Shared job homes

Use a listed public entry point rather than implementing the same job. An implementation home can be private, and not every owning package is published. The [helper ledger](../helpers-ledger.toml) defines permitted variants and the checks that reject local copies.

Regenerate this reference with `make write-shared-job-homes`.

## `purrdf-alloc-probe`

- `counting-allocator` — `purrdf_alloc_probe::CountingAllocator`
  Additional entry points are `purrdf_alloc_probe::CurrentThreadWindow`, `purrdf_alloc_probe::WholeProcessWindow`, `purrdf_alloc_probe::Measurement`.

## `purrdf-cli`

- `cli-import-pairs` — `purrdf_cli::argv_documents::parse_import_pairs`
- `cli-shared-stdin` — `purrdf_cli::argv_documents::refuse_shared_stdin`

## `purrdf-core`

- `align-up` — `purrdf_core::bytes`
- `blank-label-hex-escape` — `purrdf_core::ir::skolem::escape_label_bytes_into`
- `checked-word-prefix-plan` — `purrdf_core::content_prefix::PrefixPlan`
- `csvw-dialect-csv` — `purrdf_core::csv::Reader`
  Additional entry points are `purrdf_core::csv::Writer`, `purrdf_core::csv::Dialect`, `purrdf_core::csv::read_table`.
- `frozen-nesting-bound` — `purrdf_core::dataset_view::frozen_term_nesting_bound`
  Additional entry points are `purrdf_core::dataset_view::widest_nesting_bound`.
- `import-key-policy` — `purrdf_core::imports::ImportMap::try_insert`
  Additional entry points are `purrdf_core::imports::ImportMap::check_key`.
- `le-bytes` — `purrdf_core::bytes::read_u32_le`
  Additional entry points are `purrdf_core::bytes::read_u64_le`, `purrdf_core::bytes::put_u32_le`, `purrdf_core::bytes::put_u64_le`.
- `native-hash-table` — `purrdf_core::hash::table::HashTable`
  Additional entry points are `purrdf_core::hash::table::HashMap`, `purrdf_core::hash::reserve_table_with_memory`.
- `owned-or-borrowed` — `purrdf_core::ownership::OwnedOrBorrowed::deref`
- `purremb-fixture` — `purrdf_core::purremb_fixture::Identities`
  Additional entry points are `purrdf_core::purremb_fixture::Identities::at`.
- `rdf-collection` — `purrdf_core::DatasetView`
  Additional entry points are `purrdf_core::collections::build_rdf_list`, `purrdf_core::collections::convertible_list_cells`.
- `small-vector` — `purrdf_core::SmallVec`
- `sorted-run-end` — `purrdf_core::gallop::run_end`
- `source-row-identity` — `purrdf_core::source::LiteralSourceRow`
- `strongly-connected-components` — `purrdf_core::graph::tarjan_scc`
- `term-constructor` — `purrdf_core::TermValue`
- `term-conversion` — `purrdf_core::TermValue`
  Additional entry points are `purrdf_core::NonIriPredicate`, `purrdf_core::DatasetView`, `purrdf_core::TermLookupError`.

## `purrdf-datalog`

- `datalog-shortest-path` — `purrdf_datalog::paths::shortest_path`
- `datalog-stratum-relaxation` — `purrdf_datalog::paths::relax_strata`
- `rif-seminaive-fixpoint` — `purrdf_datalog::seminaive::evaluate_until`

## `purrdf-deflate`

- `deflate-gzip` — `purrdf_deflate::Inflater`
  Additional entry points are `purrdf_deflate::Deflater`, `purrdf_deflate::GzipDecoder`, `purrdf_deflate::GzipWriter`.
- `match-length` — `purrdf_deflate::common_prefix_len`
  Additional entry points are `purrdf_deflate::backend::Backend::match_length`.

## `purrdf-ed25519`

- `constant-time-byte-comparison` — `purrdf_ed25519::constant_time_eq`
- `ed25519` — `purrdf_ed25519::SigningKey`
  Additional entry points are `purrdf_ed25519::VerifyingKey`, `purrdf_ed25519::Signature`.

## `purrdf-events`

- `term-nesting-depth` — `purrdf_events::MAX_TERM_NESTING_DEPTH`
- `text-direction` — `purrdf_events::TextDirection`
  Additional entry points are `purrdf_cdt::TextDirection`, `purrdf_core::RdfTextDirection`, `purrdf_sparql_algebra::ast::BaseDirection`, `purrdf_gts::model::is_literal_direction`.

## `purrdf-gts`

- `cose-composite-mldsa65-ed25519` — `purrdf_gts::cose::composite::SigningKey`
  Additional entry points are `purrdf_gts::cose::composite::VerifyingKey`, `purrdf_gts::cose::composite::Signature`, `purrdf_gts::cose::composite::representative`.
- `lattice-ntt-and-codec` — `purrdf_gts::lattice::Lattice`
  Additional entry points are `purrdf_gts::lattice::pack`, `purrdf_gts::lattice::unpack`.
- `media-type-by-extension` — `purrdf_gts::files::media_type_for_extension`
  Additional entry points are `purrdf_gts::files::media_type_for_path`, `purrdf_gts::files::media_type_for_path_among`.
- `mldsa65` — `purrdf_gts::mldsa65::SigningKey`
  Additional entry points are `purrdf_gts::mldsa65::VerifyingKey`, `purrdf_gts::mldsa65::Signature`.
- `mlkem1024` — `purrdf_gts::mlkem1024::DecapsulationKey`
  Additional entry points are `purrdf_gts::mlkem1024::EncapsulationKey`, `purrdf_gts::mlkem1024::Ciphertext`, `purrdf_gts::mlkem1024::SharedSecret`.

## `purrdf-hash`

- `blake3` — `purrdf_hash::blake3::hash`
  Additional entry points are `purrdf_hash::blake3::Streaming`, `purrdf_hash::blake3::Hash`, `purrdf_hash::blake3::OutputReader`, `purrdf_hash::blake3::keyed_hash`, `purrdf_hash::blake3::derive_key`, `purrdf_hash::blake3::hash_xof`, `purrdf_hash::blake3::keyed_hash_xof`, `purrdf_hash::blake3::derive_key_xof`, `purrdf_hash::blake3::hash_batch`, `purrdf_hash::blake3::keyed_hash_batch`, `purrdf_hash::blake3::DeriveKeyContext`.
- `checked-word-identity` — `purrdf_hash::word_id::Reference`
  Additional entry points are `purrdf_hash::word_id::decode`, `purrdf_hash::word_id::decode_full`, `purrdf_hash::word_id::encode_batch`, `purrdf_hash::word_id::decode_batch`.
- `crc32` — `purrdf_hash::crc32::Crc32`
- `fixed-hasher-everywhere` — `purrdf_hash::fixed::FixedState`
  Additional entry points are `purrdf_core::FastMap`, `purrdf_core::FastSet`.
- `fixed-key-table-hash` — `purrdf_hash::fixed::FixedState`
  Additional entry points are `purrdf_core::FastHasher`, `purrdf_core::FastMap`, `purrdf_core::FastSet`.
- `fnv1a64` — `purrdf_hash::fnv`
  Additional entry points are `purrdf_hash::fnv::fnv1a64`, `purrdf_hash::fnv::fold`, `purrdf_hash::fnv::BASIS`, `purrdf_hash::fnv::PRIME`.
- `frame-le` — `purrdf_hash::frame::frame_le`
  Additional entry points are `purrdf_hash::frame::frame_le_into`, `purrdf_hash::frame::frame_be_labelled`.
- `hash-domain` — `purrdf_hash::Domain`
- `hex` — `purrdf_hash::hex::encode`
  Additional entry points are `purrdf_hash::hex::Lower`, `purrdf_hash::hex::Upper`, `purrdf_hash::hex::encode_into`, `purrdf_hash::hex::encode_to_slice`, `purrdf_hash::hex::decode`, `purrdf_hash::hex::decode_canonical`, `purrdf_hash::hex::decode_canonical_to_slice`, `purrdf_hash::hex::decode_32`, `purrdf_hash::hex::decode_32_canonical`, `purrdf_hash::hex::nibble`, `purrdf_hash::hex::nibble_canonical`, `purrdf_hash::hex::Digest32`.
- `lcg64-mmix` — `purrdf_hash::mix::lcg64_next`
  Additional entry points are `purrdf_hash::mix::LCG64_MULTIPLIER`, `purrdf_hash::mix::LCG64_MMIX_INCREMENT`, `purrdf_testkit::rng::lcg64_next`.
- `md5` — `purrdf_hash::md5::Md5`
- `multiset-256` — `purrdf_hash::multiset::Multiset256`
- `secret-clearing` — `purrdf_hash::wipe_secret`
- `sha1` — `purrdf_hash::sha1::Sha1`
- `sha2` — `purrdf_hash::sha2`
  Additional entry points are `purrdf_hash::sha2::Sha256Batch`.
- `sha3` — `purrdf_hash::sha3::Sha3`
  Additional entry points are `purrdf_hash::sha3::keccak_f1600`, `purrdf_hash::sha3::Shake`, `purrdf_hash::sha3::ShakeReader`.
- `sorted-row-set-fingerprint` — `purrdf_hash::multiset::sorted_fingerprint`
- `splitmix64` — `purrdf_hash::mix`
  Additional entry points are `purrdf_hash::mix::splitmix64_next`, `purrdf_hash::mix::splitmix64_step`, `purrdf_hash::mix::splitmix64_finalize`, `purrdf_hash::mix::GOLDEN_GAMMA`, `purrdf_testkit::rng::splitmix64_next`, `purrdf_testkit::rng::splitmix64_step`, `purrdf_hnsw::level::splitmix64`, `purrdf_bench::splitmix64`.
- `structured-error-list` — `purrdf_hash::error::visit_list`
- `xxh64` — `purrdf_hash::xxh64::Xxh64`
  Additional entry points are `purrdf_hash::xxh64::hash`.

## `purrdf-iri`

- `iri-reference-resolution` — `purrdf_iri::BaseScope`
  Additional entry points are `purrdf_iri::Iri::resolve`, `purrdf_core::BaseIri`, `purrdf_core::parse_iri`.
- `prefixed-name` — `purrdf_iri::contract_where`
  Additional entry points are `purrdf_iri::contract`, `purrdf_iri::local_name`, `purrdf_iri::split_local_name`.
- `w3c-vocab` — `purrdf_iri::vocab`
  Additional entry points are `purrdf_xsd::datatype`, `purrdf_core::vocab`, `purrdf_core::datatype`.

## `purrdf-jsonschema`

- `json-schema-validation` — `purrdf_jsonschema::Schema`
  Additional entry points are `purrdf_jsonschema::Registry`.

## `purrdf-lex`

- `byte-search` — `purrdf_lex::scan::find_byte`
  Additional entry points are `purrdf_lex::scan::find_byte2`, `purrdf_lex::scan::find_byte_pair`, `purrdf_iri::scan`.
- `cbor-codec` — `purrdf_lex::cbor::encode`
  Additional entry points are `purrdf_lex::cbor::Value`, `purrdf_lex::cbor::canonical`, `purrdf_lex::cbor::decode`, `purrdf_lex::cbor::write_canonical_map`, `purrdf_lex::cbor::head::write_head`, `purrdf_lex::cbor::head::with_encoded_head`, `purrdf_lex::cbor::head::with_encoded_text`, `purrdf_lex::cbor::head::with_encoded_bytes`.
- `crockford-base32` — `purrdf_lex::crockford::write_u128`
  Additional entry points are `purrdf_lex::crockford::parse_u128`, `purrdf_lex::crockford::encode_u128`.
- `escape-decode` — `purrdf_lex::terminals::decode_uchar`
  Additional entry points are `purrdf_lex::terminals::decode_uchar_at`, `purrdf_lex::terminals::echar_value`, `purrdf_lex::terminals::expand_uchars`, `purrdf_lex::terminals::decode_char_ref`, `purrdf_lex::json_escape::decode_escape`, `purrdf_lex::json_escape::decode_u_escape`, `purrdf_lex::json_escape::unescape`, `purrdf_iri::terminals`, `purrdf_iri::json_escape`, `purrdf_hash::hex::parse_u32`.
- `grammar-ws` — `purrdf_lex::terminals::skip_ws`
  Additional entry points are `purrdf_lex::terminals::trim_ws`, `purrdf_lex::terminals::find_first_trivia`.
- `html-character-reference` — `purrdf_lex::html::resolve`
  Additional entry points are `purrdf_lex::html::resolve_strict`, `purrdf_lex::html::Mode`, `purrdf_lex::html::Decoded`, `purrdf_lex::html::Diagnostic`, `purrdf_lex::html::ErrorKind`.
- `json-document` — `purrdf_lex::json::read`
  Additional entry points are `purrdf_lex::json::Value`, `purrdf_lex::json::Reader`, `purrdf_lex::json::write_compact`, `purrdf_lex::json::write_pretty`, `purrdf_lex::json::occurrences`, `purrdf_lex::json::record::Record`.
- `json-pointer` — `purrdf_lex::json_pointer::escape_token`
  Additional entry points are `purrdf_lex::json_pointer::push_token`, `purrdf_lex::json_pointer::unescape_token`, `purrdf_lex::json_pointer::tokens`, `purrdf_lex::json_pointer::array_index`, `purrdf_lex::json_pointer::JsonPointer`, `purrdf_iri::json_pointer`.
- `literal-and-iri-escape` — `purrdf_lex::literal_escape::write`
  Additional entry points are `purrdf_lex::literal_escape::escape`, `purrdf_lex::literal_escape::write_rdfc12_v3`, `purrdf_lex::iri_escape::push_escaped`, `purrdf_core::iri_escape`.
- `logical-capacity-panic` — `purrdf_lex::allocation::CapacityError::check`
  Additional entry points are `purrdf_lex::allocation::CapacityError::slot`.
- `percent-encoding` — `purrdf_lex::percent::encode`
  Additional entry points are `purrdf_lex::percent::push_encoded`, `purrdf_lex::percent::push_triplet`, `purrdf_lex::percent::decode`, `purrdf_lex::percent::decode_form`, `purrdf_lex::percent::normalize`, `purrdf_iri::percent`.
- `range-table-search` — `purrdf_lex::terminals::in_ranges`
  Additional entry points are `purrdf_iri::terminals`.
- `term-syntax` — `purrdf_lex::term_syntax::write_literal`
  Additional entry points are `purrdf_lex::term_syntax::write_iri`, `purrdf_lex::term_syntax::write_iri_rdfc12_v3`, `purrdf_lex::term_syntax::write_literal_rdfc12_v3`, `purrdf_lex::term_syntax::write_blank`, `purrdf_core::write_term_value`.
- `unicode-normalization` — `purrdf_lex::unicode::nfc`
  Additional entry points are `purrdf_lex::unicode::nfd`, `purrdf_lex::unicode::nfkc`, `purrdf_lex::unicode::nfkd`, `purrdf_lex::unicode::is_nfc`, `purrdf_lex::unicode::ccc`, `purrdf_lex::unicode::Decompose`, `purrdf_lex::unicode::Compose`, `purrdf_lex::unicode::drive`, `purrdf_lex::unicode::UNICODE_VERSION`, `purrdf_text::unicode::nfc`.
- `unsigned-word-division` — `purrdf_lex::integer_text::div_rem_word`
- `visual-fingerprint` — `purrdf_lex::visual_fingerprint::render_bytes`
  Additional entry points are `purrdf_lex::visual_fingerprint::present`, `purrdf_lex::visual_fingerprint::parse_digest`, `purrdf_core::visual_fingerprint`, `purrdf_gts::emojihash`.
- `xml-reader` — `purrdf_lex::xml::Document`
  Additional entry points are `purrdf_lex::xml::Reader`.
- `yaml-document` — `purrdf_lex::yaml::read`
  Additional entry points are `purrdf_lex::yaml::write`, `purrdf_lex::yaml::read_with`.

## `purrdf-rdf`

- `reifier-rows` — `purrdf_rdf::flat_rdf_quads`
  Additional entry points are `purrdf_rdf::flat_rdf_quads_from_dataset`.

## `purrdf-retrieval`

- `div-ceil` — `purrdf_retrieval::reciprocal_rank::ceil_div`

## `purrdf-shapes`

- `rules-dispatch` — `purrdf_shapes::rules::run_rules`
  Additional entry points are `purrdf_shapes::run_rules`, `purrdf_shapes::rules::RuleLimits::rule_options`, `purrdf_shapes::rules::RuleLimits::infer_options`.
- `schema-nesting-depth` — `purrdf_shapes::limits::MAX_SCHEMA_DEPTH`
  Additional entry points are `purrdf_shapes::limits::ensure_depth`.

## `purrdf-sparql-algebra`

- `term-pattern-variables` — `purrdf_sparql_algebra::TermPattern::for_each_variable`
  Additional entry points are `purrdf_sparql_algebra::TermPattern::collect_variables`, `purrdf_sparql_algebra::TermPattern::collect_variable_names`, `purrdf_sparql_algebra::TriplePattern::collect_variable_names`.

## `purrdf-stack`

- `thread-stack` — `purrdf_stack::on_stack`
  Additional entry points are `purrdf_stack::on_stack_scoped`.

## `purrdf-testkit`

- `bench-harness` — `purrdf_testkit::bench`
  Additional entry points are `purrdf_testkit::bench::Bench`, `purrdf_testkit::bench_group`, `purrdf_testkit::bench_main`.
- `byte-exact-goldens` — `purrdf_testkit::assert_golden`
- `seeded-permutation` — `purrdf_testkit::rng::permute`
- `structured-error-capture` — `purrdf_testkit::error_capture::Capture::read`
- `test-temporary-paths` — `purrdf_testkit::TempDir`
  Additional entry points are `purrdf_testkit::NamedTempFile`, `purrdf_testkit::temp_dir`, `purrdf_testkit::temp_file`.
- `test-workspace-paths` — `purrdf_testkit::paths::workspace_root_from`
  Additional entry points are `purrdf_testkit::paths::collect_rs`.
- `xorshift64` — `purrdf_testkit::rng::xorshift64_next`

## `purrdf-toml`

- `toml-document` — `purrdf_toml::analyze`
  Additional entry points are `purrdf_toml::Document`, `purrdf_toml::project`, `purrdf_toml::encode`, `purrdf_toml::decode_document`.

## `purrdf-validate`

- `query-host-plumbing` — `purrdf_validate::governors::from_parts`
  Additional entry points are `purrdf_validate::query::provenance`, `purrdf_validate::governors::from_update_parts`, `purrdf_sparql_results::SparqlResultsFormat::from_name`, `purrdf_validate::query::statistical_aggregates`, `purrdf_validate::xpath_regex::parse_profile`, `purrdf_validate::xpath_regex::sparql_engine`, `purrdf_validate::xpath_regex::query_options`.

## `purrdf-wasm`

- `wasm-host-clock` — `purrdf_wasm::host::date_now`
- `wasm-host-reflect-get` — `purrdf_wasm::host::reflect_get`

## `purrdf-xsd`

- `bigint` — `purrdf_xsd::bigint::BigInt`
  Additional entry points are `purrdf_xsd::BigInt`, `purrdf_xsd::bigint::BigInt::from_digits`, `purrdf_xsd::bigint::BigInt::from_binary`, `purrdf_xsd::bigint::BigInt::mul_small`, `purrdf_xsd::bigint::BigInt::mul_pow5`, `purrdf_xsd::bigint::BigInt::mul`, `purrdf_xsd::bigint::BigInt::rem`, `purrdf_xsd::json_number::cmp`.
- `calendar` — `purrdf_xsd::temporal::days_from_civil`
  Additional entry points are `purrdf_xsd::temporal::civil_from_days`, `purrdf_xsd::temporal::days_in_month`, `purrdf_xsd::temporal::is_leap`, `purrdf_xsd::days_from_civil`.
- `float-control-register` — `purrdf_xsd::ieee::control::mxcsr`
  Additional entry points are `purrdf_xsd::ieee::control::set_mxcsr`, `purrdf_xsd::ieee::control::Mxcsr`, `purrdf_xsd::ieee::control::fpcr`, `purrdf_xsd::ieee::control::set_fpcr`, `purrdf_xsd::ieee::control::Fpcr`.
- `numeric-predicate` — `purrdf_xsd::XsdDatatype::is_numeric`
  Additional entry points are `purrdf_xsd::XsdDatatype::is_integer_family`, `purrdf_xsd::XsdValue::is_numeric`, `purrdf_xsd::numeric::is_integer_lexical`, `purrdf_xsd::numeric::is_decimal_lexical`.
- `rfc3339` — `purrdf_xsd::rfc3339::parse`
  Additional entry points are `purrdf_xsd::rfc3339::parse_date`, `purrdf_xsd::rfc3339::parse_time`, `purrdf_xsd::rfc3339::format`, `purrdf_xsd::rfc3339::Separator`, `purrdf_xsd::rfc3339::LeapSecond`.
- `wide-arith` — `purrdf_xsd::wide::mul_div`
  Additional entry points are `purrdf_xsd::wide::wide_mul`, `purrdf_xsd::wide::div_wide`, `purrdf_xsd::wide::mul_div_ceil`, `purrdf_xsd::wide::gcd`.

## `purrdf-zstd`

- `zstd-codec` — `purrdf_zstd::decoding::FrameDecoder`
  Additional entry points are `purrdf_zstd::encoding::compress_slice_to_vec`, `purrdf_zstd::dictionary::create_fastcover_dict_from_source`.

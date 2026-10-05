<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# AI Developer Agent Guide (AGENTS.md)

Welcome, AI Agent! This file is your behavioral contract and instruction manual for
contributing to the PurRDF repository.

---

## Deficiency emergency ledger (non-negotiable)

`.deficiencies` is the log of last resort for critically undone work. Every entry
below its marker is **100% unauthorized**, is **100% a bug**, and means its
originating issue or pull request failed. It is used for work misrepresented to
pass PR gates, work misrepresented by an agent, or a discovery that an agent was
fundamentally defective. An entry is literally a cry for help from a failing
agent; it is never an accepted risk, authorized descope, backlog, or success with
caveats.

The only normal contents are the tracked notice and marker, with no entries below
them. An entry blocks completion, PR creation, and merge of the work that produced
it. Immediately verify the defect against current code and give it a durable,
visible remediation owner before removing the emergency entry. Removing an entry
does not resolve the bug or retroactively make the failed work successful. Never
add ledger text to make incomplete work appear complete.

---

## 1. What this repository is

PurRDF is the **RDF 1.2 toolkit** that several downstream projects (notably
[`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology)) use as
their **data-carrier backbone**. It must stay fast, deterministic, and boring:
one engine, one behavior, carried verbatim into Rust, Python, WebAssembly, and C.

Crate map (all under `crates/`, published names in `Cargo.toml`):

| Crate | Role |
|---|---|
| `purrdf` | Umbrella facade (RDF surface at root; `slice`/`shapes` as modules) |
| `purrdf-rdf` (`crates/rdf`) | Native text/XML/JSON-LD codecs, GTS adapters, describe, canonicalization |
| `purrdf-core` (`crates/rdf-core`) | Interned IR kernel, diagnostics, store traits, provenance, RDFC-1.0 |
| `purrdf-columnar` (`crates/columnar`) | Bidirectional five-table Parquet codec for RDF 1.2 + blobs |
| `purrdf-gts` (`crates/gts`) | GTS container engine (CBOR log through `purrdf_lex::cbor`, BLAKE3, COSE, Ed25519 through `purrdf-ed25519`) |
| `purrdf-sparql-{algebra,eval,results}` | SPARQL 1.1/1.2 parser, evaluator, results |
| `purrdf-shapes` (`crates/shapes`) | SHACL validation (full Core + SHACL-SPARQL + SHACL-AF) and SHACL Rules (`sh:rule` inference) |
| `purrdf-shex` (`crates/shex`) | ShEx 2.1 schemas + validation |
| `purrdf-slice` (`crates/slice`) | Slice catalog, artifacts, ownership analysis |
| `purrdf-datalog` (`crates/datalog`) | Deterministic semi-naive Datalog substrate beneath every rule-driven engine |
| `purrdf-entail` (`crates/entail`) | Entailment regimes: RDF/RDFS/OWL 2 RL/D materialization, OWL-Direct, RIF |
| `purrdf-geo` (`crates/geo`) | GeoSPARQL 1.1: exact float-free WKT/GeoJSON geometry and the `geof:` family over both extension seams |
| `purrdf-text` (`crates/text`) | Deterministic full-text search over literals: exact fixed-point BM25, ranked rows through the property-function seam |
| `purrdf-retrieval` (`crates/retrieval`) | Composition layer over the ranked producers: plan → compile → execute → fuse, with a canonical BLAKE3 plan identity and an exact, content-addressed fusion law; producers, strata and weights are caller-supplied |
| `purrdf-validate` (`crates/validate`) | Shared string boundary every language binding routes through, including what every host (CLI, C ABI, wasm, Python) shares around a query: the governor decoder (`governors::from_parts`, from `QueryGovernors::METERED`) and the query provenance record (`query::provenance`) |
| `purrdf-json` (`crates/json`) | Ordered JSON byte-cover codec with queryable occurrences, strict reconstruction and caller-selected profile; runtime dependencies are `purrdf-core`, `purrdf-lex` (the JSON string decoder and pointer tokens) and the `purrdf-hash` root |
| `purrdf-hash` (`crates/hash`) | Native, zero-dependency hashing: BLAKE3-256, MD5 (RFC 1321), SHA-1 (FIPS 180-4), SHA3-224/256/384/512 (FIPS 202) and CRC-32 (ISO-HDLC), streaming and one-shot; SHA-1 and CRC-32 run on the x86 SHA/`pclmulqdq` and Armv8 SHA1/CRC32 instructions when detected at run time, the portable source otherwise; the SPARQL hash built-ins, OpenPGP fingerprints and derivation identities compute through it. Also `fixed::FixedHasher`, the workspace's fixed-key table hasher: folded multiplies, or an AES accumulator on a build whose target enables AES (compile-time only, never run-time detection; wasm32 and 32-bit targets use the portable function), each function pinned by frozen self-vectors. The workspace root: zero dependencies, and every crate may depend on it; it also holds the small specified kernels shared across the workspace (`hex`, base16 in either case with the `Digest32` value every 32-byte content identity wraps; `frame`, the length framing every preimage uses; `fnv`, FNV-1a 64-bit; `mix`, SplitMix64, its `[-1, 1)` signed-unit draws and the MMIX 64-bit LCG), `Domain`, the registry type of every hash domain-separation string, and the `Backend` trait every family of named execution paths implements (`selected`, `is_available`, `all_available`, `name`), with the one `PURRDF_REQUIRE_SIMD_PATHS` check in `dispatch`; and the impl macros `debug_non_exhaustive!`, `default_from_new!` and `vector_backend!`. It implements no SHA-2: the external `sha2` crate is the workspace's one SHA-2 |
| `purrdf-deflate` (`crates/deflate`) | Native DEFLATE (RFC 1951) and gzip (RFC 1952): a push-based streaming decoder that decodes every gzip member, verifies each trailer and refuses trailing garbage and output past a caller's limit, and a deterministic encoder (gzip `MTIME` 0, `XFL` 0, `OS` 255; the same bytes on every path and however the input is chunked); match copies, match-length compares and window hashing run on SSE2/AVX2, NEON or wasm simd128, portable code otherwise; sole runtime dependency is `purrdf-hash` (the CRC-32) |
| `purrdf-jsonschema` (`crates/jsonschema`) | Native JSON Schema validation for drafts 2020-12, 2019-09 and 07, each schema resource in its own dialect: every vocabulary, `$dynamicRef`, `$recursiveRef`, `unevaluated*`, `$vocabulary`, the flag/basic/detailed output formats, exact decimal numbers, and ECMA-262 `/u` patterns using `regex` for regular expressions and a bounded explicit-stack matcher for lookaround, backreferences and scoped modifiers; `Schema::is_valid`/`evaluate` return typed errors on resource exhaustion; the `date-time`, `date` and `time` formats read through `purrdf_xsd::rfc3339` and exact number comparison computes on `purrdf_xsd::bigint::BigInt`; depends on `regex`, `purrdf-iri`, `purrdf-xsd`, `purrdf-lex` and `purrdf-hash` only |
| `purrdf-markdown` (`crates/markdown`) | Structural Markdown-to-RDF 1.2 slicer under a shipped specification: a typed stand-off model over verbatim byte spans, projected to claims; sole runtime dependency is `purrdf-core` |
| `purrdf-ed25519` (`crates/ed25519`) | Ed25519 signatures (RFC 8032): key expansion, deterministic signing over constant-time field and scalar arithmetic, and strict cofactorless verification that refuses a non-canonical `S`, a non-canonical point encoding and a small-order key or `R`; every GTS and RDF signer and verifier uses it; runtime dependencies are `purrdf-hash` and `sha2` (SHA-512) |
| `purrdf-lex` (`crates/lex`) | The workspace's lexical layer, over the zero-dependency `purrdf-hash` root alone: the exact Turtle/SPARQL/XML/ECMA-262 terminal classes and escape decoders (`terminals`: `decode_uchar`, `echar_value`, `decode_char_ref`, `skip_ws`/`trim_ws`, `is_ncname`, `in_ranges`); the chunked byte-class scanners and `ByteClass` kernel that lower to packed compares on SSE2/AVX2/AVX-512, NEON and wasm simd128, with `find_byte`/`find_byte2` for needles known at run time and `find_byte_pair` for two positions a fixed gap apart, the search behind `purrdf_core::xsd_regex`'s literal prefilter (`scan`); the RFC 8259 JSON string escaper and decoder (`json_escape`); RFC 6901 JSON Pointer tokens (`json_pointer`); RFC 3986/3987/6570 percent-encoding (`percent`); the one JSON reader, lexeme-keeping value and deterministic writer, with the strict typed record reader and its record codec (`json`, `json::record`, `json_record!`); the YAML 1.2 core-schema reader and block emitter over the JSON data model (`yaml`); the RFC 8949 CBOR codec with core deterministic encoding (`cbor`); the XML 1.0 + Namespaces reader, which expands an internal DTD subset under a budget and refuses external entities (`xml`); the literal and IRI escapers for each carrier (`literal_escape`, `iri_escape`), the RDF 1.2 term spelling (`term_syntax`), the text sink (`text_out`) and Crockford Base32 (`crockford`); the typed diagnostic presentation (`diagnostic`: stable message identity and exact typed arguments, validated against an English template; `purrdf_core::diagnostic` re-exports it); and the one Unicode normalization pipeline (`unicode`: NFC, NFD, NFKC, NFKD, `is_nfc`, `ccc` and the streaming stages the text analyzer composes its case fold with) over tables generated at `unicode::UNICODE_VERSION`, the version every Unicode table in the workspace is generated from by its one generator (`examples/gen_unicode_tables.rs`); and the shared structures and constructor macros (`walk`, `assoc`, `constructors!`, `variant_from!`, `message_error!`; see [Shared structures and constructors](#shared-structures-and-constructors)) |
| `purrdf-iri` (`crates/iri`) | IRI/URI value space (RFC 3987/3986 parse, resolution, normalization, CURIEs, BCP 47 tags and their RDF 1.2 identity fold, IDNA2008) and `vocab`, the W3C vocabulary terms (one module per W3C namespace; XSD datatype IRIs live in `purrdf_xsd::datatype`); runtime dependencies are `purrdf-lex`, whose `terminals`, `scan`, `json_escape`, `json_pointer` and `percent` it re-exports, and the `purrdf-hash` root |
| `purrdf-xsd` | Foundation over `purrdf-lex` and `purrdf-hash` |
| `purrdf-events` | Zero-dependency foundation: the event protocol and `TextDirection`, the one RDF 1.2 base-direction type |
| `purrdf-deflate` | Leaf over `purrdf-hash` alone |
| `purrdf-cdt` (`crates/cdt`) | SPARQL composite datatypes (SEP-0009 `cdt:List`/`cdt:Map`), re-exporting the one RDF 1.2 base-direction type, `purrdf_events::TextDirection`: `no_std` closed leaf over `purrdf-events`, `purrdf-iri`, `purrdf-xsd`, `purrdf-lex` and `purrdf-hash` only |
| `purrdf-stack` (`crates/stack`) | How much stack the thread has left (native OS limit, read via target-gated `libc`/`windows-sys` declarations with no C toolchain needed; wasm32 shadow stack against an installable floor) and the margin the SPARQL evaluator refuses at; `on_stack`/`on_stack_scoped` run a computation on a stack of a stated size (a fresh thread natively, checked against the floor on wasm32) |
| `purrdf-wasm`, `purrdf-capi`, `bindings/python` | WASM, C-ABI, and PyO3 bindings |
| `purrdf-cli` (`crates/cli`) | The `purrdf` command-line surface (`publish = false`) |
| `purrdf-envelope-probe` (`crates/envelope-probe`) | The micro-hardware envelope capture tool (`publish = false`) |
| `purrdf-alloc-probe` (`crates/alloc-probe`) | The shared counting allocator + per-thread/whole-process measurement windows every allocation test and bench measures with (`publish = false`, `[dev-dependencies]` only, path-only with no `version`) |
| `purrdf-bench` (`crates/bench`) | Benchmark tooling: the scale-corpus generator (`publish = false`) |
| `purrdf-testkit` (`crates/testkit`) | Shared test support: byte-exact goldens (`assert_golden!`, `golden`), the workspace root and Rust-source walks for tests and generators (`paths`), seeded test draws over `purrdf_hash::mix` (`rng`), an exact rational oracle that adds, multiplies, divides and orders exact values and rounds an integer, a decimal or a float's binary value to `f64`, `f32` or a fixed decimal scale with integer arithmetic only (`exact`), temporary paths under the target directory (`temp_dir!`, `temp_file!`, `for_unit_test`), self-hashing frozen differential vectors, the libtest-compatible `harness = false` runner, the property harness (`prop_test!`: choice-sequence shrinking, regex string generators, stateful model testing, a deterministic seed per property), and the micro-benchmark harness every bench target runs on (`purrdf_testkit::bench`, `bench_group!`/`bench_main!`: warm-up, flat sampling, median with MAD and a seeded bootstrap interval, throughput, saved baselines compared with a bootstrapped change, a fixed-schema `estimates.json` per benchmark, natively and on wasm32); its one first-party dependency is `purrdf-hash`, the root, whose own tests do not use testkit, so no member's tests close a cycle through it (`publish = false`, `[dev-dependencies]` only, path-only with no `version`) |
| `wasm-link` (`crates/wasm-link`) | The wasm package's post-link step: links the suspend, run and poison guarantees into the optimized module (`publish = false`, host tool) |
| `purrdf-hash-conformance` (`crates/hash-conformance`) | The frozen-vector suites of `purrdf-hash` (digest differentials, BLAKE3 streaming boundaries, base16 rendering, the table hasher's self-vectors and quality), the digest, base16 and BLAKE3 throughput bench and the table hasher's latency bench, on testkit's runner and bench harness, natively and on wasm32; separate from `purrdf-hash` because testkit depends on it (`publish = false`) |
| `helper-census` (`crates/helper-census`) | The structural helper census: normalises every function body in shipping code and in every test, bench and example target (local names renamed, literals abstracted, borrow, deref and value-adapter forms such as `&x`, `x.as_str()` and `x.clone()` dropped, bodies from 20 tokens up with small ones keeping their literals) and reports isomorphic bodies, repeated thin forwarders, constants by value and hex-digit tables against `helpers-ledger.toml`; also hosts the non-Rust ratchet (`--non-rust-ratchet`, see [Hard constraints](#2-hard-constraints-violating-these-fails-ci-or-review)) (`publish = false`, host tool) |

### Where each job lives

Every job the workspace implements once has one home. `helpers-ledger.toml` is
authoritative (with each job's specification, frozen vectors, sanctioned
variants and forbidden fingerprints); this table lists its rows, in ledger
order, with the home each names. Call the home; never write a second body.

| Job | Home |
|---|---|
| `iri-reference-resolution` | `purrdf_iri::BaseScope` |
| `fixed-key-table-hash` | `purrdf_hash::fixed::FixedState` |
| `fixed-hasher-everywhere` | `purrdf_hash::fixed::FixedState` |
| `hash-domain` | `purrdf_hash::Domain` |
| `blake3` | `purrdf_hash::blake3::hash` |
| `splitmix64` | `purrdf_hash::mix` |
| `fnv1a64` | `purrdf_hash::fnv` |
| `md5` | `purrdf_hash::md5::Md5` |
| `sha1` | `purrdf_hash::sha1::Sha1` |
| `sha3` | `purrdf_hash::sha3::Sha3` |
| `crc32` | `purrdf_hash::crc32::Crc32` |
| `deflate-gzip` | `purrdf_deflate::Inflater` |
| `match-length` | `purrdf_deflate::common_prefix_len` |
| `csvw-dialect-csv` | `purrdf_core::csv::Reader` |
| `strongly-connected-components` | `purrdf_core::graph::tarjan_scc` |
| `small-vector` | `purrdf_core::SmallVec` |
| `hex` | `purrdf_hash::hex::encode` |
| `json-schema-validation` | `purrdf_jsonschema::Schema` |
| `byte-exact-goldens` | `purrdf_testkit::assert_golden` |
| `test-temporary-paths` | `purrdf_testkit::TempDir` |
| `bench-harness` | `purrdf_testkit::bench` |
| `escape-decode` | `purrdf_lex::terminals::decode_uchar` |
| `grammar-ws` | `purrdf_lex::terminals::skip_ws` |
| `json-pointer` | `purrdf_lex::json_pointer::escape_token` |
| `percent-encoding` | `purrdf_lex::percent::encode` |
| `range-table-search` | `purrdf_lex::terminals::in_ranges` |
| `byte-search` | `purrdf_lex::scan::find_byte` |
| `w3c-vocab` | `purrdf_iri::vocab` |
| `text-direction` | `purrdf_events::TextDirection` |
| `frame-le` | `purrdf_hash::frame::frame_le` |
| `le-bytes` | `purrdf_core::bytes::read_u32_le` |
| `align-up` | `purrdf_core::bytes` |
| `div-ceil` | `purrdf_retrieval::reciprocal_rank::ceil_div` |
| `wide-arith` | `purrdf_xsd::wide::mul_div` |
| `bigint` | `purrdf_xsd::bigint::BigInt` |
| `calendar` | `purrdf_xsd::temporal::days_from_civil` |
| `rfc3339` | `purrdf_xsd::rfc3339::parse` |
| `numeric-predicate` | `purrdf_xsd::XsdDatatype::is_numeric` |
| `unicode-normalization` | `purrdf_lex::unicode::nfc` |
| `term-conversion` | `purrdf_core::TermValue` |
| `term-constructor` | `purrdf_core::TermValue` |
| `json-document` | `purrdf_lex::json::read` |
| `yaml-document` | `purrdf_lex::yaml::read` |
| `cbor-codec` | `purrdf_lex::cbor::encode` |
| `xml-reader` | `purrdf_lex::xml::Document` |
| `literal-and-iri-escape` | `purrdf_lex::literal_escape::write` |
| `term-syntax` | `purrdf_lex::term_syntax::write_literal` |
| `crockford-base32` | `purrdf_lex::crockford::write_u128` |
| `rdf-collection` | `purrdf_core::DatasetView` |
| `thread-stack` | `purrdf_stack::on_stack` |
| `test-workspace-paths` | `purrdf_testkit::paths::workspace_root` |
| `query-host-plumbing` | `purrdf_validate::governors::from_parts` |
| `ed25519` | `purrdf_ed25519::SigningKey` |
| `prefixed-name` | `purrdf_iri::contract_where` |

### Shared structures and constructors

The general-purpose structures a job's home is built from are shared the same
way, so a crate reaches for them rather than writing its own:

* `purrdf_lex::walk`: `WorkList` (the heap work list every whole-tree walk
  keeps), `Nested`/`Dismantle` (an owned child box whose drop is iterative)
  and `write_debug` with `Tok` (a recursive type's `Debug`, byte-identical to
  the derive's, without recursing on the machine stack).
* `purrdf_lex::assoc`: `get`, `get_mut` and `insert` over an ordered
  `[(K, V)]` association list, read by first match.
* `purrdf_lex::json_record!`: the one record codec over
  `purrdf_lex::json::record` (`ToJson`, `FromJson`, or both from one member
  list, optionally building through a validating constructor).
* `purrdf_lex::message_error!` (an error that is one message),
  `purrdf_hash::debug_non_exhaustive!` (a `Debug` that elides fields) and
  `purrdf_hash::vector_backend!` (a named family of dispatch paths).

**Constructors.** A type whose empty value is what `#[derive(Default)]`
produces derives `Default` and has no trivial `new()`. Keep `new()` and
generate `Default` from it with `purrdf_hash::default_from_new!` only when
`new` must be a `const fn` or the default differs from the derived one
(`default_from_new!(T => name)` names a constructor other than `new`). A
constructor whose whole body is one conversion is declared through
`purrdf_lex::constructors!`, and a `From` impl that wraps a source into one
enum variant through `purrdf_lex::variant_from!` (`Variant(A, B) as convert`
for a variant that holds a rendering of its sources).

### Removed external packages

The native homes above replace these packages, and
`scripts/check-banned-deps.py` (`BANNED_ANY_EDGE`) refuses every one of them on
any edge — runtime, build, dev/test or transitive — in every committed
`Cargo.lock`, each ban naming its replacement (`memchr` alone is refused as a
direct dependency, in `BANNED_DIRECT_ONLY`):

* JSON, YAML, CBOR and XML: `serde`, `serde_core`, `serde_derive`,
  `serde_json`, `itoa`, `ryu`, `zmij`, `indexmap`, `equivalent`,
  `serde_yaml_ng`, `unsafe-libyaml`, `ciborium`, `ciborium-io`, `ciborium-ll`,
  `half`, `crunchy`, `zerocopy`, `zerocopy-derive` and `roxmltree` —
  `purrdf_lex::{json, yaml, cbor, xml}`, with hand-written `to_json`/`from_json`
  over `purrdf_lex::json::Value` in place of derives.
* Ed25519: `ed25519-dalek`, `curve25519-dalek`, `curve25519-dalek-derive`,
  `fiat-crypto`, `ed25519`, `signature`, `subtle`, `zeroize`, `rustc_version`,
  `semver`, `pkcs8`, `spki`, `der` and `base64ct` — `purrdf-ed25519`.
* Benchmarks: `criterion` and its closure (`criterion-plot`, `anes`,
  `is-terminal`, `hermit-abi`, `cast`, `num-traits`, `autocfg`, `itertools`,
  `oorandom`, `tinytemplate`, `walkdir`, `same-file`, `winapi-util`) —
  `purrdf_testkit::bench`.
* Byte search: `memchr` — `purrdf_lex::scan::find_byte`/`find_byte2`.
  `regex` is built without its `perf-literal` prefilter, so neither `memchr`
  nor `aho-corasick` is compiled into any build; Cargo keeps both in
  `Cargo.lock` only because `regex`'s weak feature references name them.
* Base16, dates and normalization: `hex` (`purrdf_hash::hex`), `time`,
  `time-core`, `time-macros`, `deranged`, `num-conv` and `powerfmt`
  (`purrdf_xsd::rfc3339`), `unicode-normalization`, `tinyvec` and
  `tinyvec_macros` (`purrdf_lex::unicode`).

`sha2` is retained: it is the workspace's one SHA-2 implementation.
`dependency-ledger.toml` gives every package that remains a category, a reason
and a census verdict (`distinct`, or `duplicates_native` naming the home that
makes it removable).

## 2. Hard constraints (violating these fails CI or review)

* **NO semantic Cargo features, ever.** The sole exception is the empty,
  non-semantic `purrdf-capi:capi = []` marker that `cargo-c` requires. It gates
  no code and must never appear in `cfg(feature = ...)`; CI checks both facts
  with `helper-census --no-features`. PurRDF is a carrier; optionality changes
  semantics per consumer, which is forbidden. Do not add any other feature,
  optional dependency, or feature-gated behavior.
* **Tooling and tests are Rust.** Checks, generators, gates and tests are written
  in Rust (`helper-census`, an xtask, a Rust test). Python and JavaScript remain
  only where a language surface itself must be exercised. The non-Rust ratchet,
  `helper-census --non-rust-ratchet` (`crates/helper-census/src/non_rust.rs`),
  compares a TARGET tree with a BASE tree, both read from git with no rename or
  copy detection (a path is its identity; a moved or copied file is a new path)
  and with line counts read from the blobs themselves (a `binary` or `-diff`
  attribute hides nothing). BASE is the merge-base of the target with the
  integration branch, so deleting a legacy file and re-adding it grown later on
  the same branch is still compared with the integration branch's copy.
  * **Non-Rust code** is a file whose extension is `.py`, `.pyw`, `.pyi`, `.mjs`,
    `.js`, `.cjs`, `.ts`, `.mts`, `.cts`, `.tsx` or `.jsx`, or any other file
    whose first line is a shebang naming `python`/`python3` (any version), `node`,
    `deno` or `bun`. In a shell script (`.sh`, `.bash`, … or a shell shebang) the
    lines of each heredoc fed to one of those runtimes (`python3 - <<'PY'` …
    `PY`) are non-Rust lines of that script.
  1. **Explain.** Every path with non-Rust code that is in TARGET but not in BASE
     carries within its first 40 lines a `# Why not Rust: <reason>` or
     `// Why not Rust: <reason>` comment (any case) whose trimmed reason has at
     least 30 characters, stating concretely why the job cannot be done in Rust.
     Exempt: `vectors/` (frozen vendor corpora), `generated/` (generator output),
     and a file holding only whitespace.
  2. **Ratchet.** A path that exists in BASE under `scripts/`,
     `bindings/python/tests/`, `crates/*/tests/`, `crates/rdf-wasm/js/tests/` or
     `crates/rdf-wasm/js/bench/` may not hold more non-Rust lines in TARGET than
     in BASE (removing it is fine). The shipped surfaces are not ratcheted:
     `crates/rdf-wasm/js/index.mjs`, `index.d.ts` and `src/`, the
     `docs/playground/` app, and the Python package under
     `bindings/python/python/`.

  It runs in three places: the pre-commit hook and the pre-merge-commit hook that
  hands over to it (`make hooks`; the staged index), `make check` (the working
  tree, tracked and untracked unignored files) and CI's `workspace` job (the pull
  request's merge commit, or a push to `main` against the previous `main`). The
  integration branch defaults to `origin/main`; set `PURRDF_RATCHET_BASE` to name
  another.
* **Kernel ring-fence.** `purrdf-core` must never depend on PyO3.
  `purrdf-hash` (the `root` of `layers.toml`) has **zero runtime
  dependencies**. The ring-fenced crates — the rows of `layers.toml` that carry
  an `external` list: `purrdf-lex`, `purrdf-iri`, `purrdf-xsd`,
  `purrdf-events`, `purrdf-deflate`, `purrdf-ed25519` and `purrdf-testkit` —
  depend only on the first-party crates in their `deps` and the external
  packages in their `external` (empty for all but `purrdf-ed25519`, which takes
  `sha2`, and `purrdf-testkit`, which takes `regex-syntax`, `sha2` and
  `wasm-bindgen`). `make rdf-core-hygiene` checks both, reading the root, the
  ring-fenced crates and their rows from `layers.toml`.
* **One home per job.** `helpers-ledger.toml` names the single implementation
  of each job the workspace provides once, what it replaces, and each sanctioned
  second implementation with its criterion and documented reason;
  `scripts/check-shared-helpers.py` (in `make check`, or `make helpers-hygiene`)
  runs `crates/helper-census` and fails on an unresolvable home, a forbidden copy
  outside an enforced job's home, a stale exemption, or a `#[path]` include that
  leaves its crate. `layers.toml` declares which first-party crate may depend on
  which, with `purrdf-hash` the root every crate may use;
  `scripts/check-layers.py` (in `make check`, or `make layer-hygiene`) fails on
  any first-party normal edge it does not allow and on any row the resolved graph
  no longer matches, and `--home-for A B …` names the common dependency
  closest to the given callers — where one implementation they share belongs.
* **Terminal ring-fence: a scanner's character classes are exact, in both
  directions.** They decide **token boundaries**, not merely membership, so
  substituting a Unicode property for a production's enumerated set does not
  just widen the accepted language — it silently *re-tokenizes documents both
  the liberal and the conforming parser accept*. `?s<NBSP>?p` lexed as one
  variable, turning a join into a cross product with exit zero and no
  diagnostic. Every W3C terminal is spelled **once**, in
  `purrdf_lex::terminals` (re-exported as `purrdf_iri::terminals`), with its production cited and its ranges asserted at
  compile time; scanners call it rather than retyping a table.
  `scripts/check-terminal-predicates.py` (in `make check`, or
  `make terminal-hygiene`) refuses a Unicode-property test inside a file that
  holds a character cursor, and names in `SCANNERS` the files whose cursor lives
  in another module. This is **not** "Unicode properties are bad": a production
  that *names* one must be implemented with it, and the gate's `ALLOWLIST` is
  the reasoned ledger for exactly those cases. **Judge the clause, not the
  specification** — one spec answers this differently in different places.
  CommonMark defines a "Unicode whitespace character" and uses it for §6.2
  emphasis flanking, while its blank line (§2.1), ATX heading, thematic break
  and GFM table cell all name space-or-tab; citing "CommonMark" alone settles
  nothing, and doing so once put a false exemption into this file.
* **Everything is wasm-able.** Every release crate (all 31 publishable crates,
  `purrdf-wasm` included) must build for `wasm32-unknown-unknown` — CI
  hard-fails otherwise (`make wasm` locally). Never add a dependency that
  drags in threads, the filesystem, C toolchains, or wall-clock/RNG syscalls
  on the wasm path; crypto stays pure-Rust for exactly this reason.
* **Byte determinism.** Serializers and the GTS writer are byte-deterministic.
  If your change alters emitted bytes, you must update the affected goldens and
  say why in the PR. Never introduce iteration-order, time, or RNG dependence
  into output paths (hashers are the fixed-key `purrdf_hash::fixed::FixedHasher`
  for this reason). No `std` `HashMap`/`HashSet` is left on its random default hasher
  anywhere, tests and benches included: clippy bans the `RandomState` types and
  constructors, and the `fixed-hasher-everywhere` job in `helpers-ledger.toml`
  refuses every other spelling (`HashMap<K, V>`, `HashMap::default()`, `from`,
  `collect`) — name `purrdf_core::FastMap`/`FastSet` or a `FixedState` map.
* **Conformance corpora are the contract**: W3C SPARQL 1.1
  (`crates/sparql-conformance`), the W3C SHACL suite (`vectors/shacl/`), the
  shexTest v2.1.0 suite (`vectors/shexTest/`), the first-party SHACL corpus
  (`crates/shapes/corpus/`), RDFC-1.0 fixtures
  (`crates/rdf/tests/fixtures/rdfc/`), the official JSON-Schema-Test-Suite
  (`crates/jsonschema/tests/suite/`), and the **frozen** GTS vectors in
  `vectors/` (shared byte-exact with the other GTS engines — never regenerate or
  "fix" them here; the GTS wire format is governed in `gmeow-gts`). Harnesses
  assert exact counts and enforce XPASS discipline on their xfail ledgers —
  see [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md) for the scoreboard.
* **PurRDF is NOT an ontology.** Structural Markdown and ordered JSON codecs
  offer explicitly named standard profiles and vocabularies under their shipped
  specifications; callers must select them deliberately or supply a vocabulary.
  There is no implicit namespace fallback. Every other
  vocabulary the library reads or writes (slice manifests, statement-metadata
  downcast, box roles, language retagging, SPARQL extension-function
  namespaces, standpoint predicates, json_schema namespaces) is
  **caller-supplied configuration with no fabricated default**: a feature
  exercised without its vocabulary hard-errors or stays inactive. Never
  hardcode a `blackcatinformatics.ca` namespace in library code (the GMEOW
  ontology is a *consumer*; the dependency arrow never points from purrdf to
  it). Test fixtures use `example.org`. W3C Recommendation terms are the one
  built-in vocabulary: name them through `purrdf_iri::vocab` (and XSD through
  `purrdf_xsd::datatype`), never as string literals — the helper census
  (`w3c-vocab`) refuses a literal that spells a term or namespace of either.
* **Generated artifacts** under `generated/` are projections — never hand-edit;
  regenerate via `make metadata` (`scripts/check-generated.sh` gates drift).
* **Dependency versions live in one place**: `[workspace.dependencies]` in the
  root `Cargo.toml`. Member crates use `dep.workspace = true`. Do not pin a
  version inside a member manifest.
* **Lints are workspace-inherited** (`[workspace.lints]`, clippy pedantic +
  nursery). `cargo clippy --workspace --all-targets` must be warning-free.
  Prefer fixing code over `#[allow]`; a genuinely-right allow must be tightly
  scoped and carry a reason comment.
* **SPDX headers** on every source file: `MIT OR Apache-2.0 OR MulanPSL-2.0` (docs may be
  `CC-BY-4.0`).

## 3. Commands

```bash
make check      # the full local gate: fmt, clippy, build, tests, hygiene
make hooks      # install the pre-commit + pre-merge-commit hooks: rustfmt, the non-Rust ratchet + the fast hygiene gates, on the staged index
make test       # cargo test --workspace
make metadata   # regenerate + verify generated artifacts
make bench      # purrdf_testkit::bench benchmarks (report-only; not a gate)
make scale-corpus  # generate the deterministic scale corpus (streams; stores nothing by default)
make lubm       # the LUBM comparison workload, per entailment regime (report-only; network + JRE)
make watdiv     # the WatDiv comparison workload over a frozen dataset (report-only; network)
make build-profile-hygiene  # prove the gate is compiled the way it claims
```

`scale-corpus`, `lubm` and `watdiv` are the three comparison lanes. None is a
gate and none runs in `make check`: `lubm` needs a JRE and fetches a
GPL-2.0-or-later generator, `watdiv` fetches a 58 MB frozen dataset that expands
past a gigabyte, and neither vendors a byte. They share one implementation of
the laws that make their numbers evidence — `scripts/lane-common.sh` — so a
repair to one is a repair to all three. `docs/BENCHMARKS.md` owns the
parameters, the knobs and the comparison rules.

Toolchain: `rust-toolchain.toml` names a **floating nightly** for development and
for every CI gate. That is an analysis decision, not a licence: nightly clippy
and rustdoc carry lints stable lacks, and its default borrow checker is the
stronger one, so a finding is a real finding rather than a channel artifact.
Floating is the point — a dated channel freezes that surface as of one day, and
every check sharpened afterwards stops being a finding and becomes invisible debt
while the gates still report green. Byte-determinism is no argument for freezing:
it is a property of the code — sorted, deduplicated, explicitly ordered output,
identities content-addressed over this workspace's own declared law — and the
goldens and vectors prove it on whatever compiler runs them. A golden that moved
under a compiler bump would be a serializer defect to fix, not a reason to stop
bumping.

**The source stays nightly-free.** There are zero `#![feature(...)]` attributes
in `crates/` and `bindings/`, and adding one is forbidden. What consumers need
is `rust-version` in `Cargo.toml` (the MSRV, currently 1.98) — a *lower* floor on
the *stable* channel — enforced by the dedicated `msrv` CI job. Never "align" the
MSRV to the dev pin; they answer different questions.

Two traps, both load-bearing:

* `dtolnay/rust-toolchain` selects with `rustup default`, which ranks **below**
  `rust-toolchain.toml`. A workflow that installs one toolchain while the repo
  pins another does not fail — it silently runs the pin. Only
  `RUSTUP_TOOLCHAIN` outranks the file, which is why the `msrv` job and the
  release lanes set it explicitly, and why the `msrv` job also asserts
  `rustc --version` really is 1.98.x.
* `scripts/check-toolchain-pin.py` (in `make check` and CI) fails on any
  workflow whose install step disagrees with the pin without that explicit
  escape, and on a floating channel.

Release lanes (`release-cargo`, `release-npm`, `release-pypi`) build on
**stable** on purpose: nightly's sharper lints buy nothing for an artifact a
consumer installs, and shipping one from an unreleased compiler is risk without
upside.

## 4. Performance discipline

This library is a hot-path backbone. The IR is an immutable, value-interned
dataset (`TermId` = niche-optimized `NonZeroU32`, string arena, store-once
interner). When touching parse/serialize/eval paths:

* **Measure first** — layout and algorithm choices are justified by the
  benches (`crates/rdf-core/benches/ir_layout.rs` et al., on the
  `purrdf_testkit::bench` harness), not by assertion. Add or extend a bench
  when you claim a win.
* Avoid per-token/per-term `String` allocation; move values out of buffers
  instead of cloning; pre-size collections in parse loops.
* Hot maps use the fixed-key `purrdf_hash::fixed::FixedHasher` (`FixedState`,
  or `purrdf_core::FastHasher`; see `crates/rdf-core/src/ir/builder.rs` for
  the canonical store-once interner pattern) — never default SipHash in a hot
  path, and never a randomly-seeded hasher in an output path.

**The gate is compute, so the gate is compiled like it.** `make check` runs the
whole test surface and `make conformance` runs every W3C suite through it, so
both are bounded by the codegen under them: `[profile.dev]` builds everything —
our crates, dependencies, and (named separately, because they do not inherit the
base profile) build scripts and proc-macros — at **opt-level 3**, with
`debug-assertions` and `overflow-checks` **ON**. Those two are orthogonal to
opt-level and are not negotiable: first-party code carries
`#[cfg(debug_assertions)]` bodies that vanish silently with the flag, and
overflow checks are what stop an arithmetic bug in a byte-deterministic codec
becoming a wrong-but-green run. Optimizing the gate does not weaken it: the same
assertions run over the same corpora, on better codegen.

This is not self-enforcing, and it did fail once: four per-crate `opt-level = 2`
tables made the manifest *read* as tuned while twenty-one members compiled at
opt-level 0, because `package."*"` matches dependencies only. So the property is
asserted against the graph Cargo actually resolves, not against the manifest:
`make build-profile-hygiene` (in `make check`) reads the `--unit-graph` of
`cargo test` and `cargo build` and checks every unit's **effective** profile.
Reading effective values is deliberate — it catches a `[profile.*]` table in
`$CARGO_HOME/config.toml` or anywhere on the walk up from the workspace (your
home directory is on that walk), a `CARGO_PROFILE_*` variable, or a `--config`
override, none of which the manifest can see. The `[profile.test]` block
exists and sets only `debug = 0` (test binaries carry no debug info). It must
not set `opt-level`, `debug-assertions` or `overflow-checks` — `test` inherits
those from `dev`, and `scripts/check-build-profiles.py` asserts the inherited
values — and neither it nor `[profile.dev]` may set `lto` or
`codegen-units = 1`: both serialize codegen and inflate link memory, which is
what a constantly-rebuilt, cold-in-CI gate wants least.

## 5. Brand & naming

The project name is written **PurRDF** in prose (never PurrDF/PURRDF); all
package/crate/binary identifiers are lowercase `purrdf`. See
[`docs/BRAND.md`](./docs/BRAND.md). Logo/social assets follow the shared
black-cat family system — `#cat-head-core` is shared verbatim; only the
`#service-triple` group is purrdf-specific.

## 6. Releases

Tag-driven trusted publishing: `rust-v*` → crates.io (31 crates, ordered),
`py-v*` → PyPI (`purrdf`). See [`docs/RELEASE.md`](./docs/RELEASE.md). Version
is single-sourced in `[workspace.package]`. Eleven members never reach
crates.io: `purrdf-capi`, `purrdf-sparql-conformance`,
`purrdf-hash-conformance`, `purrdf-cli`, `purrdf-envelope-probe`,
`purrdf-bench`, `purrdf-alloc-probe`, `purrdf-testkit`, `wasm-link`,
`helper-census`, and `purrdf-python` (PyPI via maturin instead).
`purrdf-alloc-probe` and `purrdf-testkit` are dev-dependencies of published
crates, so their root `[workspace.dependencies]` entries are path-only with
**no `version`** — cargo then strips them from the packaged manifest, which is
the only way `cargo publish`'s dev-dependency-resolving verification step can
succeed.

## 7. Provenance

This repo was extracted from `gmeow-ontology` and `gmeow-gts` — see
[`PROVENANCE.md`](./PROVENANCE.md) for source commits. PurRDF's replacement
surfaces are being completed here, but the downstream `gmeow-ontology` cutover
is not yet complete. Its legacy models are migration evidence only and should
be deleted as each PurRDF replacement is integrated.

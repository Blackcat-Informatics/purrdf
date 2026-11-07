<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Rust engineering guidelines

This is PurRDF's complete Rust engineering guide. Read it with
[AGENTS.md](../../AGENTS.md), whose repository constraints take precedence.
[Cargo.toml](../../Cargo.toml), [layers.toml](../../layers.toml),
[helpers-ledger.toml](../../helpers-ledger.toml) and the actual gate definitions
specify the enforced dependency, implementation and build contracts. The older
[guide location](../RUST_GUIDELINES.md) redirects here.

## Architecture and shared implementations

Keep engine behavior in Rust libraries. Language bindings and command entry
points adapt inputs, ownership and output transport to the shared engine.
Every published crate must build for `wasm32-unknown-unknown`; core algorithms
must not acquire ambient filesystem, network, clock or random-number access.
Hosts supply those capabilities explicitly where the API admits them.

Follow the dependency edges and ring fences in `layers.toml`. Put third-party
versions in root `[workspace.dependencies]`; member crates inherit them.
Dependency examples from other projects do not authorize new packages, optional
dependencies or semantic Cargo features. The sole Cargo feature exception is
the empty, non-semantic `capi` marker documented in AGENTS.md.

Each shared job has one home. Consult `helpers-ledger.toml` before adding an
implementation, and call the existing owner. Examples include:

| Job | Shared home |
| --- | --- |
| JSON, YAML, CBOR and XML syntax | `purrdf_lex::{json, yaml, cbor, xml}` |
| RDF values and term construction | `purrdf_core::TermValue` |
| Fixed-key hash tables | `purrdf_core::hash::table` and `purrdf_hash::fixed::FixedState` |
| Exact integer and decimal arithmetic | `purrdf_xsd` |
| W3C vocabulary constants | `purrdf_iri::vocab` and `purrdf_xsd::datatype` |
| Typed error inspection | `purrdf_hash::error` |

Prefer private or `pub(crate)` implementation details. Public signatures,
variant payloads, ownership, trait requirements and persisted laws are consumer
contracts; preserve the [v4 compatibility baseline](../MIGRATION-4.0.md#compatibility-after-40).
A new abstraction must solve a concrete shared problem rather than provide an
unused alternative to the production path.

## Types, ownership and allocation

Represent validated state with types and constructors when that prevents a
real invalid transition. Borrow existing values where ownership is unnecessary:
prefer `&str`, slices and borrowed views to cloning complete strings or graphs.
Retain original owners when answers or errors must outlive a producer.

Use the shared constructor and conversion macros documented in AGENTS.md.
An empty value matching the derived default derives `Default`; a meaningful
constant constructor may use `purrdf_hash::default_from_new!`. Avoid repeating
thin wrappers or adding a constructor whose only purpose is renaming another.

Admit storage before retaining it through the established allocation interfaces.
Preserve typed resource refusals and transactional behavior. Missing dependencies,
broken invariants and incomplete input must fail actionably, rather than return
an apparently successful partial result. An estimator is not a guaranteed bound
unless its implementation and contract establish that fact.

## Errors and diagnostics

Keep semantic failure data typed and retain the original native cause. Implement
`std::error::Error::source` where a cause exists; the shared
`purrdf_hash::error_sources!` macro handles the supported source patterns.
Use `purrdf_hash::error::StructuredError`, `ErrorVisitor` and `ErrorValue` for
borrowed inspection without rendering or cloning. Producers expose stable codes
and every semantic field through the existing `error_visit!` protocol.

Use human-readable `Display` output at the presentation boundary. Do not replace
native fields with formatted strings to cross an internal or host boundary.
Callers that already hold a typed error can render it explicitly. Genuine
caller/server text remains text; it must not become a fabricated native cause.

Propagate recoverable errors with `?`. Avoid production `unwrap`, `expect` and
panics for invalid input or resource exhaustion. An unavoidable invariant check
needs a local proof explaining why failure cannot occur. Tests may use
`expect` for required fixture setup. A new refusal needs both the rejected case
and an adjacent valid case, so stricter admission does not conceal over-refusal.

## Output and host boundaries

Libraries return values, bytes, diagnostics and evidence through their APIs.
Do not add incidental printing or debug output to an engine path. The CLI and
other command tools intentionally write query results, serialized carriers,
reports and diagnostics; preserve their output protocol and I/O failures.
Tracing packages are not blanket dependencies for libraries or bindings.

For example, a command can write its already-produced payload through the
caller's output sink and propagate the original I/O error:

```rust
fn write_payload(
    output: &mut impl std::io::Write,
    payload: &[u8],
) -> std::io::Result<()> {
    output.write_all(payload)
}
```

Keep host-specific transport and panic containment at the existing boundary.
Preserve native ownership and error fields when adapting C, Python and
JavaScript results. Never put credentials, source documents or large payloads
into incidental logs.

## Determinism, arithmetic and unsafe boundaries

Preserve exact value semantics and the selected arithmetic law. Integer,
decimal, rational and temporal operations use their shared native homes;
a floating approximation cannot silently replace an exact result. Where an API
explicitly selects floating arithmetic or a numerical output grid, preserve its
rounding, accumulation and identity contract.

Serializers, content identities and GTS output are deterministic. Sort or retain
authored order explicitly; no randomized map iteration, time or ambient entropy
may enter emitted bytes. Update affected goldens only for an intentional,
explained contract change. Frozen vendor corpora remain unchanged.

Keep ordinary engine code safe. Existing FFI, SIMD and host-import boundaries
may require narrowly scoped unsafe code; a blanket prohibition must not erase
those supported implementations. Every unsafe operation needs a local safety
argument covering its actual pointer, length, alignment, aliasing and target
requirements. Dispatch must establish the instruction set before invoking a
target-specific kernel, while retaining the declared portable behavior.

## Tests and performance evidence

Test named production behavior, regressions, boundaries and invariants. Prefer
existing behavioral cases. Do not add tests for getters, compiler guarantees,
mechanical forwarding or an implementation's incidental shape. A small fake is
useful when it models a real failure or state transition; it must not merely
assert the sequence of private calls.

Use the repository's shared native support:

- `purrdf_testkit::prop_test!` and `rng` for generated inputs and shrinking.
- `purrdf_testkit::assert_golden!` for byte-exact goldens.
- `purrdf_testkit::temp_dir!`, `temp_file!` and unit-test temporary owners.
- `purrdf_testkit::bench` for benchmarks and `purrdf_alloc_probe` for measured
  allocation windows. Choose thread or process measurement to match execution.

General semantics, grammar, arithmetic, refusal, ownership and conformance run
natively. WASM execution is restricted to actual dispatch/SIMD, shadow-stack,
host-interface or explicitly named frozen byte/digest obligations in
[WASM_TESTING.md](../WASM_TESTING.md). Run the exact named cases; an existing
runner or portable type does not justify duplicating an ordinary suite.
`make wasm` establishes build portability separately.

Measure a claimed optimization against the same behavioral contract. Reuse
work counters, allocation probes and benchmark owners. Compare actual work,
requested storage, retained memory and result fidelity as appropriate; report
host/build conditions and uncertainty. Timing alone does not prove an algorithm
or a resource bound. Benchmark lanes are report-only unless a concrete required
gate says otherwise.

## Focused checks and mandatory gates

During development, select the affected behavior, test, binary, crate or real
consumer with the required profile, target, features and warnings. A shared API
change expands coverage to its actual consumers. It does not automatically
require workspace, all-target or all-feature matrices. Compile real callers
early, then reserve expensive qualification for settled changes.

Required commit hooks run fully with normal signing. Existing release gates
and explicitly required repository/CI gates remain mandatory. A full-workspace
command is appropriate when one of those requirements calls for it or the user
requests it; task completion, review or publication alone does not schedule a
new whole-workspace run. Do not manually duplicate checks that an imminent hook
will execute. Reuse valid results and repeat checks when relevant edits,
failures or unresolved concerns invalidate them.

Read the actual gate definitions before selecting verification. `make check`,
conformance, rustdoc, WASM build and host execution establish different facts;
a pass in one lane does not stand in for another required lane. Missing tools,
skips, zero selected cases and interrupted commands are not passes. Preserve
exit status and state the exact executed scope and remaining acceptance.

## Compiler and profile contracts

Development and ordinary CI use the floating `nightly` channel declared in
[rust-toolchain.toml](../../rust-toolchain.toml). Keep shipping source free of
nightly-only language features. Stable functional release builds and the stable
MSRV job serve different purposes: the release lane builds artifacts, while
the MSRV job checks the lower compiler floor in `[workspace.package]`.
Use the installed, authorized toolchain; do not change compiler selection to
hide a diagnostic or to make a failing check appear successful.

The effective gate profiles retain `opt-level = 3`, debug assertions and overflow
checks. `[profile.test]` inherits those settings and changes only debug info.
Cargo's `debug = 1` means limited debug information; it is distinct from
`debug = "line-tables-only"`. Dependencies and build overrides follow their
actual manifest settings. Changing this explanation does not change those
settings or replace the effective-profile gate.

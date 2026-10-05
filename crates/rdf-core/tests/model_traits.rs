// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned-model trait compatibility and native deep-stack regressions.

#[cfg(not(target_arch = "wasm32"))]
#[path = "support/model_terms.rs"]
mod model_terms;

use purrdf_core::{RdfTerm, RdfTriple};
use purrdf_lex::walk::{Dismantle, Nested, WorkList};

/// A test-only owner whose teardown never invokes the model's recursive drop glue.
struct Fixture(RdfTerm);

impl Dismantle for Fixture {
    fn dismantle(node: Box<Self>) {
        let mut pending: WorkList<RdfTerm, 16> = WorkList::with(node.0);
        while let Some(term) = pending.pop() {
            if let RdfTerm::Triple(triple) = term {
                pending.extend([triple.object, triple.subject]);
            }
        }
    }
}

/// Alternate subject and object nesting so both branches spill the work list.
fn chain(depth: usize, bottom: &str) -> Nested<Fixture> {
    let mut term = RdfTerm::iri(bottom);
    for level in 0..depth {
        let leaf = RdfTerm::blank_node("leaf");
        let (subject, object) = if level % 2 == 0 {
            (term, leaf)
        } else {
            (leaf, term)
        };
        term = RdfTerm::triple(RdfTriple::new(subject, "http://example.org/p", object));
    }
    Nested::new(Fixture(term))
}

struct CountingWriter(usize);
impl std::fmt::Write for CountingWriter {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0 += text.len();
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::fmt::{self, Write as _};
    use std::hash::{Hash, Hasher};
    use std::panic::{RefUnwindSafe, UnwindSafe};

    use purrdf_core::{RdfLiteral, RdfLocation, RdfTerm, RdfTriple};
    use purrdf_testkit::harness::{self, Trial};

    use super::{CountingWriter, Fixture, Nested, chain, model_terms};
    use model_terms::oracle;

    /// Also exercise the heap path beyond the two-level fast path and equal
    /// spellings in distinct variants, without changing the timed fixture corpus.
    fn compatibility_fixtures() -> Vec<(&'static str, RdfTerm)> {
        let mut fixtures = model_terms::fixtures();
        let nested = fixtures.last().expect("nested fixture").1.clone();
        fixtures.push(("same_iri", RdfTerm::iri("same")));
        fixtures.push((
            "heap_path",
            RdfTerm::Triple(Box::new(RdfTriple {
                subject: nested,
                predicate: "http://example.org/deeper".into(),
                object: RdfTerm::blank_node("same"),
                location: Some(RdfLocation::default()),
            })),
        ));
        fixtures
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Bytes(Vec<u8>),
        U8(u8),
        U16(u16),
        U32(u32),
        U64(u64),
        U128(u128),
        Usize(usize),
        I8(i8),
        I16(i16),
        I32(i32),
        I64(i64),
        I128(i128),
        Isize(isize),
    }

    #[derive(Default)]
    struct RecordingHasher(Vec<Event>);

    macro_rules! record_methods {
        ($($method:ident($ty:ty) => $variant:ident),* $(,)?) => {
            $(fn $method(&mut self, value: $ty) {
                self.0.push(Event::$variant(value));
            })*
        };
    }

    impl Hasher for RecordingHasher {
        fn finish(&self) -> u64 {
            0
        }
        fn write(&mut self, bytes: &[u8]) {
            self.0.push(Event::Bytes(bytes.to_vec()));
        }
        record_methods! {
            write_u8(u8) => U8, write_u16(u16) => U16, write_u32(u32) => U32,
            write_u64(u64) => U64, write_u128(u128) => U128, write_usize(usize) => Usize,
            write_i8(i8) => I8, write_i16(i16) => I16, write_i32(i32) => I32,
            write_i64(i64) => I64, write_i128(i128) => I128, write_isize(isize) => Isize,
        }
    }

    fn events(value: &impl Hash) -> Vec<Event> {
        let mut hasher = RecordingHasher::default();
        value.hash(&mut hasher);
        hasher.0
    }

    fn shallow_traits_match_the_independent_compiler_derives() {
        let fixtures = compatibility_fixtures();
        for (label, term) in &fixtures {
            let expected = oracle(term);
            let copied = term.clone();
            assert_eq!(oracle(&copied), expected, "clone {label}");
            assert_eq!(events(term), events(&expected), "hash events {label}");
            assert_eq!(
                format!("{term:?}"),
                format!("{expected:?}"),
                "debug {label}"
            );
            assert_eq!(
                format!("{term:#?}"),
                format!("{expected:#?}"),
                "pretty {label}"
            );
            for (other_label, other) in &fixtures {
                assert_eq!(
                    term == other,
                    expected == oracle(other),
                    "eq {label}/{other_label}"
                );
            }
        }
        let RdfTerm::Triple(mut altered) = fixtures.last().expect("nested fixture").1.clone()
        else {
            panic!("the last fixture is a triple")
        };
        altered.location = None;
        let altered = RdfTerm::Triple(altered);
        let original = &fixtures.last().expect("nested fixture").1;
        assert_ne!(altered, *original);
        assert_eq!(events(&altered), events(&oracle(&altered)));
    }

    fn public_construction_destructuring_and_traits_are_preserved() {
        fn assert_traits<
            T: Clone + Eq + Hash + fmt::Debug + Send + Sync + Unpin + UnwindSafe + RefUnwindSafe,
        >() {
        }
        assert_traits::<RdfTerm>();
        assert_traits::<RdfTriple>();
        let term = RdfTerm::Triple(Box::new(RdfTriple {
            subject: RdfTerm::Iri("s".into()),
            predicate: "p".into(),
            object: RdfTerm::Literal(RdfLiteral::simple("o")),
            location: Some(RdfLocation::default()),
        }));
        match term {
            RdfTerm::Triple(triple) => {
                let RdfTriple {
                    subject,
                    predicate,
                    object,
                    location,
                } = *triple;
                assert!(matches!(subject, RdfTerm::Iri(_)));
                assert_eq!(predicate, "p");
                assert!(matches!(object, RdfTerm::Literal(_)));
                assert_eq!(location, Some(RdfLocation::default()));
            }
            RdfTerm::Iri(_) | RdfTerm::BlankNode(_) | RdfTerm::Literal(_) => {
                panic!("constructed a triple")
            }
        }
    }

    fn fixture_construction_and_teardown_are_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || drop(chain(100_000, "bottom")))
            .expect("small stack starts");
    }

    fn clone_is_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || {
            let original = chain(100_000, "bottom");
            eprintln!("fixture built; entering Clone");
            let copied = Nested::new(Fixture(original.0.clone()));
            let depth = copied.0.try_fold(
                |_| Ok::<_, std::convert::Infallible>(0_usize),
                |_| Ok(0),
                |subject, _, object| Ok(1 + subject.max(object)),
            );
            assert_eq!(depth, Ok(100_000));
        })
        .expect("small stack starts");
    }

    fn equality_is_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || {
            let a = chain(100_000, "bottom");
            let b = chain(100_000, "bottom");
            let different = chain(100_000, "different");
            eprintln!("fixtures built; entering PartialEq");
            // A failed deep comparison reports a bounded message, not two 100k
            // trees through assert_eq!'s diagnostic formatter.
            let equal = a.0 == b.0;
            let unequal = a.0 != different.0;
            assert!(equal, "independently built deep terms compare equal");
            assert!(unequal, "the differing bottom leaf is detected");
        })
        .expect("small stack starts");
    }

    fn hash_is_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || {
            let term = chain(100_000, "bottom");
            eprintln!("fixture built; entering Hash");
            let mut hasher = purrdf_hash::fixed::FixedHasher::default();
            term.0.hash(&mut hasher);
            std::hint::black_box(hasher.finish());
        })
        .expect("small stack starts");
    }

    fn debug_is_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || {
            let term = chain(100_000, "bottom");
            let mut writer = CountingWriter(0);
            eprintln!("fixture built; entering Debug");
            write!(writer, "{:?}", term.0).expect("counting writer accepts bytes");
            assert!(writer.0 > 100_000);
        })
        .expect("small stack starts");
    }

    fn debug_propagates_formatter_errors() {
        struct RefusingWriter;
        impl fmt::Write for RefusingWriter {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        for (_, term) in compatibility_fixtures() {
            assert!(write!(RefusingWriter, "{term:?}").is_err());
            assert!(write!(RefusingWriter, "{term:#?}").is_err());
        }
    }

    pub(super) fn run() -> std::process::ExitCode {
        harness::main(vec![
            Trial::test(
                "shallow_traits_match_the_independent_compiler_derives",
                || {
                    shallow_traits_match_the_independent_compiler_derives();
                    Ok(())
                },
            ),
            Trial::test(
                "public_construction_destructuring_and_traits_are_preserved",
                || {
                    public_construction_destructuring_and_traits_are_preserved();
                    Ok(())
                },
            ),
            Trial::test(
                "fixture_construction_and_teardown_are_stack_safe_at_100k",
                || {
                    fixture_construction_and_teardown_are_stack_safe_at_100k();
                    Ok(())
                },
            ),
            Trial::test("clone_is_stack_safe_at_100k", || {
                clone_is_stack_safe_at_100k();
                Ok(())
            }),
            Trial::test("equality_is_stack_safe_at_100k", || {
                equality_is_stack_safe_at_100k();
                Ok(())
            }),
            Trial::test("hash_is_stack_safe_at_100k", || {
                hash_is_stack_safe_at_100k();
                Ok(())
            }),
            Trial::test("debug_is_stack_safe_at_100k", || {
                debug_is_stack_safe_at_100k();
                Ok(())
            }),
            Trial::test("debug_propagates_formatter_errors", || {
                debug_propagates_formatter_errors();
                Ok(())
            }),
        ])
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    native::run()
}

#[cfg(target_arch = "wasm32")]
fn owned_term_walks_stay_inside_the_wasm_shadow_stack_floor() {
    use std::fmt::Write as _;
    use std::hash::{Hash, Hasher};
    use std::hint::black_box;

    use purrdf_stack::{
        Context, MARGIN_BYTES, StackError, on_stack_scoped, remaining, replace_context,
    };

    fn context() -> Context {
        let current = replace_context(Context::on_floor(0));
        replace_context(current);
        current
    }

    const BYTES: usize = 2 * MARGIN_BYTES;
    let available = remaining();
    assert!(available >= BYTES);
    let caller = context();
    let term = chain(100_000, "bottom");
    on_stack_scoped(BYTES, || {
        let left = remaining();
        assert!(left <= BYTES && left + MARGIN_BYTES >= BYTES);
        let copied = Nested::new(Fixture(black_box(&term.0).clone()));
        black_box(black_box(&term.0) == black_box(&copied.0));
        let mut hasher = purrdf_hash::fixed::FixedHasher::default();
        black_box(&term.0).hash(&mut hasher);
        black_box(hasher.finish());
        let mut sink = CountingWriter(0);
        write!(sink, "{:?}", black_box(&term.0)).expect("the sink accepts bytes");
        black_box(sink.0);
        assert!(remaining() <= BYTES);
    })
    .expect("the actual inline shadow-stack span is admitted");
    assert_eq!(context(), caller);

    let requested = remaining().saturating_add(1024 * 1024);
    let mut ran = false;
    match on_stack_scoped(requested, || ran = true) {
        Err(StackError::ExceedsFloor {
            requested: reported,
            available,
        }) => {
            assert_eq!(reported, requested);
            assert!(available < requested && !ran);
        }
        other => panic!("the over-floor neighbor must refuse before running: {other:?}"),
    }
    assert_eq!(context(), caller);
}

#[cfg(target_arch = "wasm32")]
purrdf_testkit::harness_main!(owned_term_walks_stay_inside_the_wasm_shadow_stack_floor);

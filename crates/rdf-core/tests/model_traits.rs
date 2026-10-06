// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned-model trait compatibility and native deep-stack regressions.

#[path = "support/model_terms.rs"]
mod model_terms;

use purrdf_core::{RdfTerm, RdfTriple};

/// Alternate subject and object nesting so both branches spill the work list.
#[cfg(not(target_arch = "wasm32"))]
fn chain(depth: usize, bottom: &str) -> RdfTerm {
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
    term
}

/// [`chain`] with every option a quoted triple and a literal can carry: a full
/// source location (every integer width) on each level and a directional
/// language-tagged literal leaf, so deep walks exercise each field and Debug
/// option at every level, not only at the top.
fn located_chain(depth: usize) -> RdfTerm {
    use purrdf_core::{RdfLiteral, RdfLocation, RdfTextDirection};
    let mut term = RdfTerm::literal(RdfLiteral {
        lexical_form: "bottom\n\"λ\"".into(),
        datatype: None,
        language: Some("en".into()),
        direction: Some(RdfTextDirection::Ltr),
    });
    for level in 0..depth {
        let leaf = RdfTerm::literal(RdfLiteral::typed("7", "http://example.org/t"));
        let (subject, object) = if level % 2 == 0 {
            (term, leaf)
        } else {
            (leaf, term)
        };
        let ordinal = u64::try_from(level).expect("a test depth fits u64");
        let mut triple = RdfTriple::new(subject, "http://example.org/p", object);
        triple.location = Some(RdfLocation {
            path: Some("a\nfile".into()),
            line: Some(ordinal * 0x1_0001),
            column: Some(u32::MAX - u32::try_from(level).expect("a test depth fits u32")),
            logical: None,
            subject: Some("subject".into()),
            gts_term_id: Some(ordinal),
            gts_quad_index: None,
            gts_reifier_id: Some(u64::MAX - ordinal),
            gts_frame_index: Some(0xabc),
            gts_segment_index: Some(0),
        });
        term = RdfTerm::triple(triple);
    }
    term
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

    use super::{CountingWriter, chain, located_chain, model_terms};
    use model_terms::oracle;

    /// Also exercise the heap walks beyond the four directly walked levels and
    /// equal spellings in distinct variants, without changing the timed corpus.
    fn compatibility_fixtures() -> Vec<(&'static str, RdfTerm)> {
        let mut fixtures = model_terms::fixtures();
        let deepest = fixtures.last().expect("four-level fixture").1.clone();
        let flat = fixtures
            .iter()
            .find(|(label, _)| *label == "triple")
            .expect("flat triple fixture")
            .1
            .clone();
        fixtures.push(("same_iri", RdfTerm::iri("same")));
        fixtures.push((
            "subject_only",
            RdfTerm::triple(RdfTriple::new(
                flat.clone(),
                "http://example.org/subject-only",
                RdfTerm::blank_node("same"),
            )),
        ));
        let mut object_only =
            RdfTriple::new(RdfTerm::iri("same"), "http://example.org/object-only", flat);
        object_only.location = Some(RdfLocation::default());
        fixtures.push(("object_only", RdfTerm::triple(object_only)));
        fixtures.push((
            "heap_nullable_fields",
            RdfTerm::triple(RdfTriple::new(
                deepest.clone(),
                "http://example.org/nullable-fields",
                RdfTerm::literal(RdfLiteral::simple("content\0\n🦀\"\\")),
            )),
        ));
        fixtures.push(("located_depth_5", located_chain(5)));
        fixtures.push((
            "heap_path",
            RdfTerm::Triple(Box::new(RdfTriple {
                subject: deepest,
                predicate: "http://example.org/deeper".into(),
                object: RdfTerm::blank_node("same"),
                location: Some(RdfLocation {
                    line: Some(0x1ab),
                    column: Some(0x2cd),
                    gts_term_id: Some(0x3ef),
                    gts_quad_index: Some(0x456),
                    gts_reifier_id: Some(0x789),
                    gts_frame_index: Some(0xabc),
                    gts_segment_index: Some(0xdef),
                    ..RdfLocation::default()
                }),
            })),
        ));
        fixtures
    }

    /// One rendering of a value under one literal format spec.
    type Render = fn(&dyn fmt::Debug) -> String;

    /// Each literal spec beside the rendering it names.
    macro_rules! renders {
        ($($spec:literal),+ $(,)?) => {
            [$((stringify!($spec), (|value| format!($spec, value)) as Render)),+]
        };
    }

    /// Every format spec the grammar admits for `Debug`, generated as the cross
    /// product of fill and alignment, sign, `#`, `0`, width and precision
    /// (literal and dynamic) and the three `Debug` modes. Newline fill is in it,
    /// so the pretty form's padding indentation is exercised too.
    fn spec_matrix() -> Vec<(&'static str, Render)> {
        let mut specs: Vec<(&'static str, Render)> = Vec::new();
        macro_rules! push {
            ($spec:expr) => {
                specs.push(($spec, |value| format!($spec, value)));
            };
            ($spec:expr, $($arg:ident = $value:expr),+) => {
                specs.push(($spec, |value| format!($spec, value, $($arg = $value),+)));
            };
        }
        macro_rules! dimensions {
            ($fill:literal, $($flags:literal)+, $mode:literal) => {
                push!(concat!("{:", $fill, $($flags,)+ $mode, "}"));
                push!(concat!("{:", $fill, $($flags,)+ "1", $mode, "}"));
                push!(concat!("{:", $fill, $($flags,)+ "30", $mode, "}"));
                push!(concat!("{:", $fill, $($flags,)+ ".0", $mode, "}"));
                push!(concat!("{:", $fill, $($flags,)+ ".3", $mode, "}"));
                push!(concat!("{:", $fill, $($flags,)+ "1.0", $mode, "}"));
                push!(concat!("{:", $fill, $($flags,)+ "30.3", $mode, "}"));
                push!(
                    concat!("{:", $fill, $($flags,)+ "width$.precision$", $mode, "}"),
                    width = 0,
                    precision = 0
                );
            };
        }
        macro_rules! modes {
            ($fill:literal, $($flags:literal)+) => {
                dimensions!($fill, $($flags)+, "?");
                dimensions!($fill, $($flags)+, "x?");
                dimensions!($fill, $($flags)+, "X?");
            };
        }
        macro_rules! flags {
            ($fill:literal) => {
                modes!($fill, "");
                modes!($fill, "+");
                modes!($fill, "-");
                modes!($fill, "#");
                modes!($fill, "+#");
                modes!($fill, "-#");
                modes!($fill, "0");
                modes!($fill, "+0");
                modes!($fill, "-0");
                modes!($fill, "#" "0");
                modes!($fill, "+#" "0");
                modes!($fill, "-#" "0");
            };
        }
        flags!("");
        flags!("<");
        flags!("^");
        flags!(">");
        flags!("*<");
        flags!("*^");
        flags!("*>");
        flags!("\0<");
        flags!("\0^");
        flags!("\0>");
        flags!("\n<");
        flags!("\n^");
        flags!("\n>");
        flags!("🦀<");
        flags!("🦀^");
        flags!("🦀>");
        specs
    }

    /// Record every spec of the matrix whose bytes differ from the derive's.
    /// `format!` panics when a `Debug` implementation reports an error, so a
    /// refused formatter state fails this check rather than passing silently.
    fn debug_specs_match_the_derives(
        value: &dyn fmt::Debug,
        expected: &dyn fmt::Debug,
        label: &str,
        kind: &str,
        specs: &[(&'static str, Render)],
        mismatches: &mut Vec<String>,
    ) {
        for (spec, render) in specs {
            if render(value) != render(expected) {
                mismatches.push(format!("{kind} {label}: {spec:?}"));
            }
        }
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

    /// The ordered event stream of [`RecordingHasher`] in constant memory, for
    /// terms too deep to record: each event's kind, length and bytes feed two
    /// independently based FNV-1a streams, and the events are counted.
    use purrdf_hash::fnv;

    struct EventDigest {
        streams: [u64; 2],
        count: u64,
    }

    impl EventDigest {
        fn event(&mut self, kind: u8, bytes: &[u8]) {
            self.count += 1;
            let length = u64::try_from(bytes.len())
                .expect("a slice length fits u64")
                .to_le_bytes();
            for stream in &mut self.streams {
                *stream = fnv::fold(fnv::fold(fnv::fold(*stream, &[kind]), &length), bytes);
            }
        }
    }

    macro_rules! digest_methods {
        ($($method:ident($ty:ty) => $kind:literal),* $(,)?) => {
            $(fn $method(&mut self, value: $ty) {
                self.event($kind, &value.to_le_bytes());
            })*
        };
    }

    impl Hasher for EventDigest {
        fn finish(&self) -> u64 {
            self.streams[0]
        }
        fn write(&mut self, bytes: &[u8]) {
            self.event(0, bytes);
        }
        digest_methods! {
            write_u8(u8) => 1, write_u16(u16) => 2, write_u32(u32) => 3,
            write_u64(u64) => 4, write_u128(u128) => 5, write_usize(usize) => 6,
            write_i8(i8) => 7, write_i16(i16) => 8, write_i32(i32) => 9,
            write_i64(i64) => 10, write_i128(i128) => 11, write_isize(isize) => 12,
        }
    }

    fn event_digest(value: &impl Hash) -> ([u64; 2], u64) {
        let mut digest = EventDigest {
            streams: [fnv::BASIS, fnv::fold(fnv::BASIS, b"second stream")],
            count: 0,
        };
        value.hash(&mut digest);
        (digest.streams, digest.count)
    }

    fn shallow_traits_match_the_independent_compiler_derives() {
        let fixtures = compatibility_fixtures();
        let specs = spec_matrix();
        let mut debug_mismatches = Vec::new();
        for (label, term) in &fixtures {
            let expected = oracle(term);
            let copied = term.clone();
            assert_eq!(oracle(&copied), expected, "clone {label}");
            assert_eq!(term, &copied, "independently owned equal {label}");
            let mut altered = term.clone();
            let mut leaf = &mut altered;
            while let RdfTerm::Triple(triple) = leaf {
                leaf = if !matches!(triple.subject, RdfTerm::Triple(_))
                    && matches!(triple.object, RdfTerm::Triple(_))
                {
                    &mut triple.object
                } else {
                    &mut triple.subject
                };
            }
            match leaf {
                RdfTerm::Iri(value) | RdfTerm::BlankNode(value) => value.push('\0'),
                RdfTerm::Literal(value) => value.lexical_form.push('\0'),
                RdfTerm::Triple(_) => unreachable!("the cursor reached a leaf"),
            }
            assert_ne!(term, &altered, "nested leaf mismatch {label}");
            assert_ne!(expected, oracle(&altered), "independent mismatch {label}");
            if let (
                RdfTerm::Triple(triple),
                model_terms::derived::RdfTerm::Triple(expected_triple),
                RdfTerm::Triple(altered_triple),
            ) = (term, &expected, &altered)
            {
                let copied_triple = triple.as_ref().clone();
                assert_eq!(triple.as_ref(), &copied_triple, "struct equal {label}");
                assert_ne!(
                    triple.as_ref(),
                    altered_triple.as_ref(),
                    "struct mismatch {label}"
                );
                assert_eq!(
                    oracle(&RdfTerm::triple(copied_triple)),
                    expected,
                    "struct clone {label}"
                );
                assert_eq!(
                    events(triple.as_ref()),
                    events(expected_triple.as_ref()),
                    "struct hash events {label}"
                );
                assert_eq!(
                    format!("{triple:?}"),
                    format!("{expected_triple:?}"),
                    "struct debug {label}"
                );
                assert_eq!(
                    format!("{triple:#?}"),
                    format!("{expected_triple:#?}"),
                    "struct pretty {label}"
                );
                debug_specs_match_the_derives(
                    triple.as_ref(),
                    expected_triple.as_ref(),
                    label,
                    "struct",
                    &specs,
                    &mut debug_mismatches,
                );
            }
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
            debug_specs_match_the_derives(
                term,
                &expected,
                label,
                "term",
                &specs,
                &mut debug_mismatches,
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
        assert!(
            debug_mismatches.is_empty(),
            "Debug options differ from the independent derives:\n{}",
            debug_mismatches.join("\n"),
        );
    }

    /// Construction, enum destructuring, in-place field access and the trait set
    /// are unchanged; an owned triple is taken apart with `into_parts`, because
    /// its iterative `Drop` forbids moving fields out of it.
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
            RdfTerm::Triple(mut triple) => {
                let RdfTriple {
                    subject: RdfTerm::Iri(subject),
                    predicate: borrowed,
                    ..
                } = &mut *triple
                else {
                    panic!("constructed an IRI subject")
                };
                subject.push('!');
                assert_eq!(borrowed, "p");
                let (subject, predicate, object, location) = triple.into_parts();
                assert_eq!(subject, RdfTerm::Iri("s!".into()));
                assert_eq!(predicate, "p");
                assert!(matches!(object, RdfTerm::Literal(_)));
                assert_eq!(location, Some(RdfLocation::default()));
            }
            RdfTerm::Iri(_) | RdfTerm::BlankNode(_) | RdfTerm::Literal(_) => {
                panic!("constructed a triple")
            }
        }
    }

    /// Every nesting shape drops on a small stack: alternating subject and
    /// object chains, a fully located chain, and a balanced tree whose work list
    /// spills on both sides; `into_parts` hands a deep subject out intact.
    fn drop_is_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || {
            drop(chain(100_000, "bottom"));
            drop(located_chain(100_000));
            let mut tree = RdfTerm::iri("leaf");
            for _ in 0..16 {
                tree = RdfTerm::triple(RdfTriple::new(tree.clone(), "http://example.org/p", tree));
            }
            drop(tree);
            let RdfTerm::Triple(outer) = RdfTerm::triple(RdfTriple::new(
                chain(100_000, "bottom"),
                "http://example.org/p",
                RdfTerm::iri("o"),
            )) else {
                unreachable!("constructed a triple")
            };
            let (subject, _, _, _) = outer.into_parts();
            assert!(
                subject == chain(100_000, "bottom"),
                "the deep subject is handed out intact"
            );
            drop(subject);
        })
        .expect("small stack starts");
    }

    fn clone_is_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || {
            let original = located_chain(100_000);
            eprintln!("fixture built; entering Clone");
            let copied = original.clone();
            let depth = copied.try_fold(
                |_| Ok::<_, std::convert::Infallible>(0_usize),
                |_| Ok(0),
                |subject, _, object| Ok(1 + subject.max(object)),
            );
            assert_eq!(depth, Ok(100_000));
            // Every level's location and leaf survive: the copy equals its
            // original, and a neighbour differing at the bottom does not.
            assert!(copied == original, "the deep copy equals its original");
            let mut bottom = &copied;
            let mut locations = 0_usize;
            while let RdfTerm::Triple(triple) = bottom {
                locations += usize::from(triple.location.is_some());
                bottom = if matches!(triple.subject, RdfTerm::Triple(_)) {
                    &triple.subject
                } else {
                    &triple.object
                };
            }
            assert_eq!(locations, 100_000, "every copied level keeps its location");
            assert!(matches!(bottom, RdfTerm::Literal(_)));
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
            let equal = a == b;
            let unequal = a != different;
            assert!(equal, "independently built deep terms compare equal");
            assert!(unequal, "the differing bottom leaf is detected");
        })
        .expect("small stack starts");
    }

    fn hash_is_stack_safe_at_100k() {
        fn finish(term: &RdfTerm) -> u64 {
            let mut hasher = purrdf_hash::fixed::FixedHasher::default();
            term.hash(&mut hasher);
            hasher.finish()
        }
        purrdf_stack::on_stack(256 * 1024, || {
            let a = located_chain(100_000);
            let b = located_chain(100_000);
            eprintln!("fixtures built; entering Hash");
            // Hash agrees with Eq on an independently built equal deep pair,
            // event for event, not only in the final digest.
            assert_eq!(a, b);
            assert_eq!(finish(&a), finish(&b), "equal deep terms hash equally");
            assert_eq!(
                event_digest(&a),
                event_digest(&b),
                "equal deep terms feed identical events"
            );
        })
        .expect("small stack starts");
    }

    fn debug_is_stack_safe_at_100k() {
        purrdf_stack::on_stack(256 * 1024, || {
            let term = located_chain(100_000);
            eprintln!("fixture built; entering Debug");
            for render in [
                |w: &mut CountingWriter, t: &RdfTerm| write!(w, "{t:?}"),
                |w: &mut CountingWriter, t: &RdfTerm| write!(w, "{t:\n>+30.3x?}"),
                |w: &mut CountingWriter, t: &RdfTerm| write!(w, "{t:🦀^-030X?}"),
            ] {
                let mut writer = CountingWriter(0);
                render(&mut writer, &term).expect("every format spec is accepted");
                assert!(writer.0 > 100_000);
            }
            // The pretty form's indentation is quadratic in depth for the derive
            // too, so it is walked at a depth past the direct prefix and the
            // inline work lists that keeps the output small; newline fill takes
            // its rendering path, and width-free padding keeps the lines few.
            let pretty = located_chain(1_500);
            for render in [
                |w: &mut CountingWriter, t: &RdfTerm| write!(w, "{t:#?}"),
                |w: &mut CountingWriter, t: &RdfTerm| write!(w, "{t:\n<+#x?}"),
            ] {
                let mut writer = CountingWriter(0);
                render(&mut writer, &pretty).expect("every format spec is accepted");
                assert!(writer.0 > 1_500);
            }
        })
        .expect("small stack starts");
    }

    /// The independent compiler derive is the oracle at full depth too: on a
    /// stack large enough for its recursion, the 100,000-level terms must clone,
    /// compare, hash (event for event) and print (byte for byte) exactly as the
    /// derive does. The pretty form is compared at a depth whose quadratic
    /// indentation stays small.
    fn deep_traits_match_the_independent_derive() {
        purrdf_stack::on_stack(512 << 20, || {
            let term = located_chain(100_000);
            let expected = oracle(&term);
            eprintln!("deep oracle built; comparing");
            let copied = term.clone();
            assert!(oracle(&copied) == expected, "deep clone");
            let (digest, count) = event_digest(&term);
            assert_eq!(
                (digest, count),
                event_digest(&expected),
                "deep hash events equal the derive's"
            );
            assert!(count > 100_000 * 10, "every level feeds its events");
            assert_ne!(event_digest(&located_chain(99_999)).0, digest);
            let mut altered = located_chain(100_000);
            let mut leaf = &mut altered;
            while let RdfTerm::Triple(triple) = leaf {
                leaf = if matches!(triple.subject, RdfTerm::Triple(_)) {
                    &mut triple.subject
                } else {
                    &mut triple.object
                };
            }
            let RdfTerm::Literal(literal) = leaf else {
                panic!("the bottom is a literal")
            };
            literal.lexical_form.push('\0');
            assert!(term != altered && expected != oracle(&altered));
            for (spec, render) in renders!("{:?}", "{:x?}", "{:+012X?}", "{:\n>30x?}") {
                assert!(render(&term) == render(&expected), "deep {spec}");
            }
            let pretty = located_chain(64);
            let pretty_expected = oracle(&pretty);
            for (spec, render) in renders!("{:#?}", "{:\n>#30x?}", "{:🦀^+#30.3X?}", "{:\n<-#030?}")
            {
                assert!(
                    render(&pretty) == render(&pretty_expected),
                    "pretty depth 64 {spec}"
                );
            }
        })
        .expect("large stack starts");
    }

    fn debug_propagates_formatter_errors() {
        struct RefusingWriter(usize);
        impl fmt::Write for RefusingWriter {
            fn write_str(&mut self, text: &str) -> fmt::Result {
                self.0 = self.0.checked_sub(text.len()).ok_or(fmt::Error)?;
                Ok(())
            }
        }
        fn assert_refuses(value: &impl fmt::Debug) {
            macro_rules! check {
                ($($spec:literal),+ $(,)?) => {
                    $(let size = format!($spec, value).len();
                    for remaining in [0, 1, size / 2, size - 1] {
                        let mut writer = RefusingWriter(remaining);
                        assert!(write!(writer, $spec, value).is_err());
                    })+
                };
            }
            check!(
                "{:?}",
                "{:#?}",
                "{:x?}",
                "{:X?}",
                "{:#x?}",
                "{:#X?}",
                "{:+?}",
                "{:-?}",
                "{:12?}",
                "{:012?}",
                "{:.3?}",
                "{:*^12?}",
                "{:+012x?}",
                "{:+#012.3X?}",
                "{:\n>12?}",
                "{:\n>#12?}",
                "{:🦀^12x?}",
            );
        }
        for (_, term) in compatibility_fixtures() {
            assert_refuses(&term);
            if let RdfTerm::Triple(triple) = term {
                assert_refuses(triple.as_ref());
            }
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
            Trial::test("drop_is_stack_safe_at_100k", || {
                drop_is_stack_safe_at_100k();
                Ok(())
            }),
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
            Trial::test("deep_traits_match_the_independent_derive", || {
                deep_traits_match_the_independent_derive();
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

/// The deepest shadow-stack address seen by a `Hasher` or `fmt::Write` callback,
/// each of which runs at the bottom of the walk that calls it.
#[cfg(target_arch = "wasm32")]
mod peak {
    use std::fmt;
    use std::hash::{Hash, Hasher};

    use purrdf_stack::stack_pointer;

    struct SamplingHasher {
        inner: purrdf_hash::fixed::FixedHasher,
        lowest: usize,
    }

    impl Hasher for SamplingHasher {
        fn finish(&self) -> u64 {
            self.inner.finish()
        }
        fn write(&mut self, bytes: &[u8]) {
            self.lowest = self.lowest.min(stack_pointer());
            self.inner.write(bytes);
        }
    }

    struct SamplingWriter {
        bytes: usize,
        lowest: usize,
    }

    impl fmt::Write for SamplingWriter {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            self.lowest = self.lowest.min(stack_pointer());
            self.bytes += text.len();
            Ok(())
        }
    }

    /// The shadow-stack bytes below the caller's frame that hashing `value`
    /// reached, and its digest.
    #[inline(never)]
    pub(super) fn hash(value: &impl Hash) -> (usize, u64) {
        let base = stack_pointer();
        let mut hasher = SamplingHasher {
            inner: purrdf_hash::fixed::FixedHasher::default(),
            lowest: base,
        };
        value.hash(&mut hasher);
        (base - hasher.lowest, hasher.finish())
    }

    /// The shadow-stack bytes below the caller's frame that printing `value`
    /// compactly reached, and the bytes printed.
    #[inline(never)]
    pub(super) fn debug(value: &impl fmt::Debug) -> (usize, usize) {
        use fmt::Write as _;
        let base = stack_pointer();
        let mut writer = SamplingWriter {
            bytes: 0,
            lowest: base,
        };
        write!(writer, "{value:?}").expect("the sampling writer accepts bytes");
        (base - writer.lowest, writer.bytes)
    }
}

/// Measures, on the actual wasm32 shadow stack, that the owned-term walks use
/// a peak independent of depth while the compiler-derived recursion grows per
/// level by enough that a recursive walk could not run at the tested depth at
/// all. The 100,000-level walks, and drops, run inside a scoped floor that a trap would
/// cross: the stack is first in linear memory, so running below its floor
/// wraps the pointer and traps instead of corrupting the heap.
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

    /// The derive's per-level growth, measured between two depths it survives.
    fn per_level(peak: impl Fn(&model_terms::derived::RdfTerm) -> usize) -> usize {
        let shallow = model_terms::oracle(&located_chain(500));
        let deep = model_terms::oracle(&located_chain(1_000));
        (peak(&deep) - peak(&shallow)) / 500
    }

    const DEPTH: usize = 100_000;
    const BYTES: usize = 2 * MARGIN_BYTES;
    let available = remaining();
    assert!(available >= BYTES);
    let caller = context();

    let recursive_hash = per_level(|term| peak::hash(term).0);
    let recursive_debug = per_level(|term| peak::debug(term).0);
    eprintln!(
        "derived recursion: {recursive_hash} B/level Hash, {recursive_debug} B/level Debug; \
         {available} B of shadow stack left"
    );
    // At least one derived walk would need far more than the whole remaining
    // shadow stack at the tested depth: that depth overflows a recursive walk.
    assert!(
        recursive_hash.max(recursive_debug).saturating_mul(DEPTH) > available,
        "the tested depth must overflow the derive's recursion"
    );

    let shallow = located_chain(8);
    let (shallow_hash, _) = peak::hash(&shallow);
    let (shallow_debug, _) = peak::debug(&shallow);
    let term = located_chain(DEPTH);
    let twin = located_chain(DEPTH);
    on_stack_scoped(BYTES, || {
        let left = remaining();
        assert!(left <= BYTES && left + MARGIN_BYTES >= BYTES);
        let copied = black_box(&term).clone();
        assert!(black_box(&term) == black_box(&copied));
        assert!(black_box(&term) == black_box(&twin));
        let (deep_hash, digest) = peak::hash(&term);
        assert_eq!(digest, peak::hash(&twin).1, "equal deep terms hash equally");
        let (deep_debug, printed) = peak::debug(&term);
        assert!(printed > DEPTH);
        eprintln!(
            "owned walks: Hash {shallow_hash} B at depth 8, {deep_hash} B at depth {DEPTH}; \
             Debug {shallow_debug} B at depth 8, {deep_debug} B at depth {DEPTH}"
        );
        // The peak at 100,000 levels is the peak at 8: the walks spend heap.
        assert!(
            deep_hash <= shallow_hash + 256,
            "Hash peak grows with depth"
        );
        assert!(
            deep_debug <= shallow_debug + 256,
            "Debug peak grows with depth"
        );
        let mut hasher = purrdf_hash::fixed::FixedHasher::default();
        black_box(&term).hash(&mut hasher);
        black_box(hasher.finish());
        let mut sink = CountingWriter(0);
        write!(sink, "{:\n>+#30x?}", black_box(&located_chain(64)))
            .expect("the sink accepts bytes");
        black_box(sink.0);
        // Drop, too, takes 100,000 levels apart inside the scoped floor: the
        // copy here, and a fresh chain of each nesting side.
        drop(copied);
        drop(located_chain(DEPTH));
        drop(black_box(RdfTerm::triple(RdfTriple::new(
            located_chain(DEPTH),
            "http://example.org/p",
            located_chain(DEPTH),
        ))));
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

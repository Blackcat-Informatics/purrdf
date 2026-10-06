// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded replacement through the shared XPath replacement cursor.

use std::borrow::Cow;

use super::r#match::Vm;
use super::{Budget, CompiledPattern, Error, Limits, Resource};
use crate::xsd_regex::replace::{Cursor, ParseFailure, Part};

fn next<'a>(cursor: &mut Cursor<'a>, budget: &mut Budget) -> Result<Option<Part<'a>>, Error> {
    cursor
        .next(&mut |amount| budget.charge(Resource::MatchSteps, amount))
        .map_err(|failure| match failure {
            ParseFailure::Replacement(error) => error.into(),
            ParseFailure::Work(refusal) => refusal.into(),
        })
}

fn append(out: &mut String, text: &str, budget: &mut Budget) -> Result<(), Error> {
    budget.charge_wide(Resource::OutputBytes, text.len() as u128)?;
    let copy = if text.len() > out.capacity() - out.len() {
        out.len() as u128
    } else {
        0
    };
    budget.charge_wide(Resource::MatchSteps, text.len() as u128 + copy)?;
    out.try_reserve(text.len()).map_err(|_| Error::Allocation {
        resource: Resource::OutputBytes,
        units: budget.used(Resource::OutputBytes),
    })?;
    out.push_str(text);
    Ok(())
}

impl CompiledPattern {
    /// Replace every nonoverlapping ordered match under this dated XPath law.
    ///
    /// Pattern admission, replacement validation, the empty-string check,
    /// searching and expansion share one request's finite work fuel. The q flag
    /// makes replacement text verbatim; other calls use the same replacement
    /// grammar as the compatibility surface. Captures are reset for each match.
    ///
    /// # Errors
    ///
    /// Empty-match patterns raise FORX0003; invalid replacement text raises
    /// FORX0004. Work, live storage and output exhaustion are distinct operational
    /// refusals. No partial output is returned after any failure.
    // Out of line: a host evaluator that dispatches to the matcher must not
    // absorb the matcher's frames into its own, which every nesting level pays.
    #[inline(never)]
    pub fn replace_all<'h>(
        &self,
        input: &'h str,
        replacement: &str,
        limits: Limits,
    ) -> Result<Cow<'h, str>, Error> {
        self.admit(limits)?;
        let mut budget = Budget::new(limits);
        if !self.modes.quoted {
            let mut template = Cursor::new(replacement, self.captures);
            while next(&mut template, &mut budget)?.is_some() {}
        }
        let mut empty = Vm::new(self, "", limits);
        *empty.budget() = budget;
        if empty.find_from(0)?.is_some() {
            return Err(Error::EmptyMatch);
        }
        let mut vm = Vm::new(self, input, limits);
        *vm.budget() = std::mem::replace(empty.budget(), Budget::new(limits));
        let mut out = String::new();
        let mut position = 0;
        let mut changed = false;
        loop {
            let Some(captures) = vm.find_from(position)? else {
                if !changed {
                    // The unchanged input is returned borrowed: no output
                    // byte is produced, so none is admitted.
                    return Ok(Cow::Borrowed(input));
                }
                append(&mut out, &input[position..], vm.budget())?;
                return Ok(Cow::Owned(out));
            };
            let matched = captures.get(0).expect("successful match has capture zero");
            debug_assert!(matched.end > matched.start, "empty-match guard was passed");
            append(&mut out, &input[position..matched.start], vm.budget())?;
            if self.modes.quoted {
                append(&mut out, replacement, vm.budget())?;
            } else {
                let mut template = Cursor::new(replacement, self.captures);
                while let Some(part) = next(&mut template, vm.budget())? {
                    let text = match part {
                        Part::Literal(text) => text,
                        Part::Group(Some(number)) => {
                            captures.get(number).map_or("", |span| &input[span])
                        }
                        Part::Group(None) => "",
                    };
                    append(&mut out, text, vm.budget())?;
                }
            }
            position = matched.end;
            changed = true;
            // No previous match's capture vector survives the next search.
            drop(captures);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::xsd_regex::ReplacementError;
    use crate::xsd_regex::xpath::{Profile, Refusal, compile};

    fn pattern(source: &str, flags: &str) -> CompiledPattern {
        compile(Profile::Xpath31, source, flags, Limits::new()).unwrap()
    }

    #[test]
    fn replace_obeys_ordered_matches_and_resets_every_capture() {
        assert_eq!(
            pattern("bra", "")
                .replace_all("abracadabra", "*", Limits::new())
                .unwrap(),
            "a*cada*"
        );
        assert_eq!(
            pattern("(ab)|(a)", "")
                .replace_all("aba", "$2$1", Limits::new())
                .unwrap(),
            "aba"
        );
        assert_eq!(
            pattern("(a|aa)", "")
                .replace_all("aa", "$1!", Limits::new())
                .unwrap(),
            "a!a!"
        );
        assert_eq!(
            pattern("(é)", "")
                .replace_all("xééy", "$1$0", Limits::new())
                .unwrap(),
            "xééééy"
        );
        assert_eq!(
            pattern("^(a+?)(a*)$", "")
                .replace_all("aaa", "$2:$1", Limits::new())
                .unwrap(),
            "aa:a"
        );
    }

    #[test]
    fn shared_replacement_escapes_and_exact_digit_resolution_are_preserved() {
        let program = pattern("(a)(b)(c)", "");
        for (replacement, expected) in [
            ("$3$2$1", "cba"),
            ("$0", "abc"),
            ("x$5y", "xy"),
            ("$23", "b3"),
            ("$00000000000000000000000000000000000000002", "b"),
            (r"$2\$1\\", "b$1\\"),
            (
                "$99999999999999999999999999999999999999",
                "9999999999999999999999999999999999999",
            ),
        ] {
            assert_eq!(
                program
                    .replace_all("abc", replacement, Limits::new())
                    .unwrap(),
                expected,
                "{replacement}"
            );
        }
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            let program = compile(profile, "(a)", "", Limits::new()).unwrap();
            assert_eq!(
                program.replace_all("a", r"\$$1", Limits::new()).unwrap(),
                "$a"
            );
        }
    }

    #[test]
    fn forx0003_forx0004_and_q_have_distinct_neighbors() {
        for source in ["", "a*", "a|", "(a)?", "^$"] {
            assert_eq!(
                pattern(source, "")
                    .replace_all("xyz", "ok", Limits::new())
                    .unwrap_err(),
                Error::EmptyMatch
            );
        }
        assert_eq!(
            pattern("", "q")
                .replace_all("xyz", "$", Limits::new())
                .unwrap_err(),
            Error::EmptyMatch
        );
        let program = pattern("a", "");
        for input in ["a", "z"] {
            assert!(matches!(
                program.replace_all(input, "$x", Limits::new()),
                Err(Error::Replacement(ReplacementError::DollarWithoutGroup {
                    offset: 0
                }))
            ));
            assert!(matches!(
                program.replace_all(input, r"\n", Limits::new()),
                Err(Error::Replacement(ReplacementError::UnescapedBackslash {
                    offset: 0
                }))
            ));
        }
        assert_eq!(
            pattern("a", "q")
                .replace_all("a", r"$x\n", Limits::new())
                .unwrap(),
            r"$x\n"
        );
        assert_eq!(program.replace_all("a", r"\$", Limits::new()).unwrap(), "$");
    }

    #[test]
    fn empty_match_check_and_replacement_validation_preserve_operational_refusal() {
        for (source, replacement) in [("a*", "ok"), ("a", "$x")] {
            assert!(matches!(
                pattern(source, "").replace_all(
                    "a",
                    replacement,
                    Limits::new().with(Resource::MatchSteps, 0)
                ),
                Err(Error::Resource(Refusal {
                    resource: Resource::MatchSteps,
                    ..
                }))
            ));
        }
        let complex = pattern("(a?){1000}", "");
        assert!(matches!(
            complex.replace_all("z", "ok", Limits::new().with(Resource::MatchSteps, 100)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSteps,
                ..
            }))
        ));
    }

    #[test]
    fn output_admission_counts_utf8_and_refuses_the_whole_result() {
        let program = pattern("a", "");
        assert_eq!(
            program
                .replace_all("a", "é", Limits::new().with(Resource::OutputBytes, 2))
                .unwrap(),
            "é"
        );
        assert!(matches!(
            program.replace_all("a", "é", Limits::new().with(Resource::OutputBytes, 1)),
            Err(Error::Resource(Refusal {
                resource: Resource::OutputBytes,
                required: 2,
                ..
            }))
        ));
        assert!(matches!(
            program.replace_all("xaa", "é", Limits::new().with(Resource::OutputBytes, 4)),
            Err(Error::Resource(Refusal {
                resource: Resource::OutputBytes,
                required: 5,
                ..
            }))
        ));
        assert_eq!(
            program
                .replace_all("xaa", "é", Limits::new().with(Resource::OutputBytes, 5))
                .unwrap(),
            "xéé"
        );
        // No match borrows the input unchanged, so it produces no output bytes,
        // even when the input is larger than the bound.
        for limit in [0, 1] {
            assert!(matches!(
                program.replace_all(
                    "é",
                    "unused",
                    Limits::new().with(Resource::OutputBytes, limit)
                ),
                Ok(Cow::Borrowed("é"))
            ));
        }
        // Its neighbour with one match produces the same unmatched text and is
        // charged for every byte it copies.
        assert!(matches!(
            program.replace_all("éa", "", Limits::new().with(Resource::OutputBytes, 1)),
            Err(Error::Resource(Refusal {
                resource: Resource::OutputBytes,
                required: 2,
                ..
            }))
        ));
        assert_eq!(
            program
                .replace_all("éa", "", Limits::new().with(Resource::OutputBytes, 2))
                .unwrap(),
            "é"
        );
        assert!(matches!(
            program.replace_all("z", "unused", Limits::new()),
            Ok(Cow::Borrowed("z"))
        ));
    }

    #[test]
    fn counted_ambiguous_replacements_over_a_megabyte_answer_like_the_compatibility_engine() {
        // Runs of `a` closed by `b` or `c`, after a run of a hundred thousand
        // `a` that no `b` closes within a thousand iterations: the first
        // search's backtracking attempt is abandoned there, and that search
        // and every later one are walked from the starts the set machine marks.
        let mut input = "a".repeat(100_000);
        let mut index = 0_usize;
        while input.len() < 1 << 20 {
            input.push_str(&"a".repeat(1 + (index * 7 + index / 5) % 13));
            input.push(if index.is_multiple_of(3) { 'c' } else { 'b' });
            index += 1;
        }
        for (source, replacement) in [
            ("(a|aa){1,1000}b", "[$1]"),
            ("(a|aa){2,5}b", "<$1>"),
            ("(aa|a){1,3}?(b|c)", "$2$1"),
            ("((a|aa){1,4})(b|c)", "$3$1$2"),
            ("(a|ab|b){3,6}c", "#$1"),
        ] {
            let expected = crate::xsd_regex::compile(source, "")
                .unwrap()
                .replace_all(&input, replacement)
                .unwrap();
            let actual = pattern(source, "")
                .replace_all(&input, replacement, Limits::new())
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            assert_eq!(actual, expected, "{source}");
        }
    }

    #[test]
    fn replacement_continues_on_the_thread_machine_and_resets_every_capture() {
        // The first search's backtracking attempt explores exponentially many
        // splits of the leading run and is abandoned; that search and every
        // later one run on the thread machine, each with fresh captures.
        let forty = "a".repeat(40);
        let program = pattern("(a|aa)*(b)|(c)", "");
        let input = format!("{forty}-ab-aab-b-c");
        assert_eq!(
            program
                .replace_all(&input, "[$1$2$3]", Limits::new())
                .unwrap(),
            format!("{forty}-[ab]-[ab]-[b]-[c]")
        );
        // A large group repetition over a megabyte, replaced whole.
        let pairs = "ab".repeat(1 << 19);
        assert_eq!(
            pattern("^(a|b)+$", "")
                .replace_all(&pairs, "$1", Limits::new())
                .unwrap(),
            "b"
        );
        // FORX0003 is still decided first, by the same machines.
        assert_eq!(
            pattern("(a|aa)*", "")
                .replace_all(&forty, "x", Limits::new())
                .unwrap_err(),
            Error::EmptyMatch
        );
    }

    #[test]
    fn repeated_searches_share_work_and_do_not_inherit_stale_alternatives() {
        let program = pattern("a|b", "");
        assert_eq!(
            program
                .replace_all("aaaa", "#", Limits::new().with(Resource::MatchStates, 1))
                .unwrap(),
            "####"
        );
        let one = pattern("a", "q");
        assert!(
            one.replace_all("a", "x", Limits::new().with(Resource::MatchSteps, 20))
                .is_ok()
        );
        assert!(matches!(
            one.replace_all(
                "aaaaaaaa",
                "x",
                Limits::new().with(Resource::MatchSteps, 20)
            ),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSteps,
                ..
            }))
        ));
    }
}

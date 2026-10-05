// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered native matching, with one request's work fuel and live storage.
//!
//! Alternatives retain complete captures and continuations. Storage admission
//! accounts for those cells across all pending states, rather than admitting
//! only the small state header. Nullable repetitions stop after an empty
//! iteration once the minimum is satisfied; required empty iterations still
//! spend fuel. No operational refusal is converted to a failed branch.

use std::ops::Range;

use purrdf_lex::walk::WorkList;

use super::compile::{CompiledPattern, Count, Node, Set};
use super::{Budget, Error, Limits, Profile, Resource, unicode_tables};

/// UTF-8 byte spans captured by one ordered successful match.
#[derive(Debug, PartialEq, Eq)]
pub struct Captures {
    spans: Vec<Option<Range<usize>>>,
}

impl Captures {
    /// The capture's UTF-8 byte span; zero is the entire match.
    ///
    /// A group that did not participate and an out-of-range group return None.
    #[must_use]
    pub fn get(&self, number: usize) -> Option<Range<usize>> {
        self.spans.get(number).cloned().flatten()
    }

    /// Captures including the entire-pattern capture zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Whether no capture slots exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

impl CompiledPattern {
    /// Whether the input contains a match under this program's dated law.
    ///
    /// # Errors
    ///
    /// Current artifact admission, matcher work and live-state/storage failures
    /// are typed operational errors, never negative matches.
    pub fn is_match(&self, input: &str, limits: Limits) -> Result<bool, Error> {
        Ok(self.find(input, limits)?.is_some())
    }

    /// The first ordered match and its capturing-group spans.
    ///
    /// Search visits UTF-8 boundaries in order. Alternatives visit source order;
    /// greedy/reluctant repetition chooses the corresponding continuation order.
    ///
    /// # Errors
    ///
    /// Admission or execution may refuse its finite resource bound. No partial
    /// match is returned after an operational failure.
    pub fn find(&self, input: &str, limits: Limits) -> Result<Option<Captures>, Error> {
        self.admit(limits)?;
        Vm::new(self, input, limits).find_from(0)
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Node(usize),
    CaptureEnd {
        number: usize,
        start: usize,
    },
    Repeat {
        node: usize,
        count: u64,
        stalled: bool,
    },
    RepeatEnd {
        node: usize,
        count: u64,
        start: usize,
    },
}

struct State {
    start: usize,
    position: usize,
    actions: WorkList<Action, 8>,
    captures: Vec<Option<Range<usize>>>,
}

impl State {
    fn slots(&self) -> u128 {
        2 + (self.captures.len() as u128) * 2 + (self.actions.len() as u128) * 3
    }
}

#[derive(Clone, Copy)]
enum SetAction {
    Visit(usize),
    Not,
    Or,
    Difference,
}

pub(super) struct Vm<'a> {
    program: &'a CompiledPattern,
    input: &'a str,
    pub(super) budget: Budget,
    live_slots: u128,
    pending: WorkList<State, 4>,
}

impl<'a> Vm<'a> {
    pub(super) fn new(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        Self {
            program,
            input,
            budget: Budget::new(limits),
            live_slots: 0,
            pending: WorkList::new(),
        }
    }

    fn push<T, const N: usize>(
        &mut self,
        stack: &mut WorkList<T, N>,
        value: T,
        slots: u64,
    ) -> Result<(), Error> {
        let required = self.live_slots + u128::from(slots);
        self.budget.limits().admit(Resource::MatchSlots, required)?;
        stack.try_push(value).map_err(|_| Error::Allocation {
            resource: Resource::MatchSlots,
            units: stack.len() as u64 + 1,
        })?;
        self.live_slots = required;
        Ok(())
    }

    fn action(&mut self, state: &mut State, action: Action) -> Result<(), Error> {
        self.push(&mut state.actions, action, 3)
    }

    fn initial(&mut self, start: usize) -> Result<State, Error> {
        let count = self.program.captures + 1;
        let slots = 2 + (count as u128) * 2;
        self.budget
            .limits()
            .admit(Resource::MatchSlots, self.live_slots + slots)?;
        self.budget
            .charge_wide(Resource::MatchSteps, count as u128)?;
        let mut captures = Vec::new();
        captures
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: count as u64,
            })?;
        captures.resize_with(count, || None);
        self.live_slots += slots;
        let mut state = State {
            start,
            position: start,
            actions: WorkList::new(),
            captures,
        };
        self.action(&mut state, Action::Node(self.program.root))?;
        Ok(state)
    }

    fn fork(&mut self, state: &State) -> Result<State, Error> {
        self.budget
            .limits()
            .admit(Resource::MatchStates, self.pending.len() as u128 + 1)?;
        let slots = state.slots();
        self.budget
            .limits()
            .admit(Resource::MatchSlots, self.live_slots + slots)?;
        self.budget.charge_wide(
            Resource::MatchSteps,
            state.captures.len() as u128 + state.actions.len() as u128,
        )?;
        let mut captures = Vec::new();
        captures
            .try_reserve_exact(state.captures.len())
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: state.captures.len() as u64,
            })?;
        captures.extend(state.captures.iter().cloned());
        let mut actions = WorkList::new();
        for &action in state.actions.iter() {
            actions.try_push(action).map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: state.actions.len() as u64,
            })?;
        }
        self.live_slots += slots;
        Ok(State {
            start: state.start,
            position: state.position,
            actions,
            captures,
        })
    }

    fn enqueue(&mut self, state: State) -> Result<(), Error> {
        self.budget
            .limits()
            .admit(Resource::MatchStates, self.pending.len() as u128 + 1)?;
        self.pending.try_push(state).map_err(|_| Error::Allocation {
            resource: Resource::MatchStates,
            units: self.pending.len() as u64 + 1,
        })
    }

    pub(super) fn find_from(&mut self, mut start: usize) -> Result<Option<Captures>, Error> {
        loop {
            let mut state = self.initial(start)?;
            loop {
                self.budget.charge(Resource::MatchSteps, 1)?;
                if let Some(action) = state.actions.pop() {
                    self.live_slots -= 3;
                    if self.execute(&mut state, action)? {
                        continue;
                    }
                } else {
                    state.captures[0] = Some(state.start..state.position);
                    // A later replacement search cannot inherit continuations
                    // from this successful match. Returned captures are expanded
                    // and dropped before that caller starts its next search.
                    self.live_slots -= state.slots();
                    while let Some(stale) = self.pending.pop() {
                        self.live_slots -= stale.slots();
                    }
                    debug_assert_eq!(self.live_slots, 0);
                    return Ok(Some(Captures {
                        spans: state.captures,
                    }));
                }
                self.live_slots -= state.slots();
                drop(state);
                if let Some(next) = self.pending.pop() {
                    state = next;
                } else {
                    break;
                }
            }
            if start == self.input.len() {
                return Ok(None);
            }
            self.budget.charge(Resource::MatchSteps, 1)?;
            start += self.input[start..]
                .chars()
                .next()
                .expect("start precedes the end")
                .len_utf8();
        }
    }

    fn execute(&mut self, state: &mut State, action: Action) -> Result<bool, Error> {
        match action {
            Action::Node(node) => self.node(state, node),
            Action::CaptureEnd { number, start } => {
                state.captures[number] = Some(start..state.position);
                Ok(true)
            }
            Action::Repeat {
                node,
                count,
                stalled,
            } => self.repeat(state, node, count, stalled),
            Action::RepeatEnd { node, count, start } => {
                // This spend precedes every count increment, even an empty one.
                self.budget.charge(Resource::MatchSteps, 1)?;
                let count = count
                    .checked_add(1)
                    .expect("finite fuel refuses before a repetition count can overflow");
                self.action(
                    state,
                    Action::Repeat {
                        node,
                        count,
                        stalled: state.position == start,
                    },
                )?;
                Ok(true)
            }
        }
    }

    fn node(&mut self, state: &mut State, node: usize) -> Result<bool, Error> {
        match self.program.nodes[node] {
            Node::Empty => Ok(true),
            Node::Start => Ok(state.position == 0
                || (self.program.modes.multiline
                    && state.position < self.input.len()
                    && self.input[..state.position].ends_with('\n'))),
            Node::End => Ok(if self.program.modes.multiline {
                self.input[state.position..].starts_with('\n')
                    || (state.position == self.input.len() && !self.input.ends_with('\n'))
            } else {
                state.position == self.input.len()
            }),
            Node::Character(set) => {
                let Some(ch) = self.input[state.position..].chars().next() else {
                    return Ok(false);
                };
                if !self.set_matches(set, ch)? {
                    return Ok(false);
                }
                state.position += ch.len_utf8();
                Ok(true)
            }
            Node::Backreference(number) => self.backreference(state, number),
            Node::Sequence(left, right) => {
                self.action(state, Action::Node(right))?;
                self.action(state, Action::Node(left))?;
                Ok(true)
            }
            Node::Choice(left, right) => {
                let mut alternative = self.fork(state)?;
                self.action(&mut alternative, Action::Node(right))?;
                self.enqueue(alternative)?;
                self.action(state, Action::Node(left))?;
                Ok(true)
            }
            Node::Capture { number, body } => {
                self.action(
                    state,
                    Action::CaptureEnd {
                        number,
                        start: state.position,
                    },
                )?;
                self.action(state, Action::Node(body))?;
                Ok(true)
            }
            Node::Repeat { .. } => {
                self.action(
                    state,
                    Action::Repeat {
                        node,
                        count: 0,
                        stalled: false,
                    },
                )?;
                Ok(true)
            }
        }
    }

    fn repeat(
        &mut self,
        state: &mut State,
        node: usize,
        count: u64,
        stalled: bool,
    ) -> Result<bool, Error> {
        let Node::Repeat {
            body,
            min,
            max,
            greedy,
        } = self.program.nodes[node]
        else {
            unreachable!("a repetition continuation names its immutable repeat node");
        };
        let can_stop = matches!(min, Count::Finite(min) if count >= min);
        let can_repeat =
            !(matches!(max, Some(Count::Finite(max)) if count >= max) || stalled && can_stop);
        if !can_repeat {
            return Ok(can_stop);
        }
        if can_stop {
            let mut alternative = self.fork(state)?;
            if greedy {
                self.enqueue(alternative)?;
            } else {
                self.iteration(&mut alternative, node, body, count)?;
                self.enqueue(alternative)?;
                return Ok(true);
            }
        }
        self.iteration(state, node, body, count)?;
        Ok(true)
    }

    fn iteration(
        &mut self,
        state: &mut State,
        node: usize,
        body: usize,
        count: u64,
    ) -> Result<(), Error> {
        self.action(
            state,
            Action::RepeatEnd {
                node,
                count,
                start: state.position,
            },
        )?;
        self.action(state, Action::Node(body))
    }

    fn backreference(&mut self, state: &mut State, number: usize) -> Result<bool, Error> {
        let Some(span) = state.captures[number].clone() else {
            return Ok(true);
        };
        for expected in self.input[span].chars() {
            self.budget.charge(Resource::MatchSteps, 1)?;
            let Some(actual) = self.input[state.position..].chars().next() else {
                return Ok(false);
            };
            if actual != expected {
                if !self.program.modes.insensitive {
                    return Ok(false);
                }
                let variants = case_variants(expected);
                self.budget
                    .charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                if !variants.contains(&(actual as u32)) {
                    return Ok(false);
                }
            }
            state.position += actual.len_utf8();
        }
        Ok(true)
    }

    fn set_matches(&mut self, root: usize, ch: char) -> Result<bool, Error> {
        let mut work = WorkList::<SetAction, 16>::new();
        let mut values = WorkList::<bool, 16>::new();
        self.push(&mut work, SetAction::Visit(root), 1)?;
        while let Some(action) = work.pop() {
            self.live_slots -= 1;
            self.budget.charge(Resource::MatchSteps, 1)?;
            match action {
                SetAction::Visit(set) => match self.program.sets[set] {
                    Set::Complement(child) => {
                        self.push(&mut work, SetAction::Not, 1)?;
                        self.push(&mut work, SetAction::Visit(child), 1)?;
                    }
                    Set::Union(left, right) | Set::Difference(left, right) => {
                        let join = if matches!(self.program.sets[set], Set::Union(..)) {
                            SetAction::Or
                        } else {
                            SetAction::Difference
                        };
                        self.push(&mut work, join, 1)?;
                        self.push(&mut work, SetAction::Visit(right), 1)?;
                        self.push(&mut work, SetAction::Visit(left), 1)?;
                    }
                    set => {
                        let value = self.atomic_set(set, ch)?;
                        self.push(&mut values, value, 1)?;
                    }
                },
                SetAction::Not => {
                    let value = values.pop().expect("a complement operand completed");
                    self.live_slots -= 1;
                    self.push(&mut values, !value, 1)?;
                }
                SetAction::Or | SetAction::Difference => {
                    let right = values.pop().expect("right set operand completed");
                    let left = values.pop().expect("left set operand completed");
                    self.live_slots -= 2;
                    let value = if matches!(action, SetAction::Or) {
                        left || right
                    } else {
                        left && !right
                    };
                    self.push(&mut values, value, 1)?;
                }
            }
        }
        let value = values.pop().expect("one complete character-set result");
        self.live_slots -= 1;
        Ok(value)
    }

    fn atomic_set(&mut self, set: Set, ch: char) -> Result<bool, Error> {
        Ok(match set {
            Set::Range { lo, hi, folded } => {
                if (lo..=hi).contains(&ch) {
                    true
                } else if folded {
                    let variants = case_variants(ch);
                    self.budget
                        .charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                    variants
                        .iter()
                        .any(|&variant| (lo as u32..=hi as u32).contains(&variant))
                } else {
                    false
                }
            }
            Set::Table(ranges) => purrdf_iri::terminals::in_ranges(ch as u32, ranges),
            Set::Space => purrdf_iri::terminals::is_ws_char(ch),
            Set::Word => {
                self.budget.charge(Resource::MatchSteps, 3)?;
                !["P", "Z", "C"].iter().any(|name| {
                    let index = unicode_tables::CATEGORIES
                        .binary_search_by_key(name, |&(name, _)| name)
                        .expect("generated table contains every major general category");
                    purrdf_iri::terminals::in_ranges(ch as u32, unicode_tables::CATEGORIES[index].1)
                })
            }
            Set::Dot => {
                self.program.modes.dot_all
                    || (ch != '\n' && (self.program.profile == Profile::Xpath20 || ch != '\r'))
            }
            Set::Complement(_) | Set::Union(..) | Set::Difference(..) => {
                unreachable!("compound character sets are evaluated on the explicit work list");
            }
        })
    }
}

fn case_variants(ch: char) -> &'static [u32] {
    unicode_tables::CASE_VARIANTS
        .binary_search_by_key(&(ch as u32), |&(point, _)| point)
        .map_or(&[], |index| unicode_tables::CASE_VARIANTS[index].1)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::xsd_regex::xpath::{Refusal, compile};

    fn pattern(profile: Profile, source: &str, flags: &str) -> CompiledPattern {
        compile(profile, source, flags, Limits::new()).unwrap()
    }

    fn matched(source: &str, flags: &str, input: &str) -> Captures {
        pattern(Profile::Xpath31, source, flags)
            .find(input, Limits::new())
            .unwrap()
            .unwrap()
    }

    /// Cartesian words, including empty, specified without the pattern parser
    /// or VM. Every input below has at most five scalar values.
    fn words(alphabet: &[char], maximum: usize) -> Vec<String> {
        let mut all = vec![String::new()];
        let mut level = vec![String::new()];
        for _ in 0..maximum {
            let mut next = Vec::new();
            for prefix in level {
                for &ch in alphabet {
                    let mut word = prefix.clone();
                    word.push(ch);
                    next.push(word);
                }
            }
            all.extend(next.iter().cloned());
            level = next;
        }
        all
    }

    #[test]
    fn independent_finite_languages_cover_matching_and_first_search_position() {
        // Expected words are specified directly, including equality constraints
        // from backreferences. The bounded star language is complete for this
        // input universe; no input contains more than five scalar values.
        let fixtures = [
            ("a", vec!["a".to_owned()]),
            ("(a|b)", vec!["a".to_owned(), "b".to_owned()]),
            ("(ab|a)", vec!["ab".to_owned(), "a".to_owned()]),
            ("(a|b){0,3}", words(&['a', 'b'], 3)),
            ("a{1,3}", ["a", "aa", "aaa"].map(str::to_owned).to_vec()),
            ("(ab){1,2}", ["ab", "abab"].map(str::to_owned).to_vec()),
            ("(|a)b?", ["", "b", "a", "ab"].map(str::to_owned).to_vec()),
            ("(a|b)*", words(&['a', 'b'], 5)),
            ("[a-b-[b]]{1,2}", ["a", "aa"].map(str::to_owned).to_vec()),
            (r"(a|b)\1", ["aa", "bb"].map(str::to_owned).to_vec()),
            (
                r"((a|b)\2){2}",
                ["aaaa", "aabb", "bbaa", "bbbb"].map(str::to_owned).to_vec(),
            ),
            (r"(a?)\1", ["", "aa"].map(str::to_owned).to_vec()),
            (r"((a|ab))\2", ["aa", "abab"].map(str::to_owned).to_vec()),
            (r"(é|𐀀)\1", ["éé", "𐀀𐀀"].map(str::to_owned).to_vec()),
        ];
        let inputs = words(&['a', 'b', 'é', '𐀀'], 5);
        assert_eq!(inputs.len(), 1365);
        let mut comparisons = 0;
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            for (source, language) in &fixtures {
                // No added group: capture numbering belongs to the fixture.
                let whole = pattern(profile, &format!("^{source}$"), "");
                let search = pattern(profile, source, "");
                for input in &inputs {
                    assert_eq!(
                        whole.is_match(input, Limits::new()).unwrap(),
                        language.contains(input),
                        "whole {profile:?} {source:?} {input:?}"
                    );
                    let first = language.iter().filter_map(|word| input.find(word)).min();
                    let actual = search.find(input, Limits::new()).unwrap();
                    assert_eq!(
                        actual
                            .as_ref()
                            .map(|captures| captures.get(0).unwrap().start),
                        first,
                        "search {profile:?} {source:?} {input:?}"
                    );
                    if let Some(captures) = actual {
                        let span = captures.get(0).unwrap();
                        let text = &input[span];
                        assert!(language.iter().any(|word| word == text));
                    }
                    comparisons += 1;
                }
            }
        }
        assert_eq!(comparisons, 38_220);
    }

    #[test]
    fn dated_dot_anchor_and_quoted_modes_have_exact_neighbors() {
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            let dot = pattern(profile, ".", "");
            assert!(!dot.is_match("\n", Limits::new()).unwrap());
            assert_eq!(
                dot.is_match("\r", Limits::new()).unwrap(),
                profile == Profile::Xpath20
            );
            assert!(
                pattern(profile, ".", "s")
                    .is_match("\n", Limits::new())
                    .unwrap()
            );
            assert!(
                pattern(profile, "^a$", "m")
                    .is_match("a\n", Limits::new())
                    .unwrap()
            );
            assert!(
                !pattern(profile, "^a$", "")
                    .is_match("a\n", Limits::new())
                    .unwrap()
            );
            let start = pattern(profile, "^", "m");
            assert!(
                Vm::new(&start, "a\n", Limits::new())
                    .find_from(2)
                    .unwrap()
                    .is_none()
            );
            let end = pattern(profile, "$", "m");
            assert!(
                Vm::new(&end, "a\n", Limits::new())
                    .find_from(2)
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(matched("^b$", "m", "a\nb\n").get(0), Some(2..3));
        let source = r" a.*[\";
        let quoted = pattern(Profile::Xpath31, source, "qsimx");
        assert_eq!(
            quoted.find(source, Limits::new()).unwrap().unwrap().get(0),
            Some(0..source.len())
        );
        assert!(!quoted.is_match("aZ", Limits::new()).unwrap());
    }

    #[test]
    fn full_case_variants_are_direct_and_do_not_fold_properties() {
        for (source, input, expected) in [
            ("θ", "ϑ", true),
            ("θ", "ϴ", true),
            ("ϑ", "ϴ", false),
            ("ϴ", "ϑ", false),
            ("ß", "ẞ", true),
            ("ß", "SS", false),
            ("K", "\u{212a}", true),
            ("[A-Z]", "a", true),
            ("[^A]", "a", false),
            ("[A-Z-[K]]", "\u{212a}", false),
            (r"\p{Lu}", "a", false),
            (r"\p{Lu}", "A", true),
            (r"\P{Lu}", "a", true),
        ] {
            assert_eq!(
                pattern(Profile::Xpath31, source, "i")
                    .is_match(input, Limits::new())
                    .unwrap(),
                expected,
                "{source:?} {input:?}"
            );
        }
        assert!(
            pattern(Profile::Xpath31, r"([A-Z])\1", "i")
                .is_match("Aa", Limits::new())
                .unwrap()
        );
        assert!(
            !pattern(Profile::Xpath31, r"(ϑ)\1", "i")
                .is_match("ϑϴ", Limits::new())
                .unwrap()
        );
    }

    #[test]
    fn ordered_alternatives_repetition_and_utf8_captures_are_exact() {
        let first = matched("(a|aa)(a?)", "", "aa");
        assert_eq!(first.get(0), Some(0..2));
        assert_eq!(first.get(1), Some(0..1));
        assert_eq!(first.get(2), Some(1..2));
        let greedy = matched("^(a+)(a*)$", "", "aaa");
        assert_eq!(greedy.get(1), Some(0..3));
        assert_eq!(greedy.get(2), Some(3..3));
        let reluctant = matched("^(a+?)(a*)$", "", "aaa");
        assert_eq!(reluctant.get(1), Some(0..1));
        assert_eq!(reluctant.get(2), Some(1..3));
        assert_eq!(matched("^(a|b)+$", "", "ab").get(1), Some(1..2));
        let utf8 = matched(r"(é)\1", "", "éé");
        assert_eq!(utf8.get(0), Some(0..4));
        assert_eq!(utf8.get(1), Some(0..2));
        assert_eq!(utf8.get(2), None);
        assert_eq!(utf8.len(), 2);
        assert!(!utf8.is_empty());
        assert_eq!(matched(r"^(a)?b\1$", "", "b").get(1), None);
        assert!(
            pattern(Profile::Xpath31, r"^(a)\12$", "")
                .is_match("aa2", Limits::new())
                .unwrap()
        );
        assert!(
            pattern(
                Profile::Xpath31,
                r"^(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)\12$",
                ""
            )
            .is_match("abcdefghijkll", Limits::new())
            .unwrap()
        );
        assert!(
            pattern(Profile::Xpath31, r"(a)(?:b)\1", "")
                .is_match("aba", Limits::new())
                .unwrap()
        );
    }

    #[test]
    fn nullable_and_arbitrary_size_repetitions_spend_finite_work() {
        assert_eq!(matched("^(a?){3}$", "", "").get(1), Some(0..0));
        assert_eq!(matched("(a?)*", "", "").get(1), Some(0..0));
        let nonnullable = pattern(Profile::Xpath31, "a{18446744073709551616}", "");
        assert!(!nonnullable.is_match("", Limits::new()).unwrap());
        let nullable = pattern(Profile::Xpath31, "(a?){18446744073709551616}", "");
        assert!(matches!(
            nullable.is_match("", Limits::new().with(Resource::MatchSteps, 200)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSteps,
                ..
            }))
        ));
    }

    #[test]
    fn work_state_and_all_live_storage_refusals_never_become_false() {
        let literal = pattern(Profile::Xpath31, "a", "");
        for input in ["a", "z"] {
            assert!(matches!(
                literal.is_match(input, Limits::new().with(Resource::MatchSteps, 0)),
                Err(Error::Resource(Refusal {
                    resource: Resource::MatchSteps,
                    ..
                }))
            ));
        }
        assert!(
            literal
                .is_match("a", Limits::new().with(Resource::MatchStates, 0))
                .unwrap()
        );
        let choice = pattern(Profile::Xpath31, "z|a", "");
        assert!(matches!(
            choice.is_match("a", Limits::new().with(Resource::MatchStates, 0)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchStates,
                ..
            }))
        ));
        assert!(
            choice
                .is_match("a", Limits::new().with(Resource::MatchStates, 1))
                .unwrap()
        );
        assert!(
            literal
                .is_match("a", Limits::new().with(Resource::MatchSlots, 7))
                .unwrap()
        );
        assert!(matches!(
            literal.is_match("a", Limits::new().with(Resource::MatchSlots, 6)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSlots,
                ..
            }))
        ));
        assert!(
            choice
                .is_match("a", Limits::new().with(Resource::MatchSlots, 14))
                .unwrap()
        );
        assert!(matches!(
            choice.is_match("a", Limits::new().with(Resource::MatchSlots, 13)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSlots,
                ..
            }))
        ));
    }

    #[test]
    fn search_has_one_budget_and_each_execution_has_fresh_fuel() {
        let program = pattern(Profile::Xpath31, "a", "");
        let input = "bbbbbbba";
        let mut vm = Vm::new(&program, input, Limits::new());
        assert_eq!(vm.find_from(0).unwrap().unwrap().get(0), Some(7..8));
        let steps = vm.budget.used(Resource::MatchSteps);
        let exact = Limits::new().with(Resource::MatchSteps, steps);
        assert!(program.is_match(input, exact).unwrap());
        assert!(program.is_match(input, exact).unwrap());
        assert!(matches!(
            program.is_match(input, exact.with(Resource::MatchSteps, steps - 1)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSteps,
                ..
            }))
        ));
        assert!(matches!(
            program.is_match(input, exact.with(Resource::ProgramNodes, 0)),
            Err(Error::Resource(Refusal {
                resource: Resource::ProgramNodes,
                ..
            }))
        ));
    }

    #[test]
    fn deep_capturing_groups_match_and_drop_on_a_small_native_stack() {
        purrdf_stack::on_stack(256 * 1024, || {
            let source = format!("{}é{}", "(".repeat(6000), ")".repeat(6000));
            let program = pattern(Profile::Xpath31, &source, "");
            let captures = program.find("é", Limits::new()).unwrap().unwrap();
            assert_eq!(captures.len(), 6001);
            assert_eq!(captures.get(6000), Some(0..2));
            drop(captures);
            drop(program);
        })
        .unwrap();
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit-stack ECMA backtracking for constructs that are not regular.

use super::{
    Ast, Class, ClassItem, PatternError, Property, canonicalize, contains_property, emit,
    unicode_ranges,
};

/// Limits shared by one validation, including nested assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchLimits {
    /// Maximum number of VM instructions.
    pub steps: u64,
    /// Maximum number of active alternative states.
    pub states: usize,
}
/// A slot is one pending instruction or one capture cell. This bounds the
/// memory copied by backtracking even when one state contains a long program.
const MAX_VM_SLOTS: usize = 100_000;
impl Default for MatchLimits {
    fn default() -> Self {
        Self {
            steps: 10_000_000,
            states: 65_536,
        }
    }
}

#[derive(Clone)]
enum Task<'a> {
    Match(&'a Ast),
    Save(u32, bool),
    RestoreFlags(u8),
    Repeat {
        body: &'a Ast,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
    RepeatAfter {
        body: &'a Ast,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        position: usize,
    },
}
#[derive(Clone)]
struct State<'a> {
    pos: usize,
    tasks: Vec<Task<'a>>,
    captures: Vec<Option<(usize, usize)>>,
    flags: u8,
    backward: bool,
}
struct Frame<'a> {
    states: Vec<State<'a>>,
    /// Assertion continuation in the parent frame.
    continuation: Option<(State<'a>, bool)>,
    next_search: Option<usize>,
}

fn state_slots(state: &State<'_>) -> usize {
    state.tasks.len().saturating_add(state.captures.len())
}

fn resource() -> PatternError {
    PatternError::Resource {
        offset: 0,
        message: "ECMA matcher budget exhausted".to_owned(),
    }
}

fn ensure_slots(used: usize, extra: usize) -> Result<(), PatternError> {
    if used.saturating_add(extra) > MAX_VM_SLOTS {
        Err(resource())
    } else {
        Ok(())
    }
}

fn next_boundary(input: &str, pos: usize) -> Option<(char, usize)> {
    input
        .get(pos..)?
        .chars()
        .next()
        .map(|ch| (ch, pos + ch.len_utf8()))
}
fn previous_boundary(input: &str, pos: usize) -> Option<(char, usize)> {
    input
        .get(..pos)?
        .char_indices()
        .next_back()
        .map(|(index, ch)| (ch, index))
}
fn char_at(input: &str, state: &mut State<'_>) -> Option<char> {
    if state.backward {
        let (ch, pos) = previous_boundary(input, state.pos)?;
        state.pos = pos;
        Some(ch)
    } else {
        let (ch, pos) = next_boundary(input, state.pos)?;
        state.pos = pos;
        Some(ch)
    }
}
fn case_equal(left: char, right: char, insensitive: bool) -> bool {
    left == right
        || (insensitive && canonicalize(u32::from(left)) == canonicalize(u32::from(right)))
}
fn word(ch: Option<char>, insensitive: bool) -> bool {
    ch.is_some_and(|ch| {
        let code = if insensitive {
            canonicalize(u32::from(ch))
        } else {
            u32::from(ch)
        };
        code == u32::from('_')
            || (u32::from('0')..=u32::from('9')).contains(&code)
            || (u32::from('A')..=u32::from('Z')).contains(&code)
            || (u32::from('a')..=u32::from('z')).contains(&code)
    })
}

fn in_ranges(ranges: &[(u32, u32)], code: u32) -> bool {
    let index = ranges.partition_point(|(_, high)| *high < code);
    ranges.get(index).is_some_and(|(low, _)| *low <= code)
}

fn folded_any(code: u32, insensitive: bool, test: impl Fn(u32) -> bool) -> bool {
    if !insensitive {
        return test(code);
    }
    let folded = canonicalize(code);
    if test(folded) {
        return true;
    }
    let reverse = unicode_ranges::CASE_FOLD_REVERSE;
    let start = reverse.partition_point(|(target, _)| *target < folded);
    reverse[start..]
        .iter()
        .take_while(|(target, _)| *target == folded)
        .any(|(_, source)| test(*source))
}

fn class_match(class: &Class, code: u32, insensitive: bool) -> bool {
    let member = class.items.iter().any(|item| match item {
        ClassItem::Range(low, high) => folded_any(code, insensitive, |candidate| {
            (*low..=*high).contains(&candidate)
        }),
        ClassItem::Digit(negated) => {
            folded_any(code, insensitive, |candidate| {
                in_ranges(emit::DIGIT, candidate)
            }) != *negated
        }
        ClassItem::Word(negated) => {
            folded_any(code, insensitive, |candidate| {
                in_ranges(emit::WORD, candidate)
            }) != *negated
        }
        ClassItem::Space(negated) => {
            folded_any(code, insensitive, |candidate| {
                in_ranges(emit::SPACE, candidate)
            }) != *negated
        }
        ClassItem::Property(property, negated) => folded_any(code, insensitive, |candidate| {
            let hit = match property {
                Property::Any => true,
                Property::Ascii => candidate <= 0x7F,
                Property::Assigned => contains_property("Assigned", candidate),
                Property::Named(name) => contains_property(name, candidate),
            };
            hit != *negated
        }),
    });
    member != class.negated
}

fn reset_captures(ast: &Ast, captures: &mut [Option<(usize, usize)>]) {
    match ast {
        Ast::Group { body, capture } => {
            if let Some(index) = capture {
                captures[*index as usize] = None;
            }
            reset_captures(body, captures);
        }
        Ast::Look { body, .. } | Ast::Modifiers { body, .. } | Ast::Repeat { body, .. } => {
            reset_captures(body, captures);
        }
        Ast::Concat(items) | Ast::Alternation(items) => {
            for item in items {
                reset_captures(item, captures);
            }
        }
        _ => {}
    }
}

pub(super) fn is_match(
    ast: &Ast,
    input: &str,
    limits: &mut MatchLimits,
    capture_count: usize,
) -> Result<bool, PatternError> {
    ensure_slots(0, capture_count.saturating_add(2))?;
    let mut live_slots = capture_count + 2;
    let initial = State {
        pos: 0,
        tasks: vec![Task::Match(ast)],
        captures: vec![None; capture_count + 1],
        flags: 0,
        backward: false,
    };
    let mut frames = vec![Frame {
        states: vec![initial],
        continuation: None,
        next_search: next_boundary(input, 0).map(|(_, pos)| pos),
    }];
    loop {
        let active_states = frames.iter().map(|item| item.states.len()).sum::<usize>();
        let Some(frame) = frames.last_mut() else {
            return Ok(false);
        };
        let Some(mut state) = frame.states.pop() else {
            if let Some(pos) = frame.next_search {
                ensure_slots(live_slots, capture_count.saturating_add(2))?;
                frame.next_search = next_boundary(input, pos).map(|(_, next)| next);
                frame.states.push(State {
                    pos,
                    tasks: vec![Task::Match(ast)],
                    captures: vec![None; capture_count + 1],
                    flags: 0,
                    backward: false,
                });
                live_slots += capture_count + 2;
                continue;
            }
            let ended = frames.pop().expect("active frame");
            if let Some((parent, positive)) = ended.continuation {
                // No assertion match: only a negative assertion continues.
                if !positive {
                    frames.last_mut().expect("parent frame").states.push(parent);
                } else {
                    live_slots -= state_slots(&parent);
                }
                continue;
            }
            return Ok(false);
        };
        live_slots -= state_slots(&state);
        if limits.steps == 0 || active_states > limits.states {
            return Err(resource());
        }
        limits.steps -= 1;
        let Some(task) = state.tasks.pop() else {
            if frames.len() == 1 {
                return Ok(true);
            }
            let child = frames.pop().expect("child frame");
            for pending in &child.states {
                live_slots -= state_slots(pending);
            }
            let (mut parent, positive) = child.continuation.expect("assertion continuation");
            if positive {
                parent.captures = state.captures;
                frames.last_mut().expect("parent frame").states.push(parent);
            } else {
                live_slots -= state_slots(&parent);
            }
            continue;
        };
        match task {
            Task::Save(index, begin) => {
                let cell = &mut state.captures[index as usize];
                if begin {
                    *cell = Some((state.pos, cell.map_or(state.pos, |(_, end)| end)));
                } else {
                    *cell = Some((cell.map_or(state.pos, |(start, _)| start), state.pos));
                }
            }
            Task::RestoreFlags(flags) => state.flags = flags,
            Task::Repeat {
                body,
                min,
                max,
                greedy,
            } => {
                if max != Some(0) {
                    ensure_slots(
                        live_slots,
                        state_slots(&state).saturating_mul(2).saturating_add(2),
                    )?;
                    let mut take = state.clone();
                    reset_captures(body, &mut take.captures);
                    take.tasks.push(Task::RepeatAfter {
                        body,
                        min: min.saturating_sub(1),
                        max: max.map(|n| n - 1),
                        greedy,
                        position: take.pos,
                    });
                    take.tasks.push(Task::Match(body));
                    if min > 0 {
                        state = take;
                    } else if greedy {
                        live_slots += state_slots(&state);
                        frame.states.push(state);
                        state = take;
                    } else {
                        live_slots += state_slots(&take);
                        frame.states.push(take);
                    }
                } else if min != 0 {
                    continue;
                }
            }
            Task::RepeatAfter {
                body,
                min,
                max,
                greedy,
                position,
            } => {
                if state.pos != position || min != 0 {
                    state.tasks.push(Task::Repeat {
                        body,
                        min,
                        max,
                        greedy,
                    });
                }
            }
            Task::Match(node) => match node {
                Ast::Empty => {}
                Ast::Char(code) => {
                    let Some(expected) = char::from_u32(*code) else {
                        continue;
                    };
                    let Some(found) = char_at(input, &mut state) else {
                        continue;
                    };
                    if !case_equal(found, expected, state.flags & 1 != 0) {
                        continue;
                    }
                }
                Ast::Dot => {
                    let Some(found) = char_at(input, &mut state) else {
                        continue;
                    };
                    if state.flags & 4 == 0
                        && matches!(found, '\n' | '\r' | '\u{2028}' | '\u{2029}')
                    {
                        continue;
                    }
                }
                Ast::Class(class) => {
                    let Some(found) = char_at(input, &mut state) else {
                        continue;
                    };
                    if !class_match(class, u32::from(found), state.flags & 1 != 0) {
                        continue;
                    }
                }
                Ast::Start => {
                    if state.pos != 0
                        && !(state.flags & 2 != 0
                            && previous_boundary(input, state.pos).is_some_and(|(ch, _)| {
                                matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
                            }))
                    {
                        continue;
                    }
                }
                Ast::End => {
                    if state.pos != input.len()
                        && !(state.flags & 2 != 0
                            && next_boundary(input, state.pos).is_some_and(|(ch, _)| {
                                matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
                            }))
                    {
                        continue;
                    }
                }
                Ast::WordBoundary(negated) => {
                    let before = word(
                        previous_boundary(input, state.pos).map(|(ch, _)| ch),
                        state.flags & 1 != 0,
                    );
                    let after = word(
                        next_boundary(input, state.pos).map(|(ch, _)| ch),
                        state.flags & 1 != 0,
                    );
                    if (before != after) == *negated {
                        continue;
                    }
                }
                Ast::Group { body, capture } => {
                    if let Some(index) = capture {
                        state.tasks.push(Task::Save(*index, state.backward));
                    }
                    state.tasks.push(Task::Match(body));
                    if let Some(index) = capture {
                        state.tasks.push(Task::Save(*index, !state.backward));
                    }
                }
                Ast::Concat(items) => {
                    ensure_slots(live_slots, state_slots(&state).saturating_add(items.len()))?;
                    if state.backward {
                        for item in items {
                            state.tasks.push(Task::Match(item));
                        }
                    } else {
                        for item in items.iter().rev() {
                            state.tasks.push(Task::Match(item));
                        }
                    }
                }
                Ast::Alternation(items) => {
                    for item in items.iter().skip(1).rev() {
                        ensure_slots(
                            live_slots,
                            state_slots(&state).saturating_mul(2).saturating_add(1),
                        )?;
                        let mut alternative = state.clone();
                        alternative.tasks.push(Task::Match(item));
                        live_slots += state_slots(&alternative);
                        frame.states.push(alternative);
                    }
                    if let Some(first) = items.first() {
                        state.tasks.push(Task::Match(first));
                    }
                }
                Ast::Repeat {
                    body,
                    min,
                    max,
                    greedy,
                } => {
                    state.tasks.push(Task::Repeat {
                        body,
                        min: *min,
                        max: *max,
                        greedy: *greedy,
                    });
                }
                Ast::Backreference { targets, .. } => {
                    let Some((start, end)) = targets
                        .iter()
                        .find_map(|target| state.captures[*target as usize])
                    else {
                        live_slots += state_slots(&state);
                        frame.states.push(state);
                        continue;
                    };
                    let captured = &input[start..end];
                    let equivalent = |candidate: &str| {
                        captured
                            .chars()
                            .zip(candidate.chars())
                            .all(|(left, right)| case_equal(left, right, state.flags & 1 != 0))
                    };
                    let count = captured.chars().count();
                    if count == 0 {
                        live_slots += state_slots(&state);
                        frame.states.push(state);
                        continue;
                    }
                    if state.backward {
                        let Some((start, _)) = input[..state.pos]
                            .char_indices()
                            .rev()
                            .nth(count.saturating_sub(1))
                        else {
                            continue;
                        };
                        let candidate = &input[start..state.pos];
                        if !equivalent(candidate) {
                            continue;
                        }
                        state.pos = start;
                    } else {
                        let tail = &input[state.pos..];
                        let end = tail
                            .char_indices()
                            .nth(count)
                            .map_or(tail.len(), |(index, _)| index);
                        let candidate = &tail[..end];
                        if candidate.chars().count() != count || !equivalent(candidate) {
                            continue;
                        }
                        state.pos += end;
                    }
                }
                Ast::Modifiers {
                    body,
                    enable,
                    disable,
                    ..
                } => {
                    let prior = state.flags;
                    state.flags = (prior | enable) & !disable;
                    state.tasks.push(Task::RestoreFlags(prior));
                    state.tasks.push(Task::Match(body));
                }
                Ast::Look {
                    body,
                    positive,
                    behind,
                    ..
                } => {
                    let child_slots = state.captures.len().saturating_add(1);
                    ensure_slots(live_slots, state_slots(&state).saturating_add(child_slots))?;
                    let child = State {
                        pos: state.pos,
                        tasks: vec![Task::Match(body)],
                        captures: state.captures.clone(),
                        flags: state.flags,
                        backward: *behind,
                    };
                    live_slots += state_slots(&state) + state_slots(&child);
                    frames.push(Frame {
                        states: vec![child],
                        continuation: Some((state, *positive)),
                        next_search: None,
                    });
                    continue;
                }
            },
        }
        ensure_slots(live_slots, state_slots(&state))?;
        live_slots += state_slots(&state);
        frames.last_mut().expect("frame").states.push(state);
    }
}

#[cfg(test)]
mod tests {
    use super::{Ast, MAX_VM_SLOTS, MatchLimits, PatternError, is_match};

    #[test]
    fn lookaround_accounts_for_child_instruction_at_slot_limit() {
        let assertion = Ast::Look {
            offset: 0,
            behind: false,
            positive: true,
            body: Box::new(Ast::Empty),
        };
        // A retained parent and its child each own capture_count + 1 cells;
        // the child also owns its pending instruction.
        assert_eq!(
            is_match(
                &assertion,
                "",
                &mut MatchLimits::default(),
                MAX_VM_SLOTS / 2 - 2
            ),
            Ok(true)
        );
        assert!(matches!(
            is_match(
                &assertion,
                "",
                &mut MatchLimits::default(),
                MAX_VM_SLOTS / 2 - 1
            ),
            Err(PatternError::Resource { .. })
        ));
    }
}

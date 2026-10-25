// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{MatchError, MatchReport, Mode, Range, Token};

struct Meter {
    work: u64,
    limit: u64,
}
impl Meter {
    fn tick(&mut self, offset: usize) -> Result<(), MatchError> {
        if self.work == self.limit {
            return Err(MatchError::WorkExceeded {
                consumed: self.work,
                limit: self.limit,
                offset,
            });
        }
        self.work += 1;
        Ok(())
    }
    fn report(&self, matched: bool) -> MatchReport {
        MatchReport {
            matched,
            work: self.work,
        }
    }
}

#[derive(Clone, Copy)]
struct Retry {
    token: usize,
    input: usize,
}

pub(super) fn absent(limit: u64) -> Result<MatchReport, MatchError> {
    let mut meter = Meter { work: 0, limit };
    meter.tick(0)?;
    Ok(meter.report(false))
}

pub(super) fn matches(
    tokens: &[Token],
    ranges: &[Range],
    mode: Mode,
    input: &str,
    limit: u64,
) -> Result<MatchReport, MatchError> {
    let mut meter = Meter { work: 0, limit };
    let mut token = 0;
    let mut at = 0;
    let mut star: Option<Retry> = None;
    let mut directories: Option<Retry> = None;
    loop {
        meter.tick(at)?;
        let ch = input[at..].chars().next();
        let accepted = match tokens.get(token) {
            None if ch.is_none() => return Ok(meter.report(true)),
            None => false,
            Some(Token::Tail) => return Ok(meter.report(true)),
            Some(Token::Directories) => {
                token += 1;
                directories = Some(Retry { token, input: at });
                star = None;
                continue;
            }
            Some(Token::Star) => {
                token += 1;
                star = Some(Retry { token, input: at });
                continue;
            }
            Some(Token::Literal(expected)) => ch == Some(*expected),
            Some(Token::Separator) => ch == Some('/'),
            Some(Token::Any) => ch.is_some_and(|ch| mode == Mode::Text || ch != '/'),
            Some(Token::Class {
                start,
                end,
                negated,
            }) => {
                if let Some(ch) = ch.filter(|ch| mode == Mode::Text || *ch != '/') {
                    meter.tick(at)?;
                    purrdf_lex::terminals::in_ranges(u32::from(ch), &ranges[*start..*end])
                        != *negated
                } else {
                    false
                }
            }
        };
        if accepted {
            if matches!(tokens[token], Token::Separator) {
                star = None;
            }
            token += 1;
            at += ch.expect("an accepted scalar token has input").len_utf8();
            continue;
        }
        if let Some(retry) = star.as_mut()
            && let Some(ch) = input[retry.input..].chars().next()
            && (mode == Mode::Text || ch != '/')
        {
            meter.tick(retry.input)?;
            retry.input += ch.len_utf8();
            token = retry.token;
            at = retry.input;
            continue;
        }
        if let Some(retry) = directories.as_mut() {
            let mut next = retry.input;
            let mut boundary = None;
            for ch in input[next..].chars() {
                meter.tick(next)?;
                next += ch.len_utf8();
                if ch == '/' {
                    boundary = Some(next);
                    break;
                }
            }
            if let Some(next) = boundary {
                retry.input = next;
                at = next;
                token = retry.token;
                star = None;
                continue;
            }
        }
        return Ok(meter.report(false));
    }
}

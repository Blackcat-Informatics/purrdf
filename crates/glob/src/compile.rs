// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{
    Account, Admission, CompileError, CompiledGlob, Mode, Range, StorageError, SyntaxReason, Token,
};
use purrdf_lex::allocation::Memory;

#[derive(Default)]
struct Counts {
    tokens: usize,
    ranges: usize,
}

struct Parser<'a> {
    source: &'a str,
    at: usize,
}
impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.source[self.at..].chars().next()
    }
    fn take(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.at += ch.len_utf8();
        Some(ch)
    }
    fn error(&self, offset: usize, reason: SyntaxReason) -> CompileError {
        CompileError::Syntax { offset, reason }
    }
    fn scalar(&mut self) -> Result<char, CompileError> {
        let at = self.at;
        let ch = self
            .take()
            .ok_or_else(|| self.error(at, SyntaxReason::UnclosedClass))?;
        if ch == '\\' {
            self.take()
                .ok_or_else(|| self.error(at, SyntaxReason::TrailingEscape))
        } else {
            Ok(ch)
        }
    }
}

trait Output {
    fn token(&mut self, token: Token) -> Result<(), CompileError>;
    fn range(&mut self, range: Range) -> Result<(), CompileError>;
    fn ranges(&self) -> usize;
}
impl Output for Counts {
    fn token(&mut self, _: Token) -> Result<(), CompileError> {
        self.tokens = self
            .tokens
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        Ok(())
    }
    fn range(&mut self, _: Range) -> Result<(), CompileError> {
        self.ranges = self
            .ranges
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        Ok(())
    }
    fn ranges(&self) -> usize {
        self.ranges
    }
}
struct Destination<'a> {
    tokens: &'a mut Vec<Token>,
    ranges: &'a mut Vec<Range>,
}
impl Output for Destination<'_> {
    fn token(&mut self, token: Token) -> Result<(), CompileError> {
        if self.tokens.len() == self.tokens.capacity() {
            return Err(StorageError::SizeOverflow.into());
        }
        // The first identical parse certified and admitted this exact slot.
        self.tokens.push(token);
        Ok(())
    }
    fn range(&mut self, range: Range) -> Result<(), CompileError> {
        if self.ranges.len() == self.ranges.capacity() {
            return Err(StorageError::SizeOverflow.into());
        }
        self.ranges.push(range);
        Ok(())
    }
    fn ranges(&self) -> usize {
        self.ranges.len()
    }
}

pub(super) fn compile<S: Admission>(
    source: &str,
    mode: Mode,
    storage: S,
) -> Result<CompiledGlob<S>, CompileError> {
    let mut account = Account(storage);
    let mut counts = Counts::default();
    parse(source, mode, &mut counts)?;
    let mut tokens = Vec::new();
    let mut ranges = Vec::new();
    {
        let mut memory = Memory::new(&mut account.0);
        memory.reserve(&mut tokens, counts.tokens)?;
        memory.reserve(&mut ranges, counts.ranges)?;
        parse(
            source,
            mode,
            &mut Destination {
                tokens: &mut tokens,
                ranges: &mut ranges,
            },
        )?;
        for token in &mut tokens {
            if let Token::Class { start, end, .. } = token {
                let class = &mut ranges[*start..*end];
                class.sort_unstable();
                let mut kept = 0;
                for index in 0..class.len() {
                    let (first, last) = class[index];
                    if kept > 0 && first <= class[kept - 1].1.saturating_add(1) {
                        class[kept - 1].1 = class[kept - 1].1.max(last);
                    } else {
                        class[kept] = (first, last);
                        kept += 1;
                    }
                }
                *end = *start + kept;
            }
        }
    }
    Ok(CompiledGlob {
        tokens,
        ranges,
        mode,
        _account: account,
    })
}

fn parse(source: &str, mode: Mode, out: &mut impl Output) -> Result<(), CompileError> {
    let mut parser = Parser { source, at: 0 };
    let mut component_start = true;
    while let Some(ch) = parser.peek() {
        let at = parser.at;
        parser.take();
        match ch {
            '\\' => {
                let literal = parser
                    .take()
                    .ok_or_else(|| parser.error(at, SyntaxReason::TrailingEscape))?;
                if mode == Mode::Path && literal == '/' {
                    out.token(Token::Separator)?;
                    component_start = true;
                    continue;
                }
                out.token(Token::Literal(literal))?;
            }
            '/' if mode == Mode::Path => {
                out.token(Token::Separator)?;
                component_start = true;
                continue;
            }
            '*' => {
                let mut stars = 1usize;
                while parser.peek() == Some('*') {
                    parser.take();
                    stars += 1;
                }
                if mode == Mode::Path
                    && stars == 2
                    && component_start
                    && (matches!(parser.peek(), None | Some('/'))
                        || parser.source[parser.at..].starts_with("\\/"))
                {
                    if parser.peek().is_some() {
                        if parser.take() == Some('\\') {
                            parser.take();
                        }
                        out.token(Token::Directories)?;
                        component_start = true;
                        continue;
                    }
                    out.token(Token::Tail)?;
                } else {
                    out.token(Token::Star)?;
                }
            }
            '?' => out.token(Token::Any)?,
            '[' => {
                let negated = matches!(parser.peek(), Some('!' | '^'));
                if negated {
                    parser.take();
                }
                let start = out.ranges();
                let first_member = parser.at;
                loop {
                    match parser.peek() {
                        None => return Err(parser.error(at, SyntaxReason::UnclosedClass)),
                        Some(']') => {
                            parser.take();
                            break;
                        }
                        Some('[') if parser.source[parser.at..].starts_with("[:") => {
                            return Err(parser.error(parser.at, SyntaxReason::NamedClass));
                        }
                        _ => {}
                    }
                    let first_at = parser.at;
                    let first = parser.scalar()?;
                    let last = if !(first == '-' && first_at == first_member)
                        && parser.peek() == Some('-')
                        && !parser.source[parser.at + 1..].starts_with(']')
                    {
                        parser.take();
                        parser.scalar()?
                    } else {
                        first
                    };
                    if last < first {
                        return Err(parser.error(first_at, SyntaxReason::ReversedRange));
                    }
                    out.range((u32::from(first), u32::from(last)))?;
                }
                let end = out.ranges();
                if start == end {
                    return Err(parser.error(at, SyntaxReason::EmptyClass));
                }
                out.token(Token::Class {
                    start,
                    end,
                    negated,
                })?;
            }
            '{' | '}' => return Err(parser.error(at, SyntaxReason::BraceAlternative)),
            _ => out.token(Token::Literal(ch))?,
        }
        component_start = false;
    }
    Ok(())
}

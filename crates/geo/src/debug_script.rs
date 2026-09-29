// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The script `#[derive(Debug)]` prints for a recursive type, written token by token
//! off a work list, so a value nested as deep as memory allows prints in both the
//! plain (`{:?}`) and the pretty (`{:#?}`) form without a stack frame per level.
//!
//! A type describes one node of its value as a flat run of [`Tok`]s — struct and
//! variant names, field names, leaf values — and names each nested value as a
//! [`Tok::Node`], which [`write`] expands in place when it is popped. The bytes are
//! the derive's exactly: the separators, the brackets and the pretty form's four
//! spaces of indentation per open container are the rules of the standard library's
//! `DebugStruct`, `DebugTuple` and `DebugList` builders and of the `PadAdapter` they
//! nest once per level, spelled here once for [`crate::geom::Geometry`].

use core::fmt::{self, Write as _};

/// One token of a value's script.
#[derive(Clone, Copy)]
pub(crate) enum Tok<'a, N> {
    /// A struct opens: its name.
    Struct(&'static str),
    /// A field of the open struct: its name. Its value follows.
    Field(&'static str),
    /// The open struct closes.
    EndStruct,
    /// A tuple-like variant opens: its name. The empty name is a plain tuple.
    Tuple(&'static str),
    /// The open tuple closes.
    EndTuple,
    /// A list opens.
    List,
    /// The open list closes.
    EndList,
    /// A leaf, written with its own `Debug` in the form being printed.
    Leaf(&'a dyn fmt::Debug),
    /// A nested value, read as its own script when it is reached.
    Node(N),
}

/// Writes into a formatter, indenting every line after the first by four spaces per
/// open pretty-printed container — what nesting the standard library's `PadAdapter`
/// once per level produces.
struct Pad<'f, 'g> {
    f: &'f mut fmt::Formatter<'g>,
    pretty: bool,
    depth: usize,
    line_start: bool,
}

impl fmt::Write for Pad<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for piece in s.split_inclusive('\n') {
            if self.line_start {
                for _ in 0..self.depth {
                    self.f.write_str("    ")?;
                }
            }
            self.line_start = piece.ends_with('\n');
            self.f.write_str(piece)?;
        }
        Ok(())
    }
}

/// A container open in [`write`]: what it is and how many entries it holds so far.
struct Open {
    kind: OpenKind,
    entries: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenKind {
    Struct,
    Tuple,
    List,
}

impl Pad<'_, '_> {
    /// A value starts inside the innermost open container: write what separates it
    /// from the entry before it.
    fn begin(&mut self, open: &mut [Open]) -> fmt::Result {
        let Some(container) = open.last_mut() else {
            return Ok(());
        };
        match container.kind {
            // The field token already wrote the separator and the name.
            OpenKind::Struct => return Ok(()),
            OpenKind::Tuple => {
                if self.pretty {
                    if container.entries == 0 {
                        self.write_str("(\n")?;
                        self.depth += 1;
                    }
                } else {
                    self.write_str(if container.entries == 0 { "(" } else { ", " })?;
                }
            }
            OpenKind::List => {
                if self.pretty {
                    if container.entries == 0 {
                        self.write_str("\n")?;
                        self.depth += 1;
                    }
                } else if container.entries > 0 {
                    self.write_str(", ")?;
                }
            }
        }
        container.entries += 1;
        Ok(())
    }

    /// A value inside an open container has ended.
    fn end(&mut self, open: &[Open]) -> fmt::Result {
        if self.pretty && !open.is_empty() {
            self.write_str(",\n")?;
        }
        Ok(())
    }

    /// A leaf, written as the derive writes it in this form.
    fn leaf(&mut self, value: &dyn fmt::Debug) -> fmt::Result {
        if self.pretty {
            write!(self, "{value:#?}")
        } else {
            write!(self, "{value:?}")
        }
    }
}

/// Write `root`'s script into `f`, in the form `f` asks for.
///
/// `script` appends the tokens of one node in reading order; each [`Tok::Node`] is
/// expanded the moment it is popped, so the walk costs heap and never stack.
pub(crate) fn write<'a, N: Copy>(
    f: &mut fmt::Formatter<'_>,
    root: N,
    mut script: impl FnMut(N, &mut Vec<Tok<'a, N>>),
) -> fmt::Result {
    let pretty = f.alternate();
    let mut out = Pad {
        f,
        pretty,
        depth: 0,
        line_start: false,
    };
    let mut stack: Vec<Tok<'a, N>> = vec![Tok::Node(root)];
    let mut open: Vec<Open> = Vec::new();
    while let Some(tok) = stack.pop() {
        match tok {
            Tok::Node(node) => {
                // The node's script replaces its token, reversed so that its first
                // token is popped next.
                let before = stack.len();
                script(node, &mut stack);
                stack[before..].reverse();
            }
            Tok::Struct(name) | Tok::Tuple(name) => {
                out.begin(&mut open)?;
                out.write_str(name)?;
                open.push(Open {
                    kind: if matches!(tok, Tok::Struct(_)) {
                        OpenKind::Struct
                    } else {
                        OpenKind::Tuple
                    },
                    entries: 0,
                });
            }
            Tok::List => {
                out.begin(&mut open)?;
                out.write_str("[")?;
                open.push(Open {
                    kind: OpenKind::List,
                    entries: 0,
                });
            }
            Tok::Field(name) => {
                let container = open
                    .last_mut()
                    .expect("a field is written inside its struct");
                if pretty {
                    if container.entries == 0 {
                        out.write_str(" {\n")?;
                        out.depth += 1;
                    }
                } else {
                    out.write_str(if container.entries == 0 { " { " } else { ", " })?;
                }
                container.entries += 1;
                out.write_str(name)?;
                out.write_str(": ")?;
            }
            Tok::EndStruct | Tok::EndTuple | Tok::EndList => {
                let container = open.pop().expect("a container closes after it opens");
                match container.kind {
                    OpenKind::Struct if container.entries > 0 => {
                        if pretty {
                            out.depth -= 1;
                            out.write_str("}")?;
                        } else {
                            out.write_str(" }")?;
                        }
                    }
                    OpenKind::Struct => {}
                    OpenKind::Tuple if container.entries > 0 => {
                        if pretty {
                            out.depth -= 1;
                        }
                        out.write_str(")")?;
                    }
                    OpenKind::Tuple => {}
                    OpenKind::List => {
                        if pretty && container.entries > 0 {
                            out.depth -= 1;
                        }
                        out.write_str("]")?;
                    }
                }
                out.end(&open)?;
            }
            Tok::Leaf(value) => {
                out.begin(&mut open)?;
                out.leaf(value)?;
                out.end(&open)?;
            }
        }
    }
    Ok(())
}

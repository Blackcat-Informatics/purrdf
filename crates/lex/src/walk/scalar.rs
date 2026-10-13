// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Standard primitive leaves, written with every option of the caller's formatter
//! through the shared iterative writer.
//!
//! A primitive's `Debug` output holds a line break only as padding, when the fill
//! character is itself a newline: string `Debug` escapes every content newline and
//! ignores width, and integer `Debug` emits digits, a sign and a radix prefix. So
//! every leaf except one written in the pretty form with newline fill is written
//! by the caller's own formatter, which applies all of its options exactly. In
//! that one case the derive's `PadAdapter` indents the padding as well, so the
//! leaf is rendered with the same options into the indenting writer instead.
//! Neither path can fail except through the caller's own writer.

use core::{cell::Cell, fmt};

use super::{Pad, Tok, WorkList, write_debug_with};

/// A standard primitive leaf in a [`write_debug_scalars`] script.
///
/// The closed repertoire is what makes its output's line breaks knowable: none
/// of these types writes one except as fill. Composite values are expressed
/// with [`Tok`] containers and nodes, not opaque leaves.
#[derive(Clone, Copy)]
pub enum DebugScalar<'a> {
    /// A borrowed string, formatted by the standard string `Debug` implementation.
    Str(&'a str),
    /// An unsigned 32-bit integer.
    U32(u32),
    /// An unsigned 64-bit integer.
    U64(u64),
}

impl fmt::Debug for DebugScalar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Str(value) => value.fmt(f),
            Self::U32(value) => value.fmt(f),
            Self::U64(value) => value.fmt(f),
        }
    }
}

/// Write a fully scripted value exactly as the derive writes it, under every
/// formatter option: fill, alignment, width, precision, sign, `#`, `0` and the
/// hexadecimal `Debug` modes.
///
/// Unlike [`super::write_debug`], leaves are restricted to [`DebugScalar`]; script
/// each composite value with [`Tok`] so the single shared writer can preserve
/// nesting and indentation.
///
/// # Errors
///
/// What `f` returns; no formatter state is refused.
///
/// # Panics
///
/// When the script is unbalanced, as for [`super::write_debug`].
pub fn write_debug_scalars<'a, N, const K: usize>(
    f: &mut fmt::Formatter<'_>,
    root: N,
    mut script: impl FnMut(N, &mut WorkList<Tok<N, DebugScalar<'a>>, K>),
) -> fmt::Result {
    let mut resident = crate::allocation::Resident;
    let mut memory = crate::allocation::Memory::new(&mut resident);
    let script =
        |node,
         pending: &mut WorkList<Tok<N, DebugScalar<'a>>, K>,
         _: &mut crate::allocation::Memory<'_, crate::allocation::Resident>| {
            script(node, pending);
            Ok(())
        };
    if f.alternate() && f.fill() == '\n' {
        let options = Options::capture(f);
        write_debug_with(
            f,
            root,
            script,
            |leaf, out| options.emit(&mut NewlineFill(out), leaf),
            &mut memory,
            false,
        )
        .map_err(|_| fmt::Error)
    } else {
        write_debug_with(
            f,
            root,
            script,
            |leaf, out| fmt::Debug::fmt(&leaf, out.formatter()?),
            &mut memory,
            false,
        )
        .map_err(|_| fmt::Error)
    }
}

/// The options of a pretty-form formatter whose fill is a newline. The fill and
/// the `#` flag are therefore known; the rest is read here.
#[derive(Clone, Copy)]
struct Options {
    align: Option<fmt::Alignment>,
    width: Option<usize>,
    precision: Option<usize>,
    sign: Sign,
    zero: bool,
    mode: Mode,
}

#[derive(Clone, Copy)]
enum Sign {
    None,
    Plus,
    Minus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Normal,
    Lower,
    Upper,
}

/// The formatter's opaque option word. Stable Rust names no getter for the
/// hexadecimal `Debug` mode, so [`Mode::of`] compares this word with the words
/// standard format syntax itself produces for that mode; it never interprets a
/// fixed bit position.
#[allow(
    deprecated,
    reason = "the hexadecimal Debug mode has no other stable getter; its bits are calibrated against format syntax"
)]
fn option_word(f: &fmt::Formatter<'_>) -> u32 {
    f.flags()
}

/// Records the option word of the formatter it is written with.
struct Probe(Cell<u32>);

impl fmt::Debug for Probe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.set(option_word(f));
        Ok(())
    }
}

/// Accepts and drops every byte.
struct Discard;

impl fmt::Write for Discard {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        Ok(())
    }
}

impl Probe {
    /// The option word of the formatter `args` builds around this probe.
    fn word(&self, args: fmt::Arguments<'_>) -> u32 {
        // Neither the probe nor the sink can fail, so the result carries nothing.
        _ = fmt::write(&mut Discard, args);
        self.0.get()
    }
}

impl Mode {
    /// The word bits `{:x?}` and `{:X?}` add to `{:?}` select the mode; a word
    /// carrying neither is the decimal mode. Total: every formatter has a mode.
    fn of(f: &fmt::Formatter<'_>) -> Self {
        let probe = Probe(Cell::new(0));
        let normal = probe.word(format_args!("{probe:?}"));
        let lower = probe.word(format_args!("{probe:x?}")) & !normal;
        let upper = probe.word(format_args!("{probe:X?}")) & !normal;
        let word = option_word(f);
        if word & lower != 0 {
            Self::Lower
        } else if word & upper != 0 {
            Self::Upper
        } else {
            Self::Normal
        }
    }
}

impl Options {
    fn capture(f: &fmt::Formatter<'_>) -> Self {
        Self {
            align: f.align(),
            width: f.width(),
            precision: f.precision(),
            // Format syntax admits one sign; plus wins as it does in the
            // standard integer formatter.
            sign: if f.sign_plus() {
                Sign::Plus
            } else if f.sign_minus() {
                Sign::Minus
            } else {
                Sign::None
            },
            zero: f.sign_aware_zero_pad(),
            mode: Mode::of(f),
        }
    }

    /// Render `leaf` with these options and a NUL sentinel fill, which
    /// [`NewlineFill`] turns back into the newline. Standard primitive `Debug`
    /// remains in charge of digits, escaping, signs, prefixes, width and
    /// precision; these macros only select the equivalent format syntax.
    fn emit(&self, out: &mut dyn fmt::Write, leaf: DebugScalar<'_>) -> fmt::Result {
        macro_rules! parameters {
            ($align:literal, $sign:literal, $zero:literal, $mode:literal) => {
                match (self.width, self.precision) {
                    (None, None) => write!(
                        out,
                        concat!("{:\0", $align, $sign, "#", $zero, $mode, "}"),
                        leaf
                    ),
                    (Some(width), None) => write!(
                        out,
                        concat!("{:\0", $align, $sign, "#", $zero, "width$", $mode, "}"),
                        leaf,
                        width = width
                    ),
                    (None, Some(precision)) => write!(
                        out,
                        concat!("{:\0", $align, $sign, "#", $zero, ".precision$", $mode, "}"),
                        leaf,
                        precision = precision
                    ),
                    (Some(width), Some(precision)) => write!(
                        out,
                        concat!(
                            "{:\0",
                            $align,
                            $sign,
                            "#",
                            $zero,
                            "width$.precision$",
                            $mode,
                            "}"
                        ),
                        leaf,
                        width = width,
                        precision = precision
                    ),
                }
            };
        }
        macro_rules! mode {
            ($align:literal, $sign:literal, $zero:literal) => {
                match self.mode {
                    Mode::Normal => parameters!($align, $sign, $zero, "?"),
                    Mode::Lower => parameters!($align, $sign, $zero, "x?"),
                    Mode::Upper => parameters!($align, $sign, $zero, "X?"),
                }
            };
        }
        macro_rules! zero {
            ($align:literal, $sign:literal) => {
                if self.zero {
                    mode!($align, $sign, "0")
                } else {
                    mode!($align, $sign, "")
                }
            };
        }
        macro_rules! sign {
            ($align:literal) => {
                match self.sign {
                    Sign::None => zero!($align, ""),
                    Sign::Plus => zero!($align, "+"),
                    Sign::Minus => zero!($align, "-"),
                }
            };
        }
        // A fill needs an explicit alignment in format syntax; an absent one is
        // each standard formatter's own default: left for strings, right for
        // integers.
        let align = self.align.unwrap_or(match leaf {
            DebugScalar::Str(_) => fmt::Alignment::Left,
            DebugScalar::U32(_) | DebugScalar::U64(_) => fmt::Alignment::Right,
        });
        match align {
            fmt::Alignment::Left => sign!("<"),
            fmt::Alignment::Center => sign!("^"),
            fmt::Alignment::Right => sign!(">"),
        }
    }
}

/// Turns the sentinel padding back into newlines on the way into the indenting
/// writer, which then indents them at the actual container depth. A leaf's own
/// content holds no NUL: string `Debug` escapes it and integers have none.
struct NewlineFill<'p, 'f, 'g>(&'p mut Pad<'f, 'g>);

impl fmt::Write for NewlineFill<'_, '_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for piece in text.split_inclusive('\0') {
            if let Some(prefix) = piece.strip_suffix('\0') {
                fmt::Write::write_str(self.0, prefix)?;
                fmt::Write::write_char(self.0, '\n')?;
            } else {
                fmt::Write::write_str(self.0, piece)?;
            }
        }
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use core::fmt;

    use super::{DebugScalar, Mode, write_debug_scalars};
    use crate::walk::{Tok, WorkList};

    #[derive(Debug)]
    struct Scalars<'a> {
        text: &'a str,
        narrow: u32,
        wide: u64,
    }

    #[derive(Debug)]
    struct Outer<'a> {
        value: Option<Scalars<'a>>,
        empty: Option<u64>,
    }

    struct Scripted<'a>(&'a Outer<'a>);

    impl fmt::Debug for Scripted<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_debug_scalars(
                f,
                self.0,
                |value, out: &mut WorkList<Tok<&Outer<'_>, DebugScalar<'_>>, 4>| {
                    out.extend([Tok::Struct("Outer"), Tok::Field("value")]);
                    if let Some(value) = &value.value {
                        out.extend([
                            Tok::Tuple("Some"),
                            Tok::Struct("Scalars"),
                            Tok::Field("text"),
                            Tok::Leaf(DebugScalar::Str(value.text)),
                            Tok::Field("narrow"),
                            Tok::Leaf(DebugScalar::U32(value.narrow)),
                            Tok::Field("wide"),
                            Tok::Leaf(DebugScalar::U64(value.wide)),
                            Tok::EndStruct,
                            Tok::EndTuple,
                        ]);
                    } else {
                        out.push(Tok::Unit("None"));
                    }
                    out.push(Tok::Field("empty"));
                    if let Some(value) = value.empty {
                        out.extend([
                            Tok::Tuple("Some"),
                            Tok::Leaf(DebugScalar::U64(value)),
                            Tok::EndTuple,
                        ]);
                    } else {
                        out.push(Tok::Unit("None"));
                    }
                    out.push(Tok::EndStruct);
                },
            )
        }
    }

    fn fixtures() -> [Outer<'static>; 4] {
        [
            Outer {
                value: Some(Scalars {
                    text: "content\0\n🦀\"\\",
                    narrow: 0x1ab,
                    wide: u64::MAX,
                }),
                empty: None,
            },
            Outer {
                value: Some(Scalars {
                    text: "",
                    narrow: 0,
                    wide: 0,
                }),
                empty: Some(0x2cd),
            },
            Outer {
                value: Some(Scalars {
                    text: "three characters: abc",
                    narrow: u32::MAX,
                    wide: 0xdef,
                }),
                empty: Some(u64::MAX),
            },
            Outer {
                value: None,
                empty: None,
            },
        ]
    }

    // One home for the fixture loop, rather than a copy at every literal spec.
    #[inline(never)]
    fn compare_profile(profile: fn(&dyn fmt::Debug) -> String, label: &str) {
        for value in fixtures() {
            assert_eq!(
                profile(&Scripted(&value)),
                profile(&value),
                "format {label:?}",
            );
        }
    }

    /// Each literal formats the independent compiler derive directly; neither
    /// the oracle nor its padding passes through the implementation's emit path.
    /// `format!` panics if a `Debug` implementation reports an error, so every
    /// spec is also proved to be accepted.
    #[test]
    fn scalar_options_match_the_compiler_derive() {
        macro_rules! check {
            ($format:expr $(, $arg:ident = $argument:expr)*) => {
                compare_profile(
                    |value| format!($format, value $(, $arg = $argument)*),
                    $format,
                );
            };
        }
        macro_rules! dimensions {
            ($prefix:literal, $($flags:literal)+, $mode:literal) => {
                check!(concat!("{:", $prefix, $($flags,)+ $mode, "}"));
                check!(concat!("{:", $prefix, $($flags,)+ "30", $mode, "}"));
                check!(concat!("{:", $prefix, $($flags,)+ ".3", $mode, "}"));
                check!(concat!("{:", $prefix, $($flags,)+ "30.3", $mode, "}"));
                check!(
                    concat!("{:", $prefix, $($flags,)+ "width$.precision$", $mode, "}"),
                    width = 0,
                    precision = 0
                );
            };
        }
        macro_rules! modes {
            ($prefix:literal, $($flags:literal)+) => {
                dimensions!($prefix, $($flags)+, "?");
                dimensions!($prefix, $($flags)+, "x?");
                dimensions!($prefix, $($flags)+, "X?");
            };
        }
        macro_rules! flags {
            ($prefix:literal) => {
                modes!($prefix, "");
                modes!($prefix, "+");
                modes!($prefix, "-");
                modes!($prefix, "#");
                modes!($prefix, "+#");
                modes!($prefix, "-#");
                modes!($prefix, "0");
                modes!($prefix, "#" "0");
                modes!($prefix, "+#" "0");
            };
        }
        flags!("");
        flags!("<");
        flags!("^");
        flags!(">");
        flags!("\0<");
        flags!("\n<");
        flags!("\n^");
        flags!("\n>");
        flags!("🦀^");
    }

    /// The mode is read from the formatter itself, for each of the three.
    #[test]
    fn the_hexadecimal_mode_is_read_from_the_formatter() {
        struct Read;
        impl fmt::Debug for Read {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", Mode::of(f))
            }
        }
        assert_eq!(format!("{Read:?}"), "Normal");
        assert_eq!(format!("{Read:#?}"), "Normal");
        assert_eq!(format!("{Read:x?}"), "Lower");
        assert_eq!(format!("{Read:\n>+#030x?}"), "Lower");
        assert_eq!(format!("{Read:X?}"), "Upper");
        assert_eq!(format!("{Read:\n^-#.2X?}"), "Upper");
    }

    struct Refusal {
        left: usize,
        accepted: String,
    }

    impl fmt::Write for Refusal {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            for ch in text.chars() {
                if self.left == 0 {
                    return Err(fmt::Error);
                }
                self.accepted.push(ch);
                self.left -= 1;
            }
            Ok(())
        }
    }

    /// A writer's refusal surfaces at the same byte as the derive's, on both
    /// leaf paths.
    #[test]
    fn scalar_fill_and_nested_padding_propagate_every_refusal() {
        macro_rules! check {
            ($format:literal) => {
                for value in fixtures() {
                    let expected = format!($format, value);
                    for budget in 0..=expected.chars().count() {
                        let mut derived = Refusal {
                            left: budget,
                            accepted: String::new(),
                        };
                        let mut scripted = Refusal {
                            left: budget,
                            accepted: String::new(),
                        };
                        assert_eq!(
                            fmt::write(&mut scripted, format_args!($format, Scripted(&value))),
                            fmt::write(&mut derived, format_args!($format, value)),
                            "budget {budget}, format {:?}",
                            stringify!($format),
                        );
                        assert_eq!(scripted.accepted, derived.accepted);
                    }
                }
            };
        }
        check!("{:🦀^+#30.3X?}");
        check!("{:\n<+#30.3x?}");
        check!("{:\0>+#30.3?}");
        check!("{:🦀^+030x?}");
        check!("{:\n>#30?}");
        check!("{:#?}");
    }

    #[test]
    fn scalar_debug_itself_is_transparent() {
        macro_rules! check {
            ($value:expr, $scalar:expr) => {
                assert_eq!(format!("{:?}", $scalar), format!("{:?}", $value));
                assert_eq!(
                    format!("{:🦀^+#30.3x?}", $scalar),
                    format!("{:🦀^+#30.3x?}", $value)
                );
            };
        }
        check!("\0\n🦀", DebugScalar::Str("\0\n🦀"));
        check!(u32::MAX, DebugScalar::U32(u32::MAX));
        check!(u64::MAX, DebugScalar::U64(u64::MAX));
    }
}

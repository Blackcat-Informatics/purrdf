// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The inherent constructors whose whole body is one conversion.
//!
//! Every PurRDF type that carries a caller's text — an error's message, a
//! term's lexical form, a label — takes it the same way: as
//! `impl Into<String>`, so a `&str`, a `String` and a `format!` all work and
//! at most one allocation is made, converted once more into whatever the
//! field stores (`String`, `Arc<str>`, `Box<str>`). A type whose empty value is
//! its [`Default`] spells `new` as that default. A `From` impl that wraps a
//! lower layer's error into one of this layer's variants is that variant's
//! constructor and nothing else.
//!
//! These bodies are the same code whatever type they build, so they are
//! written once, here, as [`constructors!`](crate::constructors) and
//! [`variant_from!`](crate::variant_from), and every type instantiates them.
//! One spelling keeps the signature uniform across the workspace (no
//! constructor takes `&str` where its neighbour takes `String`) and leaves no
//! second body to drift.

/// Inherent constructors whose whole body is one conversion.
///
/// The macro takes one `impl` header (lifetime parameters allowed) and a list
/// of constructor declarations, each a signature with its body replaced by the
/// shape it builds. Attributes (documentation, `#[must_use]`, `#[inline]`) and
/// visibility are copied onto the generated function. A text parameter is
/// always `impl Into<String>`, converted once more into the field's own type.
///
/// | Declaration | Generated body |
/// |---|---|
/// | `fn name(text) -> Self::Variant;` | `Self::Variant(text.into().into())` |
/// | `fn name(text, at: usize) -> Self::Variant { .. };` | `Self::Variant { text: text.into().into(), at }` |
/// | `fn name(text) -> Self;` | `Self(text.into().into())` |
/// | `fn name(text) -> Self { .. };` | `Self { text: text.into().into() }` |
/// | `fn name() -> Self::default();` | `<Self as Default>::default()` |
///
/// In the brace forms the text parameter's name is its field's name, and every
/// further `name: Type` parameter is moved into the field of the same name.
///
/// ```rust
/// use std::sync::Arc;
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Refusal {
///     Config(String),
///     Syntax { reason: String, at: usize },
/// }
///
/// purrdf_lex::constructors! {
///     impl Refusal {
///         /// A configuration refusal.
///         pub fn config(what) -> Self::Config;
///         /// A syntax refusal at a byte offset.
///         pub fn syntax(reason, at: usize) -> Self::Syntax { .. };
///     }
/// }
///
/// #[derive(Debug, Default, PartialEq, Eq)]
/// struct Label {
///     text: Arc<str>,
/// }
///
/// purrdf_lex::constructors! {
///     impl Label {
///         /// Wrap a label.
///         pub fn new(text) -> Self { .. };
///         /// The empty label.
///         pub fn empty() -> Self::default();
///     }
/// }
///
/// assert_eq!(Refusal::config("no base"), Refusal::Config("no base".to_owned()));
/// assert_eq!(
///     Refusal::syntax(format!("unexpected {}", '}'), 7),
///     Refusal::Syntax { reason: "unexpected }".to_owned(), at: 7 },
/// );
/// assert_eq!(&*Label::new("a").text, "a");
/// assert_eq!(Label::empty(), Label::default());
/// ```
#[macro_export]
macro_rules! constructors {
    (impl <$($lifetime:lifetime),+> $type:ty { $($items:tt)* }) => {
        impl <$($lifetime),+> $type {
            $crate::constructors!(@items $($items)*);
        }
    };
    (impl $type:ty { $($items:tt)* }) => {
        impl $type {
            $crate::constructors!(@items $($items)*);
        }
    };
    (@items) => {};
    (@items
        $(#[$meta:meta])* $vis:vis fn $name:ident() -> Self::default();
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        $vis fn $name() -> Self {
            <Self as ::core::default::Default>::default()
        }
        $crate::constructors!(@items $($rest)*);
    };
    (@items
        $(#[$meta:meta])* $vis:vis fn $name:ident($text:ident) -> Self;
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        $vis fn $name($text: impl ::core::convert::Into<::std::string::String>) -> Self {
            Self($crate::__text_field!($text))
        }
        $crate::constructors!(@items $($rest)*);
    };
    (@items
        $(#[$meta:meta])* $vis:vis fn $name:ident($text:ident) -> Self { .. };
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        $vis fn $name($text: impl ::core::convert::Into<::std::string::String>) -> Self {
            Self { $text: $crate::__text_field!($text) }
        }
        $crate::constructors!(@items $($rest)*);
    };
    (@items
        $(#[$meta:meta])* $vis:vis fn $name:ident($text:ident) -> Self::$variant:ident;
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        $vis fn $name($text: impl ::core::convert::Into<::std::string::String>) -> Self {
            Self::$variant($crate::__text_field!($text))
        }
        $crate::constructors!(@items $($rest)*);
    };
    (@items
        $(#[$meta:meta])* $vis:vis fn $name:ident($text:ident $(, $field:ident: $field_type:ty)* $(,)?)
            -> Self::$variant:ident { .. };
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        $vis fn $name(
            $text: impl ::core::convert::Into<::std::string::String>,
            $($field: $field_type),*
        ) -> Self {
            Self::$variant {
                $text: $crate::__text_field!($text),
                $($field),*
            }
        }
        $crate::constructors!(@items $($rest)*);
    };
}

/// The one conversion [`constructors!`](crate::constructors) applies to a text
/// parameter: into `String`, then into the field's own type.
#[doc(hidden)]
#[macro_export]
macro_rules! __text_field {
    ($text:ident) => {
        ::core::convert::Into::into(::core::convert::Into::<::std::string::String>::into($text))
    };
}

/// `From` impls that wrap a source value into one variant of the target.
///
/// Each `Variant(Source)` entry generates `impl From<Source> for Target`,
/// whose body is `Self::Variant(value)`: the conversion `?` applies when a
/// lower layer's error crosses into this layer's.
///
/// ```rust
/// #[derive(Debug, PartialEq, Eq)]
/// enum Outer {
///     Parse(core::num::ParseIntError),
///     Utf8(core::str::Utf8Error),
/// }
///
/// purrdf_lex::variant_from!(Outer {
///     Parse(core::num::ParseIntError),
///     Utf8(core::str::Utf8Error),
/// });
///
/// fn read(text: &str) -> Result<u8, Outer> {
///     Ok(text.parse::<u8>()?)
/// }
///
/// assert!(matches!(read("x"), Err(Outer::Parse(_))));
/// assert_eq!(read("7"), Ok(7));
/// ```
#[macro_export]
macro_rules! variant_from {
    ($type:ty { $($variant:ident($source:ty)),+ $(,)? }) => {
        $(
            impl ::core::convert::From<$source> for $type {
                fn from(value: $source) -> Self {
                    Self::$variant(value)
                }
            }
        )+
    };
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[derive(Debug, PartialEq, Eq)]
    enum Refusal {
        Config(String),
        Shared(Arc<str>),
        Syntax {
            reason: String,
            at: usize,
        },
        Located {
            reason: Box<str>,
            line: u32,
            column: u32,
        },
    }

    crate::constructors! {
        impl Refusal {
            fn config(what) -> Self::Config;
            fn shared(what) -> Self::Shared;
            fn syntax(reason, at: usize) -> Self::Syntax { .. };
            fn located(reason, line: u32, column: u32) -> Self::Located { .. };
        }
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct Tuple(String);

    #[derive(Debug, Default, PartialEq, Eq)]
    struct Named {
        label: Arc<str>,
    }

    crate::constructors! {
        impl Tuple {
            fn new(text) -> Self;
            fn empty() -> Self::default();
        }
    }

    crate::constructors! {
        impl Named {
            fn new(label) -> Self { .. };
        }
    }

    struct Borrowing<'a> {
        items: Vec<&'a str>,
    }

    impl Default for Borrowing<'_> {
        fn default() -> Self {
            Self {
                items: vec!["seed"],
            }
        }
    }

    crate::constructors! {
        impl<'a> Borrowing<'a> {
            fn new() -> Self::default();
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Wrapped {
        Int(core::num::ParseIntError),
        Text(String),
    }

    crate::variant_from!(Wrapped {
        Int(core::num::ParseIntError),
        Text(String),
    });

    #[test]
    fn a_tuple_variant_takes_any_text() {
        assert_eq!(Refusal::config("a"), Refusal::Config("a".to_owned()));
        assert_eq!(
            Refusal::config(String::from("b")),
            Refusal::Config("b".to_owned())
        );
        assert_eq!(
            Refusal::shared(format!("{}", 1)),
            Refusal::Shared("1".into())
        );
    }

    #[test]
    fn a_struct_variant_moves_its_other_fields() {
        assert_eq!(
            Refusal::syntax("bad", 3),
            Refusal::Syntax {
                reason: "bad".to_owned(),
                at: 3
            }
        );
        assert_eq!(
            Refusal::located("gone", 2, 9),
            Refusal::Located {
                reason: "gone".into(),
                line: 2,
                column: 9
            }
        );
    }

    #[test]
    fn newtypes_convert_into_their_field() {
        assert_eq!(Tuple::new("x"), Tuple("x".to_owned()));
        assert_eq!(&*Named::new("y").label, "y");
    }

    #[test]
    fn a_default_constructor_is_the_default() {
        assert_eq!(Tuple::empty(), Tuple::default());
        assert_eq!(Borrowing::new().items, vec!["seed"]);
    }

    #[test]
    fn variant_from_wraps_the_source() {
        let error = "z".parse::<u8>().unwrap_err();
        assert_eq!(Wrapped::from(error.clone()), Wrapped::Int(error));
        assert_eq!(Wrapped::from("t".to_owned()), Wrapped::Text("t".to_owned()));
    }
}

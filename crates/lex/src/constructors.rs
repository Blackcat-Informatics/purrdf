// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The inherent constructors whose whole body is one conversion.
//!
//! Every PurRDF type that carries a caller's text — an error's message, a
//! term's lexical form, a label — takes it the same way: as
//! `impl Into<String>`, so a `&str`, a `String` and a `format!` all work and
//! at most one allocation is made, converted once more into whatever the
//! field stores (`String`, `Arc<str>`, `Box<str>`). A `From` impl that wraps a
//! lower layer's error into one of this layer's variants is that variant's
//! constructor and nothing else. (A type's empty value is not one of these: it
//! is `#[derive(Default)]`, or a `new` that `purrdf_hash::default_from_new!`
//! answers `Default` from.)
//!
//! These bodies are the same code whatever type they build, so they are
//! written once, here, as [`constructors!`](macro@crate::constructors) and
//! [`variant_from!`](crate::variant_from), and every type instantiates them.
//! One spelling keeps the signature uniform across the workspace (no
//! constructor takes `&str` where its neighbour takes `String`) and leaves no
//! second body to drift.

/// Implement resident `Clone` through a type's original admitted clone body.
/// Native callers use `clone_with_memory` directly and retain its admission.
#[macro_export]
macro_rules! resident_clone {
    ($type:ty) => {
        impl Clone for $type {
            fn clone(&self) -> Self {
                let mut resident = $crate::allocation::Resident;
                self.clone_with_memory(&mut $crate::allocation::Memory::new(&mut resident))
                    .expect("resident diagnostic clone allocation")
            }
        }
    };
}

/// Inherent constructors whose whole body is one conversion.
///
/// The macro takes one `impl` header (lifetime parameters allowed) and a list
/// of constructor declarations, each a signature with its body replaced by the
/// shape it builds. Attributes (documentation, `#[must_use]`, `#[inline]`) and
/// visibility are copied onto the generated function. An untyped text parameter
/// accepts `impl Into<String>`, converted once more into the field's own type.
/// A declared source type moves its original value directly through `Into`,
/// including an already admitted immutable text owner.
///
/// | Declaration | Generated body |
/// |---|---|
/// | `fn name(text) -> Self::Variant;` | `Self::Variant(text.into())` |
/// | `fn name(text, at: usize) -> Self::Variant { .. };` | `Self::Variant { text: text.into(), at }` |
/// | `fn name(text) -> Self;` | `Self(text.into())` |
/// | `fn name(text) -> Self { .. };` | `Self { text: text.into() }` |
/// | `fn name(text: Source) -> Self { .. };` | `Self { text: text.into() }` |
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
/// #[derive(Debug, PartialEq, Eq)]
/// struct Label {
///     text: Arc<str>,
/// }
///
/// purrdf_lex::constructors! {
///     impl Label {
///         /// Wrap a label.
///         pub fn new(text) -> Self { .. };
///     }
/// }
///
/// assert_eq!(Refusal::config("no base"), Refusal::Config("no base".to_owned()));
/// assert_eq!(
///     Refusal::syntax(format!("unexpected {}", '}'), 7),
///     Refusal::Syntax { reason: "unexpected }".to_owned(), at: 7 },
/// );
/// assert_eq!(&*Label::new("a").text, "a");
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
    (@items $(#[$meta:meta])* $visibility:vis fn $name:ident($field:ident: $source:ty) -> Self { .. }; $($rest:tt)*) => {
        $(#[$meta])*
        $visibility fn $name($field: $source) -> Self {
            Self { $field: $field.into() }
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
/// A variant that holds a rendering of its sources rather than the sources
/// themselves takes them as one group, `Variant(Source, ..) as convert`, and
/// each body is `Self::Variant(convert(&value))` — `ToString::to_string` for a
/// variant that carries the source's message.
///
/// A generic target names its unbounded type parameters in an `impl<T, U>`
/// header. Both forms support that header, including source types that use the
/// parameters; the conversion must be valid for every instantiation.
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
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Rendered {
///     Message(String),
/// }
///
/// purrdf_lex::variant_from!(Rendered {
///     Message(core::num::ParseIntError, core::str::Utf8Error) as ToString::to_string
/// });
///
/// let error = "x".parse::<u8>().unwrap_err();
/// assert_eq!(Rendered::from(error.clone()), Rendered::Message(error.to_string()));
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Operational<E> {
///     Read(E),
///     Message(String),
/// }
///
/// purrdf_lex::variant_from!(impl<E> Operational<E> {
///     Message(core::num::ParseIntError, core::fmt::Error) as ToString::to_string
/// });
///
/// assert_eq!(
///     Operational::<core::convert::Infallible>::from(error.clone()),
///     Operational::Message(error.to_string()),
/// );
/// ```
#[macro_export]
macro_rules! variant_from {
    (impl <$($parameter:ident),+ $(,)?> $type:ty { $($items:tt)* }) => {
        $crate::variant_from!(@variants [<$($parameter),+>] $type { $($items)* });
    };
    ($type:ty { $($items:tt)* }) => {
        $crate::variant_from!(@variants [] $type { $($items)* });
    };
    (@variants $parameters:tt $type:ty {
        $variant:ident($($source:ty),+ $(,)?) as $convert:path
    }) => {
        $(
            $crate::variant_from!(@render $parameters $type {
                $variant($source) as $convert
            });
        )+
    };
    (@variants $parameters:tt $type:ty {
        $($variant:ident($source:ty)),+ $(,)?
    }) => {
        $(
            $crate::variant_from!(@wrap $parameters $type { $variant($source) });
        )+
    };
    (@render [$($parameters:tt)*] $type:ty {
        $variant:ident($source:ty) as $convert:path
    }) => {
        impl $($parameters)* ::core::convert::From<$source> for $type {
            fn from(value: $source) -> Self {
                Self::$variant($convert(&value))
            }
        }
    };
    (@wrap [$($parameters:tt)*] $type:ty { $variant:ident($source:ty) }) => {
        impl $($parameters)* ::core::convert::From<$source> for $type {
            fn from(value: $source) -> Self {
                Self::$variant(value)
            }
        }
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

    #[derive(Debug, PartialEq, Eq)]
    struct Tuple(String);

    #[derive(Debug, PartialEq, Eq)]
    struct Named {
        label: Arc<str>,
    }

    crate::constructors! {
        impl Tuple {
            fn new(text) -> Self;
        }
    }

    crate::constructors! {
        impl Named {
            fn new(label) -> Self { .. };
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Borrowing<'a> {
        Owned(String),
        Borrowed(&'a str),
    }

    crate::constructors! {
        impl<'a> Borrowing<'a> {
            fn owned(text) -> Self::Owned;
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

    #[derive(Debug, PartialEq, Eq)]
    enum Rendered {
        Message(String),
    }

    crate::variant_from!(Rendered {
        Message(core::num::ParseIntError, core::fmt::Error) as ToString::to_string
    });

    #[derive(Debug, PartialEq, Eq)]
    enum GenericWrapped<T> {
        Value(T),
    }

    crate::variant_from!(impl<T> GenericWrapped<T> {
        Value(T),
    });

    #[derive(Debug, PartialEq, Eq)]
    enum GenericError<E, V> {
        Operational(E, V),
        Parse(core::num::ParseIntError),
        Utf8(core::str::Utf8Error),
    }

    crate::variant_from!(impl<E, V> GenericError<E, V> {
        Parse(core::num::ParseIntError),
        Utf8(core::str::Utf8Error),
    });

    #[derive(Debug, PartialEq, Eq)]
    struct GenericSource<T>(T);

    impl<T> core::fmt::Display for GenericSource<T> {
        /// Render without inspecting `T`, so the conversion test can detect an
        /// unintended trait bound on an otherwise opaque generic parameter.
        fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            output.write_str("generic source")
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum GenericRendered<E> {
        Operational(E),
        Message(String),
    }

    crate::variant_from!(impl<E> GenericRendered<E> {
        Message(core::num::ParseIntError, core::fmt::Error, GenericSource<E>) as ToString::to_string
    });

    struct Opaque;

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
    fn a_type_with_lifetime_parameters_takes_constructors() {
        assert_eq!(
            Borrowing::owned("seed"),
            Borrowing::Owned("seed".to_owned())
        );
        assert_ne!(Borrowing::owned("seed"), Borrowing::Borrowed("seed"));
    }

    #[test]
    fn variant_from_wraps_the_source() {
        let error = "z".parse::<u8>().unwrap_err();
        assert_eq!(Wrapped::from(error.clone()), Wrapped::Int(error));
        assert_eq!(Wrapped::from("t".to_owned()), Wrapped::Text("t".to_owned()));
    }

    #[test]
    fn variant_from_renders_each_grouped_source() {
        let error = "z".parse::<u8>().unwrap_err();
        assert_eq!(
            Rendered::from(error.clone()),
            Rendered::Message(error.to_string())
        );
        assert_eq!(
            Rendered::from(core::fmt::Error),
            Rendered::Message(core::fmt::Error.to_string())
        );
    }

    /// A generic wrapping conversion moves its source without cloning the owner.
    #[test]
    fn variant_from_wraps_a_generic_source_without_cloning() {
        let source = Arc::new(String::from("owned source"));
        let witness = Arc::clone(&source);
        let GenericWrapped::Value(wrapped) = GenericWrapped::from(source);
        assert!(Arc::ptr_eq(&wrapped, &witness));
        assert_eq!(Arc::strong_count(&witness), 2);
    }

    /// Each fixed source converts independently of the target's opaque parameters.
    #[test]
    fn variant_from_wraps_each_source_for_multiple_type_parameters() {
        let error = "z".parse::<u8>().unwrap_err();
        assert_eq!(
            GenericError::<u8, bool>::from(error.clone()),
            GenericError::Parse(error),
        );
        let invalid = [u8::MAX];
        let utf8 = core::str::from_utf8(&invalid).unwrap_err();
        assert_eq!(
            GenericError::<u8, bool>::from(utf8),
            GenericError::Utf8(utf8),
        );
        let operational = GenericError::Operational(Opaque, Opaque);
        assert!(matches!(operational, GenericError::Operational(_, _)));
        assert!(matches!(
            GenericError::<Opaque, Opaque>::from(utf8),
            GenericError::Utf8(_),
        ));
    }

    /// Grouped rendering accepts both fixed and generic sources without imposing
    /// a display or clone bound on the target's operational error parameter.
    #[test]
    fn variant_from_renders_fixed_and_generic_sources_for_a_generic_error() {
        let error = "z".parse::<u8>().unwrap_err();
        assert_eq!(
            GenericRendered::<u8>::from(error.clone()),
            GenericRendered::Message(error.to_string()),
        );
        assert_eq!(
            GenericRendered::<u8>::from(core::fmt::Error),
            GenericRendered::Message(core::fmt::Error.to_string()),
        );
        assert_eq!(
            GenericRendered::from(GenericSource(7_u8)),
            GenericRendered::Message("generic source".to_owned()),
        );
        let operational = GenericRendered::Operational(Opaque);
        assert!(matches!(operational, GenericRendered::Operational(_)));
        assert!(matches!(
            GenericRendered::from(GenericSource(Opaque)),
            GenericRendered::Message(message) if message == "generic source",
        ));
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`message_error!`](crate::message_error): an error type whose whole content is
//! one human-readable message.

/// Declare an error type whose whole content is one human-readable message.
///
/// Many codecs refuse with a sentence and nothing else to match on. Each such
/// declaration supplies the struct holding the message, `new`, verbatim
/// `Display`, standard `Error` and borrowed structured inspection. These are
/// spelled once, here; each type is one invocation. The attributes written
/// on the struct (documentation, derives) are kept; `new` has the struct's
/// visibility. Adding `detail` after the name also gives the type a
/// `detail(&self) -> &str` accessor.
/// Borrowed structured inspection exposes the original text payload; this does
/// not turn a rendered native cause into structured fields at its producer.
///
/// ```rust
/// purrdf_lex::message_error! {
///     /// Why a widget was refused.
///     #[derive(Clone, Debug, PartialEq, Eq)]
///     pub struct WidgetError, detail;
/// }
///
/// let error = WidgetError::new("the widget is bent");
/// assert_eq!(error.to_string(), "the widget is bent");
/// assert_eq!(error.detail(), "the widget is bent");
/// let _: &dyn std::error::Error = &error;
/// ```
#[macro_export]
macro_rules! message_error {
    ($(#[$meta:meta])* $vis:vis struct $name:ident, detail;) => {
        $crate::message_error! { $(#[$meta])* $vis struct $name; }

        impl $name {
            /// The human-readable message, verbatim.
            #[must_use]
            $vis fn detail(&self) -> &str {
                &self.detail
            }
        }
    };
    ($(#[$meta:meta])* $vis:vis struct $name:ident;) => {
        $(#[$meta])*
        $vis struct $name {
            detail: ::std::string::String,
        }

        impl $name {
            /// An error whose whole message is `detail`.
            $vis fn new(detail: impl ::core::convert::Into<::std::string::String>) -> Self {
                Self {
                    detail: detail.into(),
                }
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(&self.detail)
            }
        }

        impl ::core::error::Error for $name {}

        impl $crate::error::StructuredError for $name {
            fn visit<'error>(
                &'error self,
                visitor: &mut dyn $crate::error::ErrorVisitor<'error>,
            ) {
                let Self { detail } = self;
                visitor.variant(concat!("message-", stringify!($name)));
                visitor.value(Some("detail"), $crate::error::ErrorValue::Text(detail));
            }
        }
    };
}

/// Implement an enum error's borrowed cause through its named wrapper variants.
///
/// Each named variant contains one error. Other variants describe the failure
/// themselves and have no underlying cause. The original error is borrowed,
/// preserving its type for downcasting without allocating or rendering it.
/// The enum already supplies its `Debug` and `Display` implementations.
///
/// ```rust
/// #[derive(Debug)]
/// enum Refusal {
///     Storage(purrdf_lex::allocation::StorageError),
///     InvalidInput,
/// }
/// impl core::fmt::Display for Refusal {
///     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
///         write!(f, "{self:?}")
///     }
/// }
/// purrdf_lex::error_source!(Refusal { Storage });
/// use core::error::Error as _;
/// let error = Refusal::Storage(purrdf_lex::allocation::StorageError::SizeOverflow);
/// assert_eq!(
///     error.source().unwrap().downcast_ref(),
///     Some(&purrdf_lex::allocation::StorageError::SizeOverflow),
/// );
/// assert!(Refusal::InvalidInput.source().is_none());
/// ```
#[macro_export]
macro_rules! error_source {
    ($type:ty { $($variant:ident),+ $(,)? }) => {
        impl ::core::error::Error for $type {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                match self {
                    $(Self::$variant(error) => Some(error),)+
                    _ => None,
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    crate::message_error! {
        /// A test error without the accessor.
        #[derive(Debug)]
        struct Plain;
    }

    crate::message_error! {
        /// A test error with the accessor.
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct Detailed, detail;
    }

    #[test]
    fn the_message_is_displayed_verbatim() {
        assert_eq!(Plain::new("a: b").to_string(), "a: b");
        assert_eq!(Plain::new(String::from("owned")).to_string(), "owned");
    }

    #[test]
    fn detail_answers_the_message() {
        let error = Detailed::new("why");
        assert_eq!(error.detail(), "why");
        let copy = error.clone();
        assert_eq!(copy, error);
        assert_ne!(copy, Detailed::new("other"));
    }

    #[test]
    fn the_type_is_a_standard_error_without_a_source() {
        let error = Plain::new("x");
        let dynamic: &dyn core::error::Error = &error;
        assert!(dynamic.source().is_none());
    }
}

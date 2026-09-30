// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`message_error!`](crate::message_error): an error type whose whole content is
//! one human-readable message.

/// Declare an error type whose whole content is one human-readable message.
///
/// Many codecs refuse with a sentence and nothing else to match on. Each such
/// type is the same four items — the struct holding the message, `new`, the
/// `Display` that writes the message verbatim and the `Error` impl — so they are
/// spelled once, here, and each type is one invocation. The attributes written
/// on the struct (documentation, derives) are kept; `new` has the struct's
/// visibility. Adding `detail` after the name also gives the type a
/// `detail(&self) -> &str` accessor.
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

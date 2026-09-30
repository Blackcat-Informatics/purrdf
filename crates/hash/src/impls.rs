// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Trait impls whose body is the same for every type of one shape, each spelled
//! once as a macro every such type invokes:
//! [`debug_non_exhaustive!`](crate::debug_non_exhaustive) (a `Debug` that shows
//! some fields and elides the rest) and
//! [`default_from_new!`](crate::default_from_new) (a `Default` that is the type's
//! `new`).

/// Implement `Debug` for a struct as the named fields and then `..`.
///
/// A type that holds a closure, a writer, a buffer or any other field with no
/// useful `Debug` shows the fields that identify it and elides the rest —
/// `Trial { name: "a", ignored: false, .. }` — through the standard library's
/// `DebugStruct::finish_non_exhaustive`. Every such impl is this one body,
/// spelled once: the invocation names the type (with its generic parameters
/// and their bounds in brackets, when it has any) and the fields shown, in
/// order. Attributes before the type (a doc comment on why the others are
/// elided) are kept on the generated `fmt`.
///
/// ```rust
/// struct Job<F> {
///     name: &'static str,
///     retries: u8,
///     run: F,
/// }
///
/// purrdf_hash::debug_non_exhaustive!([F] Job<F> { name, retries });
///
/// let job = Job { name: "sweep", retries: 2, run: || () };
/// assert_eq!(format!("{job:?}"), r#"Job { name: "sweep", retries: 2, .. }"#);
/// (job.run)();
/// ```
#[macro_export]
macro_rules! debug_non_exhaustive {
    (
        $(#[$meta:meta])*
        $([$($generics:tt)*])? $name:ident $(<$($arg:tt),+>)? { $($field:ident),+ $(,)? }
    ) => {
        impl $(<$($generics)*>)? ::core::fmt::Debug for $name $(<$($arg),+>)? {
            $(#[$meta])*
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(::core::stringify!($name))
                    $(.field(::core::stringify!($field), &self.$field))+
                    .finish_non_exhaustive()
            }
        }
    };
}

/// Implement `Default` as the type's own no-argument `new`.
///
/// The workspace's one constructor convention: a type whose default is what
/// `#[derive(Default)]` produces derives it and has no `new`; a type whose empty
/// value must be a `const fn`, or differs from the derived one — a hasher's
/// initial state, a sink with its default scope open — writes that value once, in
/// `new`, and answers `Default` through this impl, so the two can never diverge.
/// The invocation names the type, with its generic parameters and their bounds in
/// brackets when it has any; attributes before the type are kept on the generated
/// `default`. A type whose no-argument constructor carries a more specific name
/// than `new` names it after `=>`.
///
/// ```rust
/// struct Counter {
///     count: u64,
/// }
///
/// impl Counter {
///     const fn new() -> Self {
///         Self { count: 1 }
///     }
/// }
///
/// purrdf_hash::default_from_new!(Counter);
///
/// assert_eq!(Counter::default().count, 1);
///
/// struct Filter {
///     narrowed: bool,
/// }
///
/// impl Filter {
///     const fn unconstrained() -> Self {
///         Self { narrowed: false }
///     }
/// }
///
/// purrdf_hash::default_from_new!(Filter => unconstrained);
///
/// assert!(!Filter::default().narrowed);
/// ```
#[macro_export]
macro_rules! default_from_new {
    (
        $(#[$meta:meta])*
        $([$($generics:tt)*])? $name:ident $(<$($arg:tt),+>)? => $constructor:ident
    ) => {
        impl $(<$($generics)*>)? ::core::default::Default for $name $(<$($arg),+>)? {
            $(#[$meta])*
            fn default() -> Self {
                Self::$constructor()
            }
        }
    };
    (
        $(#[$meta:meta])*
        $([$($generics:tt)*])? $name:ident $(<$($arg:tt),+>)?
    ) => {
        $crate::default_from_new!(
            $(#[$meta])*
            $([$($generics)*])? $name $(<$($arg),+>)? => new
        );
    };
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    struct Plain {
        shown: u8,
        also: &'static str,
        #[allow(dead_code, reason = "the elided field is never printed")]
        hidden: fn(),
    }

    crate::debug_non_exhaustive!(Plain { shown, also });

    struct Generic<T, F> {
        value: T,
        #[allow(dead_code, reason = "the elided field is never printed")]
        run: F,
    }

    crate::debug_non_exhaustive!(
        /// Only the value has a `Debug`.
        [T: Debug, F] Generic<T, F> { value }
    );

    #[test]
    fn the_named_fields_print_and_the_rest_are_elided() {
        let plain = Plain {
            shown: 1,
            also: "x",
            hidden: || (),
        };
        assert_eq!(format!("{plain:?}"), r#"Plain { shown: 1, also: "x", .. }"#);
        assert_eq!(
            format!("{plain:#?}"),
            "Plain {\n    shown: 1,\n    also: \"x\",\n    ..\n}"
        );
    }

    struct Start<const N: usize>(usize);

    impl<const N: usize> Start<N> {
        const fn new() -> Self {
            Self(N)
        }
    }

    crate::default_from_new!(
        /// The start is `N`.
        [const N: usize] Start<N>
    );

    #[test]
    fn default_is_the_types_new() {
        assert_eq!(Start::<3>::default().0, 3);
    }

    struct Named(u8);

    impl Named {
        const fn unconstrained() -> Self {
            Self(7)
        }
    }

    crate::default_from_new!(Named => unconstrained);

    #[test]
    fn default_is_the_named_constructor() {
        assert_eq!(Named::default().0, 7);
    }

    #[test]
    fn generic_parameters_and_their_bounds_are_carried() {
        let generic = Generic {
            value: [1_u8, 2],
            run: |x: u8| x,
        };
        assert_eq!(format!("{generic:?}"), "Generic { value: [1, 2], .. }");
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed error inspection, independent of human presentation.
//!
//! Producers report a stable variant code and every semantic field. A consumer
//! handles this fixed value algebra, rather than matching each producer's enum.
//! Stable nested owners are references a consumer can put on its own work list.
//! Temporary views emit balanced record events with fields borrowed from their
//! original owner. Inspection neither formats nor clones a payload.

mod io;
pub use io::{IoFailure, io_error};

/// Implement the standard error source chain from single-field enum variants
/// or a native-cause accessor (`error_sources!(Type, native accessor)`). Other
/// enum variants have no underlying error. The original cause is borrowed
/// without rendering, cloning or type erasure beyond the Error trait.
#[macro_export]
macro_rules! error_sources {
    ($kind:ty, native $method:ident) => {
        impl ::core::error::Error for $kind {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                self.$method()
                    .map(|cause| cause as &dyn ::core::error::Error)
            }
        }
    };
    ($kind:ty, $($variant:ident),+ $(,)?) => {
        impl ::core::error::Error for $kind {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                match self {
                    $(Self::$variant(cause) => Some(cause),)+
                    _ => None,
                }
            }
        }
    };
}

/// An error, or a structured compound value carried by an error.
///
/// Variant codes and field names are stable public identities. Adding a producer
/// variant or field does not add a variant to [`ErrorValue`]. Implementations
/// report all semantic fields, including explicit absence and empty lists.
/// Standard-library I/O categories report a portable `name` for public stable
/// categories and the original `native_discriminant` for every category.
/// That native discriminant is compiler-specific, not a persistent error code;
/// hidden or future categories have an absent name and retain their exact value.
pub trait StructuredError {
    /// Report this value's variant and fields without rendering it to text.
    /// Borrowed payloads remain valid for the original value's lifetime.
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>);
}

/// An original error whose typed fields survive shared or boxed type erasure.
/// Standard error upcasting preserves downcasting and the source chain, while
/// structured upcasting preserves borrowed fields without rendering a message.
pub trait NativeError: std::error::Error + StructuredError + Send + Sync {}
impl<T: std::error::Error + StructuredError + Send + Sync> NativeError for T {}

impl StructuredError for core::convert::Infallible {
    fn visit<'error>(&'error self, _visitor: &mut dyn ErrorVisitor<'error>) {
        // Generic query errors can name Infallible without ever carrying one.
        // Do not dereference an uninhabited reference to implement that bound.
        unreachable!("an Infallible error has no value to inspect")
    }
}

impl StructuredError for core::fmt::Error {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("std-format");
    }
}

impl StructuredError for core::num::TryFromIntError {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        // The standard error has no payload; the native producer retains the
        // source value and destination limit alongside this original cause.
        visitor.variant("integer-conversion-range");
    }
}

impl StructuredError for core::num::IntErrorKind {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let name = match self {
            Self::Empty => Some("empty"),
            Self::InvalidDigit => Some("invalid-digit"),
            Self::PosOverflow => Some("positive-overflow"),
            Self::NegOverflow => Some("negative-overflow"),
            Self::Zero => Some("zero"),
            _ => None,
        };
        visitor.variant("std-integer-kind");
        visitor.value(
            Some("name"),
            name.map_or(ErrorValue::Absent, ErrorValue::Code),
        );
        // As for ErrorKind, retain unlisted external unit categories without
        // formatting them or confusing them with a known category.
        visitor.value(
            Some("native_discriminant"),
            ErrorValue::Unsigned(*self as u128),
        );
    }
}

impl StructuredError for core::num::ParseIntError {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("std-integer-parse");
        visitor.value(Some("kind"), ErrorValue::Cause(self.kind()));
    }
}

impl StructuredError for core::str::Utf8Error {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("std-utf8");
        visitor.value(
            Some("valid_up_to"),
            ErrorValue::ByteCount(self.valid_up_to() as u128),
        );
        visitor.value(
            Some("error_len"),
            self.error_len()
                .map_or(ErrorValue::Absent, |len| ErrorValue::ByteCount(len as u128)),
        );
    }
}

impl StructuredError for std::time::SystemTimeError {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("std-system-time");
        let duration = self.duration();
        visitor.value(
            Some("seconds"),
            ErrorValue::Unsigned(u128::from(duration.as_secs())),
        );
        visitor.value(
            Some("nanoseconds"),
            ErrorValue::Unsigned(u128::from(duration.subsec_nanos())),
        );
    }
}

impl StructuredError for std::io::ErrorKind {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        // Public stable categories have portable names. The standard library
        // also produces hidden categories and may add further ones. Retain
        // their actual unit discriminant rather than calling them Other or
        // formatting them. This native value is not a persistent identity.
        let name = match self {
            Self::NotFound => Some("not-found"),
            Self::PermissionDenied => Some("permission-denied"),
            Self::ConnectionRefused => Some("connection-refused"),
            Self::ConnectionReset => Some("connection-reset"),
            Self::HostUnreachable => Some("host-unreachable"),
            Self::NetworkUnreachable => Some("network-unreachable"),
            Self::ConnectionAborted => Some("connection-aborted"),
            Self::NotConnected => Some("not-connected"),
            Self::AddrInUse => Some("address-in-use"),
            Self::AddrNotAvailable => Some("address-not-available"),
            Self::NetworkDown => Some("network-down"),
            Self::BrokenPipe => Some("broken-pipe"),
            Self::AlreadyExists => Some("already-exists"),
            Self::WouldBlock => Some("would-block"),
            Self::NotADirectory => Some("not-a-directory"),
            Self::IsADirectory => Some("is-a-directory"),
            Self::DirectoryNotEmpty => Some("directory-not-empty"),
            Self::ReadOnlyFilesystem => Some("read-only-filesystem"),
            Self::StaleNetworkFileHandle => Some("stale-network-file-handle"),
            Self::InvalidInput => Some("invalid-input"),
            Self::InvalidData => Some("invalid-data"),
            Self::TimedOut => Some("timed-out"),
            Self::WriteZero => Some("write-zero"),
            Self::StorageFull => Some("storage-full"),
            Self::NotSeekable => Some("not-seekable"),
            Self::QuotaExceeded => Some("quota-exceeded"),
            Self::FileTooLarge => Some("file-too-large"),
            Self::ResourceBusy => Some("resource-busy"),
            Self::ExecutableFileBusy => Some("executable-file-busy"),
            Self::Deadlock => Some("deadlock"),
            Self::CrossesDevices => Some("crosses-devices"),
            Self::TooManyLinks => Some("too-many-links"),
            Self::InvalidFilename => Some("invalid-filename"),
            Self::ArgumentListTooLong => Some("argument-list-too-long"),
            Self::Interrupted => Some("interrupted"),
            Self::Unsupported => Some("unsupported"),
            Self::UnexpectedEof => Some("unexpected-eof"),
            Self::OutOfMemory => Some("out-of-memory"),
            Self::Other => Some("other"),
            _ => None,
        };
        visitor.variant("std-io-kind");
        visitor.value(
            Some("name"),
            name.map_or(ErrorValue::Absent, ErrorValue::Code),
        );
        visitor.value(
            Some("native_discriminant"),
            ErrorValue::Unsigned(*self as u128),
        );
    }
}

/// One field value. These primitive kinds form a closed interchange algebra.
///
/// Compound values use [`Self::Record`], [`Self::Term`] or [`Self::Cause`]; lists
/// use the visitor's list events. New producer variants and fields therefore do
/// not require a consumer wildcard or an enum mirror. Float bit patterns retain
/// every value, including negative zero and NaN payloads, without formatting.
#[derive(Clone, Copy)]
pub enum ErrorValue<'error> {
    /// An optional field is present in the schema but has no value.
    Absent,
    /// An exact unsigned integer, including full-width capacity counters.
    Unsigned(u128),
    /// An exact signed integer.
    Signed(i128),
    /// A Boolean condition.
    Boolean(bool),
    /// An IEEE 754 binary32 value's original bits.
    Float32Bits(u32),
    /// An IEEE 754 binary64 value's original bits.
    Float64Bits(u64),
    /// One Unicode scalar value.
    Character(char),
    /// A machine-readable code or discriminant, distinct from prose.
    Code(&'error str),
    /// Text that is itself data, such as an input lexical form.
    Text(&'error str),
    /// A genuine caller or external message, not a rendered native cause.
    Message(&'error str),
    /// An IRI-valued field; inspection does not parse or normalize its bytes.
    Iri(&'error str),
    /// Original bytes, without an encoding or allocation.
    Bytes(&'error [u8]),
    /// A byte quantity, distinct from an unqualified integer.
    ByteCount(u128),
    /// A nested error retaining its own stable variant and fields.
    Cause(&'error dyn StructuredError),
    /// A compound non-error payload retaining its variant and fields.
    Record(&'error dyn StructuredError),
    /// A structured RDF term payload, visited without converting it to a string.
    Term(&'error dyn StructuredError),
}

// A debug view must not accidentally render a retained or recursively nested
// cause. The visitor, not Debug, is the complete structural interface.
crate::debug_non_exhaustive!(ErrorValue<'_> {});

/// Receives one structured error and its fields.
///
/// A value starts with `variant`. Named fields have `Some(name)`; list entries
/// have `None`. A list's begin/end pair encloses its values, including nested
/// lists. Nested records/causes are borrowed values: the producer does not call
/// their visitor recursively. No method has a default that can discard a field.
pub trait ErrorVisitor<'error> {
    /// Stable identity of the current error or compound payload variant.
    fn variant(&mut self, code: &'static str);

    /// A named field or an unnamed list entry, preserving its primitive kind.
    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>);

    /// Begin a named list field or an unnamed nested list entry.
    fn begin_list(&mut self, name: Option<&'static str>, length: usize);

    /// End the most recently begun list.
    fn end_list(&mut self);

    /// Begin a compound view whose temporary adapter need not survive inspection.
    /// Its field values still borrow the original owner for `'error`. A named
    /// record is a field; an unnamed record is an entry of the enclosing list.
    fn begin_record(&mut self, name: Option<&'static str>, code: &'static str);

    /// End the most recently begun record, after its fields and nested lists.
    fn end_record(&mut self);
}

impl StructuredError for () {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("unit");
    }
}

// Native views use these scalar counters as their original evidence. The
// consumer sees the exact count rather than needing a wrapper or a downcast.
macro_rules! unsigned_evidence {
    ($($integer:ty => $widen:expr),* $(,)?) => {$ (
        impl StructuredError for $integer {
            fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
                visitor.variant("unsigned-evidence");
                visitor.value(Some("value"), ErrorValue::Unsigned(($widen)(*self)));
            }
        }
    )*};
}
unsigned_evidence!(u32 => u128::from, usize => |value: usize| value as u128);

impl StructuredError for str {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("text-value");
        visitor.value(Some("text"), ErrorValue::Text(self));
    }
}

impl StructuredError for String {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        self.as_str().visit(visitor);
    }
}

impl<First: StructuredError, Second: StructuredError> StructuredError for (First, Second) {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let (first, second) = self;
        visitor.variant("pair");
        visitor.value(Some("first"), ErrorValue::Record(first));
        visitor.value(Some("second"), ErrorValue::Record(second));
    }
}

impl<T: StructuredError + ?Sized> StructuredError for &T {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        T::visit(*self, visitor);
    }
}

/// Emit a borrowed list without materializing or cloning its items.
///
/// The iterator's exact length includes absent entries. A consumer receives an
/// empty begin/end pair for an empty list, rather than losing the field.
pub fn visit_list<'error>(
    visitor: &mut dyn ErrorVisitor<'error>,
    name: Option<&'static str>,
    values: impl ExactSizeIterator<Item = ErrorValue<'error>>,
) {
    visitor.begin_list(name, values.len());
    for value in values {
        visitor.value(None, value);
    }
    visitor.end_list();
}

/// Implement an enum's borrowed visitor through exhaustive producer patterns.
///
/// Use inside its [`StructuredError`](crate::error::StructuredError) impl.
/// Every variant and named/tuple field
/// must be listed; the grammar supplies no wildcard or rest pattern. New fields
/// or variants therefore require producer support at compilation, while a
/// consumer continues to handle the fixed [`ErrorValue`](crate::error::ErrorValue)
/// algebra. A field is
/// `"name" => Kind(value)`, `"name" => Absent`, or `"name" => List(iterator)`.
/// `Value(expression)` forwards an existing value, for example an optional
/// cause mapped to either `Cause` or `Absent`.
/// For a named struct, `@record self { all, physical, fields } => "code" { ... }`
/// checks its complete shape while expressions use its existing semantic
/// accessors. Encoded private fields need not be decoded a second time.
///
/// A producer cannot add a variant without exposing it:
///
/// ```compile_fail,E0004
/// use purrdf_hash::error::StructuredError;
/// pub enum Failure { Original, Added }
/// impl StructuredError for Failure {
///     purrdf_hash::error_visit! { Original => "original" {} }
/// }
/// ```
///
/// Nor can it add a named payload field without updating the visitor:
///
/// ```compile_fail
/// use purrdf_hash::error::StructuredError;
/// pub enum Failure { Refused { attempted: u128, limit: u128 } }
/// impl StructuredError for Failure {
///     purrdf_hash::error_visit! {
///         Refused { attempted } => "refused" { "attempted" => Unsigned(*attempted) }
///     }
/// }
/// ```
///
/// Structs must also account for every physical field:
///
/// ```compile_fail
/// use purrdf_hash::error::StructuredError;
/// pub struct Failure { attempted: u128, limit: u128 }
/// impl StructuredError for Failure {
///     purrdf_hash::error_visit! {
///         @record self { attempted } => "refused" {
///             "attempted" => Unsigned(self.attempted),
///         }
///     }
/// }
/// ```
///
/// Tuple owners likewise enumerate their complete physical shape:
///
/// ```compile_fail
/// use purrdf_hash::error::StructuredError;
/// pub struct Owner(u64, u64);
/// impl StructuredError for Owner {
///     purrdf_hash::error_visit! {
///         @tuple self (value) => "owner" { "value" => Unsigned(u128::from(*value)) }
///     }
/// }
/// ```
#[macro_export]
macro_rules! error_visit {
    (
        @tuple $this:ident ($($member:ident),* $(,)?) => $code:literal {
            $($field:literal => $kind:ident $(($value:expr))?),* $(,)?
        }
    ) => {
        fn visit<'error>(
            &'error $this,
            visitor: &mut dyn $crate::error::ErrorVisitor<'error>,
        ) {
            // Bind the complete shape, as for named records and enum variants.
            let Self($($member),*) = $this;
            visitor.variant($code);
            $($crate::error_visit!(@field visitor, $field, $kind $(($value))?);)*
        }
    };
    (
        @record $this:ident { $($member:ident),* $(,)? } => $code:literal {
            $($field:literal => $kind:ident $(($value:expr))?),* $(,)?
        }
    ) => {
        fn visit<'error>(
            &'error $this,
            visitor: &mut dyn $crate::error::ErrorVisitor<'error>,
        ) {
            // Check the complete physical shape, while semantic accessors can
            // decode private representations without duplicating their logic.
            let Self { $($member: _),* } = $this;
            visitor.variant($code);
            $($crate::error_visit!(@field visitor, $field, $kind $(($value))?);)*
        }
    };
    (
        $(
            $variant:ident
            $({ $($named:ident),* $(,)? })?
            $(($($tuple:ident),* $(,)?))?
            => $code:literal {
                $($field:literal => $kind:ident $(($value:expr))?),* $(,)?
            }
        ),* $(,)?
    ) => {
        fn visit<'error>(
            &'error self,
            visitor: &mut dyn $crate::error::ErrorVisitor<'error>,
        ) {
            match self {
                $(Self::$variant $({ $($named),* })? $(($($tuple),*))? => {
                    visitor.variant($code);
                    $($crate::error_visit!(@field visitor, $field, $kind $(($value))?);)*
                }),*
            }
        }
    };
    (@field $visitor:ident, $field:literal, List($value:expr)) => {
        $crate::error::visit_list($visitor, Some($field), $value)
    };
    (@field $visitor:ident, $field:literal, Value($value:expr)) => {
        $visitor.value(Some($field), $value)
    };
    (@field $visitor:ident, $field:literal, $kind:ident $(($value:expr))?) => {
        $visitor.value(Some($field), $crate::error::ErrorValue::$kind $(($value))?)
    };
}

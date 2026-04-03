//! JSON error types.
//!
//! # Error Types
//!
//! - [`JsonError`] — serialization errors (non-finite numbers, I/O errors)
//! - [`JsonParseError`] — parsing errors with detailed diagnostics
//!
//! # Example
//!
//! ```
//! use lifegraph_json::{parse_json, JsonParseError};
//!
//! let result = parse_json("{invalid}");
//! assert!(matches!(result, Err(JsonParseError::UnexpectedCharacter { .. })));
//! ```

use core::fmt;

/// Errors that can occur during JSON serialization.
///
/// This enum covers edge cases during serialization:
/// - [`NonFiniteNumber`] — `NaN` or `Infinity` values (not valid JSON)
/// - [`Io`] — I/O errors when writing to a [`Write`](std::io::Write) (requires `std` feature)
///
/// [`NonFiniteNumber`]: JsonError::NonFiniteNumber
/// [`Io`]: JsonError::Io
#[derive(Clone, Debug, PartialEq)]
pub enum JsonError {
    /// A non-finite float (`NaN` or `Infinity`) was encountered.
    /// JSON does not support these values.
    NonFiniteNumber,
    /// An I/O error occurred during serialization.
    Io,
}

/// Errors that can occur during JSON parsing.
///
/// Each variant includes context about where the error occurred,
/// making it easier to debug invalid JSON.
///
/// # Example
///
/// ```
/// use lifegraph_json::{parse_json, JsonParseError};
///
/// match parse_json("[1, 2,]") {
///     Err(JsonParseError::UnexpectedCharacter { index, found }) => {
///         println!("Unexpected '{}' at position {}", found, index);
///     }
///     other => panic!("Unexpected result: {:?}", other),
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonParseError {
    /// Input is not valid UTF-8.
    InvalidUtf8,
    /// Input ended unexpectedly while parsing a value.
    UnexpectedEnd,
    /// Trailing characters after a complete JSON value.
    ///
    /// The contained value is the byte offset of the first trailing character.
    UnexpectedTrailingCharacters(usize),
    /// An unexpected character was found.
    UnexpectedCharacter {
        /// Byte offset where the error occurred.
        index: usize,
        /// The character that was found.
        found: char,
    },
    /// An invalid literal was found (expected `true`, `false`, or `null`).
    InvalidLiteral {
        /// Byte offset where the error occurred.
        index: usize,
    },
    /// An invalid number was found.
    InvalidNumber {
        /// Byte offset where the error occurred.
        index: usize,
    },
    /// An invalid escape sequence was found in a string.
    InvalidEscape {
        /// Byte offset where the error occurred.
        index: usize,
    },
    /// An invalid Unicode escape sequence was found.
    InvalidUnicodeEscape {
        /// Byte offset where the error occurred.
        index: usize,
    },
    /// An invalid Unicode scalar value was found.
    InvalidUnicodeScalar {
        /// Byte offset where the error occurred.
        index: usize,
    },
    /// Expected a `:` after an object key.
    ExpectedColon {
        /// Byte offset where the error occurred.
        index: usize,
    },
    /// Expected `,` or end of array/object.
    ExpectedCommaOrEnd {
        /// Byte offset where the error occurred.
        index: usize,
        /// Whether we were parsing an "array" or "object".
        context: &'static str,
    },
    /// JSON nesting depth exceeds the maximum allowed (128)
    NestingTooDeep { depth: usize, max: usize },
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteNumber => {
                f.write_str("cannot serialize non-finite floating-point value")
            }
            Self::Io => f.write_str("i/o error while serializing JSON"),
        }
    }
}

impl fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 => f.write_str("input is not valid UTF-8"),
            Self::UnexpectedEnd => f.write_str("unexpected end of JSON input"),
            Self::UnexpectedTrailingCharacters(index) => {
                write!(f, "unexpected trailing characters at byte {index}")
            }
            Self::UnexpectedCharacter { index, found } => {
                write!(f, "unexpected character '{found}' at byte {index}")
            }
            Self::InvalidLiteral { index } => write!(f, "invalid literal at byte {index}"),
            Self::InvalidNumber { index } => write!(f, "invalid number at byte {index}"),
            Self::InvalidEscape { index } => write!(f, "invalid escape sequence at byte {index}"),
            Self::InvalidUnicodeEscape { index } => {
                write!(f, "invalid unicode escape at byte {index}")
            }
            Self::InvalidUnicodeScalar { index } => {
                write!(f, "invalid unicode scalar at byte {index}")
            }
            Self::ExpectedColon { index } => write!(f, "expected ':' at byte {index}"),
            Self::ExpectedCommaOrEnd { index, context } => {
                write!(f, "expected ',' or end of {context} at byte {index}")
            }
            Self::NestingTooDeep { depth, max } => {
                write!(f, "JSON nesting depth {} exceeds maximum {}", depth, max)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for JsonError {}
#[cfg(feature = "std")]
impl std::error::Error for JsonParseError {}

impl JsonError {
    #[cfg(feature = "std")]
    pub fn io(_error: std::io::Error) -> Self {
        Self::Io
    }
}

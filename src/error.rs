use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonError {
    NonFiniteNumber,
    Io,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonParseError {
    InvalidUtf8,
    UnexpectedEnd,
    UnexpectedTrailingCharacters(usize),
    UnexpectedCharacter {
        index: usize,
        found: char,
    },
    InvalidLiteral {
        index: usize,
    },
    InvalidNumber {
        index: usize,
    },
    InvalidEscape {
        index: usize,
    },
    InvalidUnicodeEscape {
        index: usize,
    },
    InvalidUnicodeScalar {
        index: usize,
    },
    ExpectedColon {
        index: usize,
    },
    ExpectedCommaOrEnd {
        index: usize,
        context: &'static str,
    },
    /// JSON nesting depth exceeds the maximum allowed (128)
    NestingTooDeep {
        depth: usize,
        max: usize,
    },
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

impl std::error::Error for JsonError {}
impl std::error::Error for JsonParseError {}

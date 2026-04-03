use crate::{JsonError, JsonParseError};

#[cfg(feature = "serde")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    message: String,
    category: Category,
    line: usize,
    column: usize,
    #[cfg(feature = "std")]
    io_error_kind: Option<std::io::ErrorKind>,
}

#[cfg(feature = "serde")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Io,
    Syntax,
    Data,
    Eof,
}

#[cfg(feature = "serde")]
#[allow(dead_code)]
pub struct NumberFromString;

#[cfg(feature = "serde")]
impl Error {
    pub fn io(error: std::io::Error) -> Self {
        Self {
            message: error.to_string(),
            category: Category::Io,
            line: 0,
            column: 0,
            #[cfg(feature = "std")]
            io_error_kind: Some(error.kind()),
        }
    }

    pub fn custom(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            category: Category::Data,
            line: 0,
            column: 0,
            #[cfg(feature = "std")]
            io_error_kind: None,
        }
    }

    pub fn classify(&self) -> Category {
        self.category
    }

    pub fn is_io(&self) -> bool {
        self.category == Category::Io
    }

    pub fn is_syntax(&self) -> bool {
        self.category == Category::Syntax
    }

    pub fn is_data(&self) -> bool {
        self.category == Category::Data
    }

    pub fn is_eof(&self) -> bool {
        self.category == Category::Eof
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    #[cfg(feature = "std")]
    pub fn io_error_kind(&self) -> Option<std::io::ErrorKind> {
        self.io_error_kind
    }
}

#[cfg(feature = "serde")]
impl std::error::Error for Error {}

#[cfg(feature = "serde")]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(all(feature = "serde", feature = "std"))]
impl From<Error> for std::io::Error {
    fn from(error: Error) -> Self {
        if error.is_io() {
            if let Some(kind) = error.io_error_kind() {
                return std::io::Error::new(kind, error.message);
            }
        }
        std::io::Error::new(std::io::ErrorKind::Other, error.message)
    }
}

#[cfg(feature = "serde")]
impl serde_crate::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::custom(msg.to_string())
    }
}

#[cfg(feature = "serde")]
impl serde_crate::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::custom(msg.to_string())
    }
}

#[cfg(feature = "serde")]
pub fn json_parse_error_to_serde(input: &str, error: JsonParseError) -> Error {
    let byte_index = match &error {
        JsonParseError::InvalidUtf8 | JsonParseError::UnexpectedEnd => input.len(),
        JsonParseError::UnexpectedTrailingCharacters(index)
        | JsonParseError::UnexpectedCharacter { index, .. }
        | JsonParseError::InvalidLiteral { index }
        | JsonParseError::InvalidNumber { index }
        | JsonParseError::InvalidEscape { index }
        | JsonParseError::InvalidUnicodeEscape { index }
        | JsonParseError::InvalidUnicodeScalar { index }
        | JsonParseError::ExpectedColon { index }
        | JsonParseError::ExpectedCommaOrEnd { index, .. } => *index,
        JsonParseError::NestingTooDeep { .. } => input.len(),
    };

    let (line, column) = line_column_for_byte(input, byte_index);
    let category = match &error {
        JsonParseError::UnexpectedEnd => Category::Eof,
        JsonParseError::InvalidUtf8
        | JsonParseError::UnexpectedTrailingCharacters(_)
        | JsonParseError::UnexpectedCharacter { .. }
        | JsonParseError::InvalidLiteral { .. }
        | JsonParseError::InvalidNumber { .. }
        | JsonParseError::InvalidEscape { .. }
        | JsonParseError::InvalidUnicodeEscape { .. }
        | JsonParseError::InvalidUnicodeScalar { .. }
        | JsonParseError::ExpectedColon { .. }
        | JsonParseError::ExpectedCommaOrEnd { .. }
        | JsonParseError::NestingTooDeep { .. } => Category::Syntax,
    };
    Error {
        message: error.to_string(),
        category,
        line,
        column,
        #[cfg(feature = "std")]
        io_error_kind: None,
    }
}

#[cfg(feature = "serde")]
pub fn json_error_to_serde(error: JsonError) -> Error {
    let category = match error {
        JsonError::Io => Category::Io,
        JsonError::NonFiniteNumber => Category::Data,
    };
    Error {
        message: error.to_string(),
        category,
        line: 0,
        column: 0,
        #[cfg(feature = "std")]
        io_error_kind: None,
    }
}

#[cfg(feature = "serde")]
#[allow(dead_code)]
pub fn io(error: std::io::Error) -> Error {
    Error::io(error)
}

#[cfg(feature = "serde")]
impl From<JsonError> for Error {
    fn from(error: JsonError) -> Self {
        json_error_to_serde(error)
    }
}

#[cfg(feature = "serde")]
fn line_column_for_byte(input: &str, byte_index: usize) -> (usize, usize) {
    let clamped = byte_index.min(input.len());
    let prefix = &input[..clamped];
    let line = prefix.bytes().filter(|&byte| byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .map(|tail| tail.chars().count() + 1)
        .unwrap_or(1);
    (line, column)
}

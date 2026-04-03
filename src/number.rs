use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonNumber {
    I64(i64),
    U64(u64),
    F64(f64),
}

impl JsonNumber {
    #[must_use]
    pub fn from_i128(value: i128) -> Option<Self> {
        if let Ok(value) = u64::try_from(value) {
            Some(Self::U64(value))
        } else if let Ok(value) = i64::try_from(value) {
            Some(Self::I64(value))
        } else {
            None
        }
    }

    pub fn from_u128(value: u128) -> Option<Self> {
        u64::try_from(value).ok().map(Self::U64)
    }

    #[must_use]
    pub fn is_i64(&self) -> bool {
        match self {
            Self::I64(_) => true,
            Self::U64(value) => i64::try_from(*value).is_ok(),
            Self::F64(_) => false,
        }
    }

    #[must_use]
    pub fn is_u64(&self) -> bool {
        matches!(self, Self::U64(_))
    }

    #[must_use]
    pub fn is_f64(&self) -> bool {
        matches!(self, Self::F64(_))
    }

    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(*value),
            Self::U64(value) => i64::try_from(*value).is_ok().then_some(*value as i64),
            Self::F64(_) => None,
        }
    }

    #[must_use]
    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Self::I64(value) => Some(i128::from(*value)),
            Self::U64(value) => Some(i128::from(*value)),
            Self::F64(_) => None,
        }
    }

    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::I64(value) => (*value >= 0).then_some(*value as u64),
            Self::U64(value) => Some(*value),
            Self::F64(_) => None,
        }
    }

    #[must_use]
    pub fn as_u128(&self) -> Option<u128> {
        match self {
            Self::I64(value) => (*value >= 0).then_some(*value as u128),
            Self::U64(value) => Some(u128::from(*value)),
            Self::F64(_) => None,
        }
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::I64(value) => Some(*value as f64),
            Self::U64(value) => Some(*value as f64),
            Self::F64(value) => Some(*value),
        }
    }

    #[must_use]
    pub fn from_f64(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self::F64(value))
    }
}

impl fmt::Display for JsonNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I64(value) => write!(f, "{value}"),
            Self::U64(value) => write!(f, "{value}"),
            Self::F64(value) => write!(f, "{value}"),
        }
    }
}

impl From<i64> for JsonNumber {
    fn from(value: i64) -> Self {
        if value >= 0 {
            Self::U64(value as u64)
        } else {
            Self::I64(value)
        }
    }
}

impl From<u64> for JsonNumber {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

use crate::pocketoption::error::PocketError;
use rust_decimal::Decimal;
use std::num::ParseFloatError;

#[derive(thiserror::Error, Debug)]
pub enum BinaryOptionsError {
    #[error("Pocket Options Error: {0}")]
    PocketOptions(#[from] PocketError),

    /// Couldn't parse f64 to Decimal
    #[error("Couldn't parse f64 to Decimal: {0}")]
    ParseFloat(#[from] ParseFloatError),

    /// Couldn't parse Decimal to f64
    #[error("Couldn't parse Decimal to f64: {0}")]
    ParseDecimal(String),

    /// General error with a message
    #[error("General error: {0}")]
    General(String),
}

pub type BinaryOptionsResult<T> = Result<T, BinaryOptionsError>;

impl From<Decimal> for BinaryOptionsError {
    fn from(decimal: Decimal) -> Self {
        BinaryOptionsError::ParseDecimal(format!("Failed to convert Decimal {} to f64", decimal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pocketoption::error::PocketError;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_pocket_options_error_conversion() {
        let pocket_err = PocketError::General("test".into());
        let err: BinaryOptionsError = pocket_err.into();
        match err {
            BinaryOptionsError::PocketOptions(pe) => {
                assert_eq!(format!("{}", pe), "General error: test")
            }
            _ => panic!("Expected PocketOptions error variant"),
        }
    }

    #[test]
    fn test_parse_float_error_conversion() {
        let float_err = "abc".parse::<f64>().unwrap_err();
        let err: BinaryOptionsError = float_err.into();
        assert!(err.to_string().contains("Couldn't parse f64 to Decimal"));
    }

    #[test]
    fn test_decimal_to_error_conversion() {
        let decimal = Decimal::from_str("1.23").unwrap();
        let err: BinaryOptionsError = decimal.into();
        assert_eq!(
            err.to_string(),
            "Couldn't parse Decimal to f64: Failed to convert Decimal 1.23 to f64"
        );
    }

    #[test]
    fn test_general_error() {
        let err = BinaryOptionsError::General("critical failure".into());
        assert_eq!(err.to_string(), "General error: critical failure");
    }

    #[test]
    fn test_debug_implementation() {
        let err = BinaryOptionsError::General("debug test".into());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("General(\"debug test\")"));
    }
}

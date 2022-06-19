use std::fmt::{Display, Formatter};
use std::error::Error as StdError;

#[derive(Debug)]
pub enum ErrorType {
    NoneError,
    SystemError,
    Unauthorized,
    Forbidden,
    BadRequest,
}

#[derive(Debug)]
pub struct Error {
    pub error_type: ErrorType,
    pub error: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.error_type {
            ErrorType::NoneError => write!(f, "NoneError : {}", self.error),
            ErrorType::SystemError => write!(f, "SystemError : {}", self.error),
            ErrorType::Unauthorized => write!(f, "Unauthorized  : {}", self.error),
            ErrorType::Forbidden => write!(f, "Forbidden  : {}", self.error),
            ErrorType::BadRequest => write!(f, "Bad Request  : {}", self.error),
        }
    }
}

impl StdError for Error {}
impl Error {
    pub fn new(error_type: ErrorType, error: &str) -> Self {
        Self {
            error_type,
            error: error.to_string(),
        }
    }
    pub fn none_error(error: &str) -> Self {
        Self::new(ErrorType::NoneError, error)
    }
    pub fn system_error(error: &str) -> Self {
        Self::new(ErrorType::SystemError, error)
    }
}

impl From<Box<dyn StdError>> for Error {
    fn from(e: Box<dyn StdError>) -> Self {
        Self {
            error_type: ErrorType::SystemError,
            error: format!("{}", e),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::system_error(e.to_string().as_str())
    }
}
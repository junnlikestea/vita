use std::error::Error;
use std::fmt::{self, Formatter};

pub type Result<T> = std::result::Result<T, VitaError>;

#[derive(Debug)]
pub enum VitaError {
    SourceError(String),
    AuthError(String),
    UnsetKeys(Vec<String>),
    ReqwestError(reqwest::Error),
    JoinError(tokio::task::JoinError),
    IoError(std::io::Error),
    Msg(String),
    ParseError,
    CrobatError,
    EmptyResults,
}

impl fmt::Display for VitaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            VitaError::SourceError(s) => write!(f, "couldn't fetch data from {}", s),
            VitaError::AuthError(s) => {
                write!(
                    f,
                    "error authenticating to {} or may have hit rate limits",
                    s
                )
            }
            VitaError::UnsetKeys(v) => write!(f, "error reading environment variables {:?}", v),
            VitaError::EmptyResults => write!(f, "returned no results"),
            VitaError::CrobatError => {
                write!(f, "got error when trying to pull results from crobat")
            }
            VitaError::ParseError => write!(f, "got error trying to parse cli args"),
            VitaError::Msg(s) => write!(f, "got error {}", s),
            VitaError::ReqwestError(ref err) => err.fmt(f),
            VitaError::JoinError(ref err) => err.fmt(f),
            VitaError::IoError(ref err) => err.fmt(f),
        }
    }
}

impl Error for VitaError {}

impl From<String> for VitaError {
    fn from(err: String) -> Self {
        VitaError::Msg(err)
    }
}

impl From<reqwest::Error> for VitaError {
    fn from(err: reqwest::Error) -> Self {
        VitaError::ReqwestError(err)
    }
}

impl From<tokio::task::JoinError> for VitaError {
    fn from(err: tokio::task::JoinError) -> Self {
        VitaError::JoinError(err)
    }
}

impl From<std::io::Error> for VitaError {
    fn from(err: std::io::Error) -> Self {
        VitaError::IoError(err)
    }
}

impl From<std::num::ParseIntError> for VitaError {
    fn from(_: std::num::ParseIntError) -> Self {
        VitaError::ParseError
    }
}

// Monkey patch until I add custom error type to Crobat
impl From<Box<dyn Error + Sync + std::marker::Send>> for VitaError {
    fn from(_: Box<dyn Error + Sync + std::marker::Send>) -> Self {
        VitaError::CrobatError
    }
}

use std::error;
use std::fmt;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Clone, Debug)]
pub enum ErrorKind {
    SourceError {
        source: String,
        host: Arc<String>,
    },
    AuthError {
        source: String,
    },
    KeyError {
        source: String,
        env_variables: Vec<String>,
    },
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub(crate) fn source_error(source: &str, host: Arc<String>) -> Box<Error> {
        let source = source.to_string();
        Box::new(Error {
            kind: ErrorKind::SourceError { source, host },
        })
    }

    pub(crate) fn auth_error(source: &str) -> Box<Error> {
        let source = source.to_string();
        Box::new(Error {
            kind: ErrorKind::AuthError { source },
        })
    }

    pub(crate) fn key_error(source: &str, envs: &[&str]) -> Box<Error> {
        let source = source.to_string();
        let env_variables = envs.iter().map(|s| s.to_string()).collect();
        Box::new(Error {
            kind: ErrorKind::KeyError {
                source,
                env_variables,
            },
        })
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::SourceError { .. } => "there was an error retrieving data from the source",
            ErrorKind::AuthError { .. } => {
                "there was an error authenticating or you may have reached rate-limits."
            }
            ErrorKind::KeyError { .. } => "error reading environment variables",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::SourceError { source, host } => {
                write!(f, "{} couldn't find any results for: {}", source, host,)
            }
            ErrorKind::AuthError { source } => write!(
                f,
                "Couldn't authenticate or have hit rate-limits for {}",
                source
            ),
            ErrorKind::KeyError {
                source,
                env_variables,
            } => write!(
                f,
                "Couldn't read {:?} for {}. Check if you have them set.",
                env_variables, source
            ),
        }
    }
}

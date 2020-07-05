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
    SourceError { source: String, host: Arc<String> },
    FacebookAuthError,
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

    pub(crate) fn fb_auth_error() -> Box<Error> {
        Box::new(Error {
            kind: ErrorKind::FacebookAuthError,
        })
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::SourceError { .. } => "there was an error retrieving data from the source",
            ErrorKind::FacebookAuthError => "there was an error authenticating to Facebook.",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::SourceError { source, host } => {
                write!(f, "{} couldn't find any results for: {}", source, host,)
            }

            ErrorKind::FacebookAuthError => write!(
                f,
                "Failed to authenticate to the Facebook API\
            using credentials supplied."
            ),
        }
    }
}

use std::{error::Error, fmt::Display};

macro_rules! generic_error_wrapper {
    ($error: ty) => {
        impl From<$error> for ServerError {
            fn from(e: $error) -> Self {
                Self {
                    kind: ErrorKind::Other(e.to_string()),
                }
            }
        }
    };
}

#[derive(Debug)]
#[allow(dead_code)]
enum ErrorKind {
    Server(String),
    Other(String),
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ServerError {
    kind: ErrorKind,
}

impl ServerError {
    pub(crate) fn new(reason: String) -> Self {
        Self {
            kind: ErrorKind::Server(reason),
        }
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

generic_error_wrapper!(std::io::Error);

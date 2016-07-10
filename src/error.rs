use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};
use std::io::{self, ErrorKind};
use std::sync::PoisonError;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Poison(String),
    FileType,
    Version(u32, u32),
    UnsupportedFeature(String),
    FileFormat(String),
    Internal(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(err: PoisonError<T>) -> Error {
        Error::Poison(format!("{}", err))
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
            Error::FileType => "Not a qcow2 file",
            Error::Version(_, _) => "Unsupported version",
            Error::UnsupportedFeature(_) => "Unsupported feature",
            Error::FileFormat(_) => "Malformed qcow2 file",
            Error::Internal(_) => "Internal error",
            Error::Poison(ref s) => s,
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::Version(cur, sup) => {
                write!(f, "Unsupported version {}, only {} is allowed", cur, sup)
            }
            Error::UnsupportedFeature(ref feat) => write!(f, "Unsupported feature: {}", feat),
            Error::FileFormat(ref err) => write!(f, "Malformed qcow2 file: {}", err),
            Error::Internal(ref err) => write!(f, "Internal error: {}", err),
            _ => f.write_str(self.description()),
        }
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> io::Error {
        io::Error::new(ErrorKind::Other, err)
    }
}

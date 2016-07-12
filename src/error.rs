use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};
use std::io::{self, ErrorKind};
use std::sync::PoisonError;

/// The error type for Qcow2 operations.
#[derive(Debug)]
pub enum Error {
    /// The underlying source of a Qcow2 reported an I/O error.
    Io(io::Error),

    /// A synchronization primitive reported being poisoned.
    /// See [std::sync::PoisonError](https://doc.rust-lang.org/std/sync/struct.PoisonError.html).
    Poison(String),

    /// The file being opened is not a qcow2 file.
    FileType,

    /// The file being opened has an unsupported version.
    Version(u32),

    /// A feature unsupported by this library was detected.
    UnsupportedFeature(String),

    /// An error was detected in a qcow2 file. The file may be corrupt.
    FileFormat(String),

    /// An internal error was detected, there must be a bug in this library.
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
            Error::Version(_) => "Unsupported version",
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
            Error::Version(found) => write!(f, "Unsupported version {}", found),
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

use std;
use std::error::Error as StdError;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    FileType,
    Version(u32, u32),
    UnsupportedFeature(String),
    FileFormat(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
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
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::Version(cur, sup) => {
                write!(f, "Unsupported version {}, only {} is allowed", cur, sup)
            }
            Error::UnsupportedFeature(ref feat) => write!(f, "Unsupported feature {}", feat),
            Error::FileFormat(ref err) => write!(f, "Malformed qcow2 file: {}", err),
            _ => f.write_str(self.description()),
        }
    }
}

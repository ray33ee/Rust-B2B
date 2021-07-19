
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum ErrorKind {
    IOError(std::io::Error),
    BincodeError(bincode::Error),
    InvalidBitmapID,
    InvalidB2BSignature,
    BadPaddingSize,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Box<dyn std::error::Error>,
}

impl Error {
    pub fn new<E: Into<Box<dyn std::error::Error>>>(kind: ErrorKind, error: E) -> Self {
        Self {
            kind,
            error: error.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::IOError(err),
            error: "".into(),
        }
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Self {
            kind: ErrorKind::BincodeError(err),
            error: "".into(),
        }
    }
}
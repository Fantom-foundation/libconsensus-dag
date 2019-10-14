use failure::Error as FailureError;
use libconsensus::errors::Error as BaseError;
use libhash::errors::Error as LibhashError;
use std::option::NoneError;

pub type Result<T> = std::result::Result<T, FailureError>;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Libconsensus Base Error: {:?}", 0)]
    Base(BaseError),
    #[fail(display = "Bincode Error: {:?}", 0)]
    Bincode(bincode::Error),
    #[fail(display = "Sled Error: {:?}", 0)]
    Sled(sled::Error),
    #[fail(display = "Io Error: {:?}", 0)]
    Io(std::io::Error),
    #[fail(display = "SerdeJson Error: {:?}", 0)]
    SerdeJson(serde_json::error::Error),
    #[fail(display = "None an error!")]
    NoneError,
    #[fail(display = "Libhash Error: {:?}", 0)]
    LibHash(LibhashError),
}

impl From<LibhashError> for Error {
    #[inline]
    fn from(e: LibhashError) -> Error {
        Error::LibHash(e)
    }
}

impl From<NoneError> for Error {
    #[inline]
    fn from(_none_error: NoneError) -> Error {
        Error::NoneError
    }
}

impl From<sled::Error> for Error {
    #[inline]
    fn from(sled_error: sled::Error) -> Error {
        Error::Sled(sled_error)
    }
}

impl From<bincode::Error> for Error {
    #[inline]
    fn from(bincode_error: bincode::Error) -> Error {
        Error::Bincode(bincode_error)
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(io_error: std::io::Error) -> Error {
        Error::Io(io_error)
    }
}

impl From<serde_json::error::Error> for Error {
    #[inline]
    fn from(json_error: serde_json::error::Error) -> Error {
        Error::SerdeJson(json_error)
    }
}

impl From<BaseError> for Error {
    #[inline]
    fn from(b: BaseError) -> Error {
        Error::Base(b)
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match *self {
            Error::NoneError => Error::NoneError == *other,
            Error::SerdeJson(ref _l) => {
                // FIXME: serde_json::error::Error has no PartialEq trait implemented
                false
            }
            Error::Io(ref _l) => {
                // FIXME: std::io::Error has no PartialEq trait implemented
                false
            }
            Error::Sled(ref l) => {
                if let Error::Sled(ref r) = *other {
                    l == r
                } else {
                    false
                }
            }
            Error::Bincode(ref _l) => {
                // FIXME: add comparison for bincode::Error
                false
            }
            Error::Base(ref _l) => {
                // FIXME: implement PartialEq trait for libconsensus::errors::Error
                false
            }
            Error::LibHash(ref _l) => {
                // FIXME: implement PartialEq trait for libhash::errors::Error
                false
            }
        }
    }
}

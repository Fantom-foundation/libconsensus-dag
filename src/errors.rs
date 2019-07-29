use fantom_common_rs::errors::Error as BaseError;
//use bincode::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Base(BaseError),
    AtMaxVecCapacity,
    Bincode(bincode::Error),
    Sled(sled::Error),
    Io(std::io::Error),
    SerdeJson(serde_json::error::Error),
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

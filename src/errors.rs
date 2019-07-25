use fantom_common_rs::errors::Error as BaseError;
//use bincode::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Base(BaseError),
    Bincode(bincode::Error),
    Sled(sled::Error),
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

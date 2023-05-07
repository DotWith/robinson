use thiserror::Error;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf16Error),
    #[error(transparent)]
    Float(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    Int(#[from] std::num::ParseIntError),
    #[error(transparent)]
    Image(#[from] image::ImageError),

    // css
    #[error("Invalid Unit {0}")]
    InvalidUnit(String),
    #[error("Invalid char {0}")]
    InvalidChar(char),
}


use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Http error: {0}")]
    Http(#[from] http::Error),
    #[error("Unknown error")]
    Other
}

pub type Result<T> = std::result::Result<T, Error>;

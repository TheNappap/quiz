
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Http error: {0}")]
    Http(#[from] http::Error),
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("Unknown error")]
    Other
}

pub type QuizResult<T> = Result<T, Error>;

pub trait IntoQuizResult<T> {
    fn into_result(self) -> QuizResult<T>;
}

impl<T> IntoQuizResult<T> for http::Result<T> {
    fn into_result(self) -> QuizResult<T> {
        Ok(self?)
    }
}
use poem::{Body, Response};
use poem::error::ResponseError;
use poem::http::StatusCode;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid session ID")]
    InvalidSession,
    #[error("Invalid song URL")]
    InvalidSong,
    #[error("Song already exists in queue")]
    SongExists,
}

impl ResponseError for Error {
    fn status(&self) -> StatusCode {
        match self {
            Error::InvalidSession => StatusCode::NOT_FOUND,
            Error::InvalidSong => StatusCode::BAD_REQUEST,
            Error::SongExists => StatusCode::CONFLICT,
        }
    }

    fn as_response(&self) -> Response {
        let status = self.status();
        let body = Body::from_json(json!({
            "status": status.as_u16(),
            "message": self.to_string(),
        })).unwrap();
        Response::builder().status(status).body(body)
    }
}

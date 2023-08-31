use poem::{Body, Response};
use poem::error::ResponseError;
use poem::http::StatusCode;
use serde_json::json;
use crate::session::models::{Packet, PacketKind};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Invalid session ID")]
    InvalidSession,
    #[error("Invalid song URL")]
    InvalidSong,
    #[error("Song already in queue")]
    AlreadyInQueue,
}

impl ApiError {
    pub fn as_serialized_packet(&self) -> String {
        Packet::serialized_str(PacketKind::Error, self.to_string())
    }
}

impl ResponseError for ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::InvalidSession => StatusCode::NOT_FOUND,
            ApiError::InvalidSong => StatusCode::BAD_REQUEST,
            ApiError::AlreadyInQueue => StatusCode::CONFLICT,
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

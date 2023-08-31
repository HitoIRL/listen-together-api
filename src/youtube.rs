use serde::{Deserialize, Serialize};
use youtube_dl::YoutubeDl;
use crate::errors::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDetails {
    pub title: String,
    pub thumbnail: String,
    pub audio: String,
}

pub async fn get_song_details(song_url: String) -> Result<SongDetails, ApiError> {
    let output = YoutubeDl::new(&song_url)
        .youtube_dl_path("./yt-dlp.exe")
        .socket_timeout("15")
        .run_async()
        .await
        .unwrap();
    let video = output.into_single_video().unwrap();

    let title = video.title.unwrap();
    let thumbnail = video.thumbnail.unwrap();
    let audio = video
        .formats
        .unwrap()
        .into_iter()
        .filter(|f| f.format_id == Some("251".to_string()))
        .next()
        .unwrap();

    Ok(SongDetails {
        title,
        thumbnail,
        audio: audio.url.unwrap(),
    })
}

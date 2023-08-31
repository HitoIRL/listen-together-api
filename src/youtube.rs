use serde::{Deserialize, Serialize};
use youtube_dl::YoutubeDl;
use crate::errors::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDetails {
    pub id: String,
    pub title: String,
    pub thumbnail: String,
    pub audio: String,
}

pub async fn get_song_details(song_url: String) -> Result<SongDetails, ApiError> {
    let output = YoutubeDl::new(&song_url)
        .youtube_dl_path("./yt-dlp.exe")
        .socket_timeout("15")
        .run_async()
        .await;

    let output = match output {
        Ok(output) => output,
        Err(_) => return Err(ApiError::InvalidSong), // we can generalize the error for now since we don't care about the specific error
    };

    let video = output.into_single_video().unwrap();

    let id = video.id;
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
        id,
        title,
        thumbnail,
        audio: audio.url.unwrap(),
    })
}

use std::env;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::errors::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDetails {
    pub title: String,
    thumbnail: String,
}

pub async fn get_song_details(song_url: String) -> Result<SongDetails, Error> {
    let id_regex = Regex::new(r"(?:youtube(?:-nocookie)?\.com\/(?:[^\/\n\s]+\/\S+\/|(?:v|e(?:mbed)?)\/|\S*?[?&]vi?=)|youtu\.be\/)([a-zA-Z0-9_-]{11})").unwrap();    
    let id = id_regex.captures(&song_url).unwrap().get(1).unwrap().as_str();

    let api_key = env::var("YOUTUBE_API").unwrap();
    let api_url = format!("https://www.googleapis.com/youtube/v3/videos?part=snippet&id={id}&key={api_key}");

    let resp = reqwest::get(&api_url).await.unwrap();
    let resp = resp.json::<serde_json::Value>().await.unwrap();

    match resp["items"].get(0) {
        Some(video) => {
            let title = video["snippet"]["title"].as_str().unwrap().to_string();
            let thumbnail = video["snippet"]["thumbnails"]["default"]["url"].as_str().unwrap().to_string();

            Ok(SongDetails {
                title,
                thumbnail,
            })
        }
        None => Err(Error::InvalidSong)
    }
}

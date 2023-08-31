use serde::{Deserialize, Serialize};
use crate::youtube;

// TODO: implement own serialization
#[derive(Debug)]
pub struct SessionData {
    pub current_song: u32,
    pub songs: Vec<youtube::SongDetails>,
    pub users: u32,
}

impl SessionData {
    pub fn new() -> Self {
        Self {
            current_song: 0,
            songs: vec![],
            users: 0,
        }
    }

    pub fn as_vec(&self) -> Vec<(String, String)> {
        vec![
            ("current_song".to_string(), self.current_song.to_string()),
            ("songs".to_string(), serde_json::to_string(&self.songs).unwrap()),
            ("users".to_string(), self.users.to_string()),
        ]
    }

    pub fn from_vec(v: Vec<(String, String)>) -> Self {
        let mut current_song = 0;
        let mut songs = vec![];
        let mut users = 0;

        for (key, value) in v {
            match key.as_str() {
                "current_song" => current_song = value.parse().unwrap(),
                "songs" => songs = serde_json::from_str(&value).unwrap(),
                "users" => users = value.parse().unwrap(),
                _ => (),
            }
        }

        Self { current_song, songs, users }
    }
}

#[derive(Debug, Clone)]
pub struct SinkData {
    pub session: String,
    pub body: String,
}

impl SinkData {
    pub fn new(session: &str, body: String) -> Self {
        Self {
            session: session.to_string(),
            body,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PacketKind {
    // sent from client only
    AddSong,
    RemoveSong,
    ForwardSkip,
    BackwardSkip,

    // sent from server only
    Error,
    SetSongs,
    SetCurrentSong,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
    pub kind: PacketKind,
    pub data: String,
}

impl Packet {
    pub fn serialized<T: Serialize>(kind: PacketKind, data: T) -> String {
        serde_json::to_string(&Packet {
            kind,
            data: serde_json::to_string(&data).unwrap(),
        }).unwrap()
    }

    pub fn serialized_str(kind: PacketKind, data: String) -> String {
        serde_json::to_string(&Packet {
            kind,
            data,
        }).unwrap()
    }
}

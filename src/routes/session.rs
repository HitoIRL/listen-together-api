use futures_util::{StreamExt, SinkExt};
use log::debug;
use poem::{IntoEndpoint, Route, web::{Path, Data, websocket::{WebSocket, Message}}, handler, post, get, IntoResponse, EndpointExt};
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::sync::broadcast::{Sender, channel};
use uuid::Uuid;

use crate::{errors::Error, youtube};

#[derive(Debug)]
struct SessionData {
    songs: Vec<youtube::SongDetails>,
    users: u32,
}

impl SessionData {
    fn new() -> Self {
        Self {
            songs: vec![],
            users: 0,
        }
    }

    fn as_vec(&self) -> Vec<(String, String)> {
        vec![
            ("songs".to_string(), serde_json::to_string(&self.songs).unwrap()),
            ("users".to_string(), self.users.to_string()),
        ]
    }

    fn from_vec(v: Vec<(String, String)>) -> Self {
        let mut songs = vec![];
        let mut users = 0;

        for (key, value) in v {
            match key.as_str() {
                "songs" => {
                    songs = serde_json::from_str(&value).unwrap();
                }
                "users" => {
                    users = value.parse().unwrap();
                }
                _ => (),
            }
        }

        Self { songs, users }
    }
}

#[handler]
async fn create_session(redis: Data<&ConnectionManager>) -> String {
    let mut redis = redis.clone();

    let id = Uuid::new_v4().to_string();
    let key = format!("session:{id}");

    let data = SessionData::new();
    let data = data.as_vec();
    let _: () = redis.hset_multiple(key, &data).await.unwrap();

    json!({ "id": id }).to_string()
}

#[derive(Debug, Clone)]
struct SinkData {
    session: String,
    body: String,
}

impl SinkData {
    fn new(session: &str, body: String) -> Self {
        Self {
            session: session.to_string(),
            body,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum PacketKind {
    AddSong,
    SetSongs,
}

#[derive(Debug, Serialize, Deserialize)]
struct Packet {
    kind: PacketKind,
    data: String,
}

impl Packet {
    fn serialized<T: Serialize>(kind: PacketKind, data: T) -> String {
        serde_json::to_string(&Packet {
            kind,
            data: serde_json::to_string(&data).unwrap(),
        }).unwrap()
    }
}

async fn get_session_data(id: &str, redis: &mut ConnectionManager) -> Result<SessionData, Error> {
    let key = format!("session:{id}");
    let vec: Vec<(String, String)> = redis.hgetall(&key).await.unwrap();
    if vec.is_empty() {
        return Err(Error::InvalidSession);
    }
    Ok(SessionData::from_vec(vec))
}

async fn get_songs(id: &str, redis: &mut ConnectionManager) -> Vec<youtube::SongDetails> {
    let key = format!("session:{id}");
    let songs: String = redis.hget(&key, "songs".to_string()).await.unwrap();
    serde_json::from_str(&songs).unwrap()
}

async fn receive_packet(
    packet: Packet,
    redis: &mut ConnectionManager,
    sender: &Sender<SinkData>,
    session: &str,
) -> bool {
    let key = format!("session:{session}");

    match packet.kind {
        PacketKind::AddSong => {
            let details = youtube::get_song_details(packet.data).await.unwrap();
            let mut songs = get_songs(&session, redis).await;

            if (songs.iter().find(|&s| s.title == details.title)).is_some() {
                return false;
            }

            songs.push(details);
            let songs_serialized = serde_json::to_string(&songs).unwrap();
            let _: () = redis.hset(&key, "songs".to_string(), &songs_serialized).await.unwrap();

            let send_packet = Packet {
                kind: PacketKind::SetSongs,
                data: songs_serialized,
            };

            if sender.send(SinkData::new(session, serde_json::to_string(&send_packet).unwrap())).is_err() {
                return true;
            }
        }
        _ => ()
    }

    false
}

#[handler]
async fn join_session(
    redis: Data<&ConnectionManager>,
    Path(session): Path<String>,
    ws: WebSocket,
    sender: Data<&Sender<SinkData>>,
) -> Result<impl IntoResponse, Error> {
    let mut redis = redis.clone();

    let key = format!("session:{session}");

    // check if session exists
    let mut data = get_session_data(&session, &mut redis).await?;
    data.users += 1;
    let _: () = redis.hset_multiple(&key, &data.as_vec()).await.unwrap();

    // handle websocket
    let sender = sender.clone();
    let mut receiver = sender.subscribe();
    Ok(ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();

        debug!("Client connected to session {session}");
        
        // send current songs
        let packet = Packet::serialized(PacketKind::SetSongs, data.songs);
        if sink.send(Message::Text(packet)).await.is_err() {
            return;
        }

        let recv_session = session.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                match msg {
                    Message::Text(text) => {
                        let packet = serde_json::from_str::<Packet>(&text).unwrap();
                        if receive_packet(packet, &mut redis, &sender, &recv_session).await {
                            break;
                        }
                    }
                    Message::Close(_) => {
                        let mut data = get_session_data(&recv_session, &mut redis).await.unwrap();

                        data.users -= 1;
                        debug!("Client disconnected from session {recv_session} | {} users left", data.users);

                        if data.users > 0 {
                            let _: () = redis.hset_multiple(&key, &data.as_vec()).await.unwrap();
                        } else {
                            let _: () = redis.del(&key).await.unwrap();
                            debug!("Session {recv_session} deleted");
                        }
                    }
                    _ => (),
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(data) = receiver.recv().await {
                if data.session != session {
                    continue;
                }

                if sink.send(Message::Text(data.body)).await.is_err() {
                    break;
                }
            }
        });
    }))
}

pub fn register_routes() -> impl IntoEndpoint {
    Route::new()
        .at("/", post(create_session))
        .at("/:id", get(join_session.data(channel::<SinkData>(32).0)))
}

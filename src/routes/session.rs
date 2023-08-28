use futures_util::{StreamExt, SinkExt};
use log::debug;
use poem::{IntoEndpoint, Route, web::{Path, Data, websocket::{WebSocket, Message}}, handler, post, get, IntoResponse, EndpointExt};
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::sync::broadcast::{Sender, channel};
use uuid::Uuid;

use crate::{errors::Error, youtube};

#[handler]
async fn create_session(redis: Data<&ConnectionManager>) -> String {
    let mut redis = redis.clone();

    let id = Uuid::new_v4().to_string();
    let key = format!("session:{id}");
    let _: () = redis.hset_multiple(key, &[
        ("songs", "[]")
    ]).await.unwrap();

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

#[handler]
async fn join_session(
    redis: Data<&ConnectionManager>,
    Path(session): Path<String>,
    ws: WebSocket,
    sender: Data<&Sender<SinkData>>,
) -> Result<impl IntoResponse, Error> {
    let mut redis = redis.clone();

    // check if session exists
    let key = format!("session:{session}");
    let exists: bool = redis.exists(&key).await.unwrap();
    if !exists {
        return Err(Error::InvalidSession);
    }

    // handle websocket
    let sender = sender.clone();
    let mut receiver = sender.subscribe();
    Ok(ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();

        debug!("Client connected to session {session}");
        
        // send current songs
        let songs: String = redis.hget(&key, "songs".to_string()).await.unwrap();
        let send_packet = Packet {
            kind: PacketKind::SetSongs,
            data: songs,
        };
        if sink.send(Message::Text(serde_json::to_string(&send_packet).unwrap())).await.is_err() {
            return;
        }

        let recv_session = session.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                match msg {
                    Message::Text(text) => {
                        debug!("Received message: {text}");

                        let packet = serde_json::from_str::<Packet>(&text).unwrap();
                        debug!("Packet: {:?}", packet);

                        match packet.kind {
                            PacketKind::AddSong => {
                                let details = youtube::get_song_details(packet.data).await.unwrap();
                                let songs: String = redis.hget(&key, "songs".to_string()).await.unwrap();
                                let mut songs: Vec<youtube::SongDetails> = serde_json::from_str(&songs).unwrap();
                                songs.push(details);
                                let songs_serialized = serde_json::to_string(&songs).unwrap();
                                let _: () = redis.hset(&key, "songs".to_string(), &songs_serialized).await.unwrap();

                                let send_packet = Packet {
                                    kind: PacketKind::SetSongs,
                                    data: songs_serialized,
                                };

                                if sender.send(SinkData::new(&recv_session, serde_json::to_string(&send_packet).unwrap())).is_err() {
                                    break;
                                }
                            }
                            _ => ()
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

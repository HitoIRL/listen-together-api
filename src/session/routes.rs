use futures_util::{SinkExt, StreamExt};
use log::debug;
use poem::{EndpointExt, get, handler, IntoResponse, post, Route};
use poem::web::{Data, Path};
use poem::web::websocket::{Message, WebSocket};
use redis::AsyncCommands;
use serde_json::json;
use tokio::sync::broadcast::{channel, Sender};
use uuid::Uuid;
use crate::database::Database;
use crate::errors::ApiError;
use crate::session::functions::{get_session_data, handle_packet};
use crate::session::models::{Packet, PacketKind, SessionData, SinkData};

#[handler]
async fn create_session(mut redis: Database) -> String {
    let id = Uuid::new_v4().to_string();
    let key = format!("session:{id}");

    let data = SessionData::new();
    let data = data.as_vec();
    let _: () = redis.0.hset_multiple(key, &data).await.unwrap();

    json!({ "id": id }).to_string()
}

#[handler]
async fn join_session(
    mut redis: Database,
    Path(session): Path<String>,
    ws: WebSocket,
    sender: Data<&Sender<SinkData>>,
) -> Result<impl IntoResponse, ApiError> {
    let key = format!("session:{session}");

    // check if session exists
    let mut data = get_session_data(&session, &mut redis).await?;
    data.users += 1;
    let _: () = redis.0.hset_multiple(&key, &data.as_vec()).await.unwrap();

    // handle websocket
    let sender = sender.clone();
    let mut receiver = sender.subscribe();
    Ok(ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        debug!("Client connected to session {session}");

        // send current session data
        let packet = Packet::serialized(PacketKind::SetSongs, data.songs);
        if sink.send(Message::Text(packet)).await.is_err() {
            return;
        }

        let packet = Packet::serialized(PacketKind::SetCurrentSong, data.current_song);
        if sink.send(Message::Text(packet)).await.is_err() {
            return;
        }

        let recv_session = session.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                match msg {
                    Message::Text(text) => {
                        let packet = serde_json::from_str::<Packet>(&text).unwrap();

                        if let Some(packet) = handle_packet(packet, &mut redis, &recv_session).await {
                            if sender.send(SinkData::new(&recv_session, packet)).is_err() {
                                break
                            }
                        }
                    }
                    Message::Close(_) => {
                        let mut data = get_session_data(&recv_session, &mut redis).await.unwrap();

                        data.users -= 1;
                        debug!("Client disconnected from session {recv_session} | {} users left", data.users);

                        if data.users > 0 {
                            let _: () = redis.0.hset_multiple(&key, &data.as_vec()).await.unwrap();
                        } else {
                            let _: () = redis.0.del(&key).await.unwrap();
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

pub fn register_routes() -> Route {
    Route::new()
        .at("/", post(create_session))
        .at("/:id", get(join_session.data(channel::<SinkData>(32).0)))
}

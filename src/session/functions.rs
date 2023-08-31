use redis::AsyncCommands;
use crate::database::Database;
use crate::errors::ApiError;
use crate::session::models::{Packet, PacketKind, SessionData};
use crate::youtube;

pub async fn handle_packet(
    packet: Packet,
    redis: &mut Database,
    session: &str,
) -> Option<String> {
    let key = format!("session:{session}");

    match packet.kind {
        PacketKind::AddSong => {
            let details = match youtube::get_song_details(packet.data).await {
                Ok(details) => details,
                Err(_) => return Some(ApiError::InvalidSong.as_serialized_packet()),
            };

            let mut songs = get_songs(&session, redis).await;

            if (songs.iter().find(|&s| s.id == details.id)).is_some() {
                return Some(ApiError::AlreadyInQueue.as_serialized_packet());
            }

            songs.push(details);
            let songs_serialized = serde_json::to_string(&songs).unwrap();
            let _: () = redis.0.hset(&key, "songs".to_string(), &songs_serialized).await.unwrap();

            let packet = Packet::serialized(PacketKind::SetSongs, songs);
            Some(packet)
        }
        PacketKind::RemoveSong => {
            let songs = get_songs(&session, redis).await;
            let songs = songs.into_iter().filter(|s| s.id != packet.data).collect::<Vec<_>>();
            let songs_serialized = serde_json::to_string(&songs).unwrap();
            let _: () = redis.0.hset(&key, "songs".to_string(), &songs_serialized).await.unwrap();

            let packet = Packet::serialized(PacketKind::SetSongs, songs);
            Some(packet)
        }
        PacketKind::ForwardSkip => {
            let current_song = get_current_song(&session, redis).await + 1;
            let songs = get_songs(&session, redis).await;
            if songs.len() > current_song as usize {
                let _: () = redis.0.hset(&key, "current_song".to_string(), current_song).await.unwrap();

                let packet = Packet::serialized(PacketKind::SetCurrentSong, current_song);
                return Some(packet);
            }

            None
        }
        PacketKind::BackwardSkip => {
            let mut current_song = get_current_song(&session, redis).await;
            if current_song > 0 {
                current_song -= 1;
            } else {
                return None;
            }

            let _: () = redis.0.hset(&key, "current_song".to_string(), current_song).await.unwrap();
            let packet = Packet::serialized(PacketKind::SetCurrentSong, current_song);
            Some(packet)
        }
        _ => None
    }
}

// database wrappers
pub async fn get_session_data(id: &str, redis: &mut Database) -> Result<SessionData, ApiError> {
    let key = format!("session:{id}");
    let vec: Vec<(String, String)> = redis.0.hgetall(&key).await.unwrap();
    if vec.is_empty() {
        return Err(ApiError::InvalidSession);
    }
    Ok(SessionData::from_vec(vec))
}

pub async fn get_songs(id: &str, redis: &mut Database) -> Vec<youtube::SongDetails> {
    let key = format!("session:{id}");
    let songs: String = redis.0.hget(&key, "songs".to_string()).await.unwrap();
    serde_json::from_str(&songs).unwrap()
}

pub async fn get_current_song(id: &str, redis: &mut Database) -> u32 {
    let key = format!("session:{id}");
    let current_song: String = redis.0.hget(&key, "current_song".to_string()).await.unwrap();
    current_song.parse().unwrap()
}

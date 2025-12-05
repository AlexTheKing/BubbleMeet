use std::{collections::HashMap, error::Error, fmt::Display, net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::{
        ConnectInfo, Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::{any, get},
};
use axum_extra::{TypedHeader, headers};
use futures::StreamExt;
use log::{info, warn};
use tokio::sync::{Mutex, RwLock};

pub mod messages;
pub mod participant;
pub mod replayer;
pub mod room;
pub mod sfu;
pub mod webrtc_factory;

use crate::{messages::SignalingMessage, room::Room, sfu::SelectiveForwardingUnit};

#[derive(Clone)]
struct AppState {
    rooms: Arc<RwLock<HashMap<String, Arc<Room>>>>,
    sfu: Arc<SelectiveForwardingUnit>,
}

#[derive(Debug)]
struct NonEmptyRoomError {
    room_id: String,
}

impl Error for NonEmptyRoomError {}

impl Display for NonEmptyRoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Room with id={} is not empty", self.room_id)
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            rooms: Default::default(),
            sfu: Arc::new(SelectiveForwardingUnit::new()),
        }
    }

    async fn get_room(&self, room_id: &str) -> Arc<Room> {
        let mut rooms = self.rooms.write().await;
        if !rooms.contains_key(room_id) {
            rooms.insert(room_id.to_string(), Default::default());
        }
        rooms.get(room_id).cloned().unwrap()
    }

    async fn remove_room(&self, room_id: &str) -> Result<(), NonEmptyRoomError> {
        let mut rooms = self.rooms.write().await;
        if rooms.is_empty() {
            rooms.remove(room_id);
            Ok(())
        } else {
            Err(NonEmptyRoomError {
                room_id: room_id.to_string(),
            })
        }
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!(
        "Received connect from ip={} to room_id={}",
        addr.ip(),
        room_id
    );
    ws.on_upgrade(move |socket| handle_socket(socket, state, room_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, room_id: String) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let room = state.get_room(&room_id).await;
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            if let Ok(message) = serde_json::from_str::<SignalingMessage>(&text) {
                let _ = state
                    .sfu
                    .consume_signaling_message(message, sender.clone(), room.clone())
                    .await;
            } else {
                warn!("Received unknown message from participant: {}", text);
            }
        }
    }
    // TODO: Cannot remove room as there may be stale participants
    if state.remove_room(&room_id).await.is_ok() {
        info!("Removed room={room_id}");
    } else {
        warn!(
            "Failed to remove room={room_id}, it is not empty, {} participants still connected",
            room.len().await
        );
    }
}

async fn health_handler() -> impl IntoResponse {
    "OK"
}

#[tokio::main]
async fn main() {
    let config_path =
        std::env::var("LOG4RS_CONFIG_PATH").unwrap_or("resources/log4rs.yml".to_string());
    log4rs::init_file(config_path, Default::default()).unwrap();
    let app = Router::new()
        .route("/api/v1/rooms/:room_id", any(websocket_handler))
        .route("/api/v1/health", get(health_handler))
        .with_state(AppState::new());

    let address = std::env::var("SIGNALING_SERVER_HOST").unwrap_or("0.0.0.0".to_string());
    let port = std::env::var("SIGNALING_SERVER_PORT").unwrap_or("8080".to_string());
    info!("Listening on {}:{}", address, port);
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", address, port))
        .await
        .unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

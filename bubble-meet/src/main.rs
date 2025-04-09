use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{any, get},
    Router,
};
use axum_extra::{headers, TypedHeader};
use futures::{SinkExt, StreamExt};
use log::{info, warn};
use models::messages::{PingSignalingMessage, SignalingMessage};
use tokio::sync::Mutex;

pub mod models;
pub mod sfu;
pub mod webrtc_api_factory;

use crate::models::room::Room;
use crate::sfu::SelectiveForwardingUnit;

#[derive(Clone, Default)]
struct AppState {
    rooms: Arc<Mutex<HashMap<String, Arc<Mutex<Room>>>>>,
}

struct RoomIsNotEmptyError();

impl AppState {
    async fn get_room(&self, room_id: &str) -> Arc<Mutex<Room>> {
        let mut rooms_lock = self.rooms.lock().await;
        if !rooms_lock.contains_key(room_id) {
            rooms_lock.insert(room_id.to_string(), Default::default());
        }
        rooms_lock.get(room_id).unwrap().clone()
    }

    async fn try_remove_room(&self, room_id: &str) -> Result<(), RoomIsNotEmptyError> {
        let mut rooms_lock = self.rooms.lock().await;
        if rooms_lock.is_empty() {
            rooms_lock.remove(room_id);
            Ok(())
        } else {
            Err(RoomIsNotEmptyError())
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
    {
        let sender = sender.clone();
        tokio::spawn(async move {
            let mut ping_result = Result::Ok(());
            while ping_result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(3));
                tokio::pin!(timeout);
                tokio::select! {
                    _ = timeout.as_mut() => {
                        ping_result = sender.lock().await.send(Message::Text(
                            serde_json::to_string(&SignalingMessage::Ping(PingSignalingMessage::new())).expect("Failed to serialize ping message")
                        )).await;
                    }
                };
            }
        });
    }
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            if let Ok(message) = serde_json::from_str::<SignalingMessage>(&text) {
                SelectiveForwardingUnit::process_signaling_message(message, sender.clone(), &room)
                    .await;
            } else {
                warn!("Received unknown message from user: {}", text);
            }
        }
    }
    if state.try_remove_room(&room_id).await.is_ok() {
        info!("Removed room={room_id}");
    } else {
        warn!(
            "Failed to remove room={room_id}, it is not empty, {} users still connected",
            room.lock().await.users.len()
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
        .with_state(AppState::default());

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

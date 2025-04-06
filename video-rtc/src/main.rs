use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::any,
    Router,
};
use axum_extra::{headers, TypedHeader};
use futures::StreamExt;
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
    println!(
        "Received connect from ip={} to room_id={room_id}",
        addr.ip()
    );
    ws.on_upgrade(move |socket| handle_socket(socket, state, room_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, room_id: String) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let room = state.get_room(&room_id).await;
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            SelectiveForwardingUnit::process_signaling_message(
                serde_json::from_str(&text).expect("Cannot parse websocket message"),
                sender.clone(),
                &room,
            )
            .await
        }
    }
    match state.try_remove_room(&room_id).await {
        Ok(_) => println!("Removed room={room_id}"),
        Err(_) => println!("Websocket connection closed, but room={room_id} is not empty"),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/rooms/:room_id", any(websocket_handler))
        .with_state(AppState::default());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

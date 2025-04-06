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

#[derive(Clone)]
struct AppState {
    rooms: Arc<Mutex<HashMap<String, Arc<Mutex<Room>>>>>,
    sfu: Arc<Mutex<SelectiveForwardingUnit>>,
}

impl AppState {
    fn new() -> AppState {
        AppState {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            sfu: Arc::new(Mutex::new(SelectiveForwardingUnit::new())),
        }
    }

    async fn get_room(&self, room_id: String) -> Arc<Mutex<Room>> {
        let mut rooms = self.rooms.lock().await;
        if !rooms.contains_key(&room_id) {
            rooms.insert(room_id.clone(), Arc::new(Mutex::new(Room::new())));
        }
        rooms.get(&room_id).unwrap().clone()
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
        "Received connect from ip={} to room_id={} ",
        addr.ip(),
        room_id,
    );
    let room = state.get_room(room_id).await;
    let sfu = state.sfu.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, addr, room, sfu))
}

async fn handle_socket(
    socket: WebSocket,
    _: SocketAddr,
    room: Arc<Mutex<Room>>,
    sfu: Arc<Mutex<SelectiveForwardingUnit>>,
) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    while let Some(Ok(ws_message)) = receiver.next().await {
        if let Message::Text(text) = ws_message {
            sfu.lock()
                .await
                .process_signaling_message(
                    serde_json::from_str(&text).expect("Cannot parse websocket message"),
                    sender.clone(),
                    &room,
                )
                .await
        }
    }
    println!("Websocket connection closed!");
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/rooms/:room_id", any(websocket_handler))
        .with_state(AppState::new());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

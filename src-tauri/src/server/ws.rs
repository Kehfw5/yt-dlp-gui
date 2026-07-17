//! WebSocket 事件推送

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::ServerState;
use yt_dlp_shared::download::DownloadEvent;

/// 创建事件回调（用于下载进程 → broadcast channel）
pub fn make_emit_callback(
    event_tx: Arc<std::sync::Mutex<std::collections::HashMap<String, broadcast::Sender<String>>>>,
    task_id: String,
) -> yt_dlp_shared::download::EventCallback {
    Arc::new(move |event: DownloadEvent| {
        let msg = serde_json::to_string(&event).unwrap_or_default();
        let mut channels = event_tx.lock().unwrap();
        if let Some(tx) = channels.get(&task_id) {
            let _ = tx.send(msg);
        } else {
            let (tx, _) = broadcast::channel::<String>(256);
            let _ = tx.send(msg);
            channels.insert(task_id.clone(), tx);
        }
    })
}

/// WebSocket 升级处理
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<ServerState>) {
    let mut rx: Option<broadcast::Receiver<String>> = None;

    loop {
        tokio::select! {
            // 从 broadcast channel 读取 → 推送到 WebSocket
            result = async {
                if let Some(ref mut rx) = rx {
                    rx.recv().await.ok()
                } else {
                    std::future::pending::<Option<String>>().await
                }
            } => {
                if let Some(msg) = result {
                    let _ = socket.send(Message::Text(msg.into())).await;
                }
            }

            // 从客户端接收订阅消息
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            if parsed["type"].as_str() == Some("subscribe")
                                && parsed["taskId"].as_str().is_some()
                            {
                                let task_id = parsed["taskId"].as_str().unwrap();
                                let mut channels = state.event_tx.lock().unwrap();
                                let tx = if let Some(tx) = channels.get(task_id) {
                                    tx.clone()
                                } else {
                                    let (tx, _) = broadcast::channel(256);
                                    channels.insert(task_id.to_string(), tx.clone());
                                    tx
                                };
                                rx = Some(tx.subscribe());
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

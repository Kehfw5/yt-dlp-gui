//! 内嵌 HTTP + WebSocket 服务（LAN 访问）
//!
//! 桌面端开启「允许局域网连接」后，在后台启动 axum 服务，
//! 移动端设备可通过局域网 IP:端口 连接进行远程下载。

mod ws;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::AppHandle;
use tower_http::cors::{Any, CorsLayer};

use crate::commands::DownloadState;
use crate::utils;
use yt_dlp_shared::common;
use yt_dlp_shared::types::DownloadParams;

/// 服务端共享状态（从 Tauri AppHandle 构建）
pub struct ServerState {
    pub processes: Arc<std::sync::Mutex<std::collections::HashMap<String, yt_dlp_shared::types::DownloadProcessInfo>>>,
    pub app_data_dir: std::path::PathBuf,
    pub event_tx: Arc<std::sync::Mutex<std::collections::HashMap<String, tokio::sync::broadcast::Sender<String>>>>,
}

/// 从 Tauri 状态构建 HTTP 路由
pub fn build_router(app: &AppHandle, state: &DownloadState) -> Router {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let server_state = Arc::new(ServerState {
        processes: state.processes.clone(),
        app_data_dir,
        event_tx: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // 下载
        .route("/api/start_download", post(start_download))
        .route("/api/pause_download", post(pause_download))
        .route("/api/resume_download", post(resume_download))
        .route("/api/cancel_download", post(cancel_download))
        .route("/api/check_files_exist", post(check_files_exist))
        .route("/api/delete_file", post(delete_file))
        // 视频信息
        .route("/api/fetch_video_info", post(fetch_video_info))
        .route("/api/save_cookie_text", post(save_cookie_text))
        .route("/api/get_platform", get(get_platform))
        .route("/api/ping", get(ping))
        // WebSocket
        .route("/ws", get(ws::ws_handler))
        .layer(cors)
        .with_state(server_state)
}

/// 启动 HTTP 服务器（在后台 tokio task 中运行）
pub fn start_server(app: AppHandle, port: u16) -> Result<(), String> {
    let state = app.state::<DownloadState>().clone();
    let router = build_router(&app, &state);

    tokio::spawn(async move {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to bind LAN server on port {}: {}", port, e);
                return;
            }
        };

        tracing::info!("LAN server listening on http://{}", addr);

        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("LAN server error: {}", e);
        }
    });

    Ok(())
}

// ========== 请求体参数提取 ==========

fn extract_params(body: &Value) -> Value {
    body.get("params").cloned().unwrap_or_else(|| body.clone())
}

// ========== 路由处理函数 ==========

async fn start_download(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let params_value = extract_params(&body);
    let params: DownloadParams = serde_json::from_value(params_value)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid params: {}", e)))?;

    let ytdlp_path = yt_dlp_shared::utils::get_ytdlp_path(&state.app_data_dir);
    let task_id = params.id.clone();
    let app_data_dir = state.app_data_dir.clone();
    let processes = state.processes.clone();
    let event_tx = state.event_tx.clone();

    let emit = ws::make_emit_callback(event_tx, task_id.clone());

    let pid = yt_dlp_shared::download::start_download_process(
        &ytdlp_path,
        &app_data_dir,
        emit,
        processes,
        params,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(json!({ "id": task_id, "pid": pid })))
}

async fn pause_download(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let id = body["id"].as_str().ok_or((StatusCode::BAD_REQUEST, "missing id".into()))?;
    let processes = state.processes.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let info = processes.get(id).ok_or((StatusCode::NOT_FOUND, "err_task_not_found".into()))?;
    yt_dlp_shared::process::suspend_process(info.pid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

async fn resume_download(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let id = body["id"].as_str().ok_or((StatusCode::BAD_REQUEST, "missing id".into()))?;
    let processes = state.processes.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let info = processes.get(id).ok_or((StatusCode::NOT_FOUND, "err_task_not_found".into()))?;
    yt_dlp_shared::process::resume_process(info.pid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

async fn cancel_download(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let id = body["id"].as_str().ok_or((StatusCode::BAD_REQUEST, "missing id".into()))?;
    let delete_files = body["deleteFiles"].as_bool().unwrap_or(false);

    let (pid, files) = {
        let mut processes = state.processes.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let info = processes.get_mut(id).ok_or((StatusCode::NOT_FOUND, "err_task_not_found".into()))?;
        info.cancelled = true;
        (info.pid, info.output_files.clone())
    };

    yt_dlp_shared::process::kill_process(pid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if delete_files {
        for file in &files {
            let _ = std::fs::remove_file(file);
            let _ = std::fs::remove_file(format!("{}.part", file));
        }
    }

    Ok(Json(json!({ "ok": true })))
}

async fn check_files_exist(
    Json(body): Json<Value>,
) -> Result<Json<Vec<bool>>, (StatusCode, String)> {
    let paths: Vec<String> = serde_json::from_value(body["paths"].clone())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let exists: Vec<bool> = paths.iter().map(|p| std::path::Path::new(p).exists()).collect();
    Ok(Json(exists))
}

async fn delete_file(
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let path = body["path"].as_str().ok_or((StatusCode::BAD_REQUEST, "missing path".into()))?;
    let p = std::path::Path::new(path);
    if p.exists() {
        std::fs::remove_file(p).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    Ok(Json(json!({ "ok": true })))
}

async fn fetch_video_info(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let url = body["url"].as_str().ok_or((StatusCode::BAD_REQUEST, "missing url".into()))?;

    let result = common::run_ytdlp_json(
        &yt_dlp_shared::utils::get_ytdlp_path(&state.app_data_dir),
        &state.app_data_dir,
        url,
        &[],
        body["cookieFile"].as_str(),
        body["cookieBrowser"].as_str(),
        body["proxy"].as_str(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(result))
}

async fn save_cookie_text(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let text = body["text"].as_str().ok_or((StatusCode::BAD_REQUEST, "missing text".into()))?;
    let cookie_path = utils::get_cookie_path_shared(&state.app_data_dir);
    tokio::fs::write(&cookie_path, text.as_bytes())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("err_save_cookie:{}", e)))?;
    Ok(Json(json!({ "path": cookie_path.to_string_lossy() })))
}

fn get_platform() -> Json<Value> {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    Json(json!(platform))
}

fn ping() -> Json<Value> {
    Json(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

/// 从 shared utils 获取 cookie 路径（不依赖 AppHandle）
fn get_cookie_path_shared(app_data_dir: &std::path::Path) -> std::path::PathBuf {
    yt_dlp_shared::utils::get_cookie_path(app_data_dir)
}

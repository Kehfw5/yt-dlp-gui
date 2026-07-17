//! 命令模块共享辅助函数（薄封装 → yt_dlp_shared）

use serde_json::Value;
use tauri::AppHandle;

use crate::utils;

#[cfg(target_os = "windows")]
use super::CREATE_NO_WINDOW;

// Re-export from shared
pub use yt_dlp_shared::common::{
    append_cookie_proxy_args, build_http_client, extract_ytdlp_error, validate_path_within,
};

/// 运行 yt-dlp -J 并解析 JSON 输出（Tauri 版本：从 AppHandle 提取路径）
pub async fn run_ytdlp_json(
    app: &AppHandle,
    url: &str,
    extra_args: &[&str],
    cookie_file: Option<&str>,
    cookie_browser: Option<&str>,
    proxy: Option<&str>,
) -> Result<Value, String> {
    let ytdlp_path = utils::get_ytdlp_path(app)?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("err_app_data_dir:{}", e))?;

    yt_dlp_shared::common::run_ytdlp_json(
        &ytdlp_path,
        &app_data,
        url,
        extra_args,
        cookie_file,
        cookie_browser,
        proxy,
    )
    .await
}

//! Tauri 端 utils 薄封装层
//!
//! 所有函数接受 `&AppHandle`，提取 `app_data_dir` 后委托给 `yt_dlp_shared::utils`。

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub use yt_dlp_shared::utils::{
    build_youtube_extractor_args, get_deno_download_url, get_ytdlp_download_url,
    set_binary_path_resolve_mode, set_youtube_extractor_args,
};

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("err_app_data_dir:{}", e))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("err_create_dir:{}", e))?;
    Ok(dir)
}

pub fn get_managed_ytdlp_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    Ok(yt_dlp_shared::utils::get_managed_ytdlp_path(&dir))
}

pub fn get_ytdlp_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    Ok(yt_dlp_shared::utils::get_ytdlp_path(&dir))
}

pub fn get_managed_deno_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    Ok(yt_dlp_shared::utils::get_managed_deno_path(&dir))
}

pub fn get_deno_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    Ok(yt_dlp_shared::utils::get_deno_path(&dir))
}

pub fn get_cookie_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    Ok(yt_dlp_shared::utils::get_cookie_path(&dir))
}

pub fn get_plugin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    Ok(yt_dlp_shared::utils::get_plugin_dir(&dir))
}

pub fn build_plugin_args(app: &AppHandle) -> Vec<String> {
    if let Ok(dir) = app_data_dir(app) {
        yt_dlp_shared::utils::build_plugin_args(&dir)
    } else {
        vec![]
    }
}

pub fn build_js_runtime_args(app: &AppHandle) -> Vec<String> {
    if let Ok(dir) = app_data_dir(app) {
        yt_dlp_shared::utils::build_js_runtime_args(&dir)
    } else {
        vec![]
    }
}

//! 下载任务控制：启动、暂停、继续、取消、文件检查
//!
//! Tauri 命令薄封装 → 核心逻辑在 yt_dlp_shared::download

use crate::process;
use crate::utils;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

use super::common::append_cookie_proxy_args;
use super::{DownloadParams, DownloadProcessInfo, DownloadState};

#[cfg(target_os = "windows")]
use super::CREATE_NO_WINDOW;

// Re-export shared helpers for use by other command modules
pub use yt_dlp_shared::download::{
    build_download_args, format_duration, parse_destination, read_filepath_from_file,
    resolve_output_file,
};

// ========== 事件回调适配 ==========

/// 创建 Tauri 事件回调：将共享 DownloadEvent 转换为 Tauri app.emit()
fn tauri_emit_callback(app: AppHandle) -> yt_dlp_shared::download::EventCallback {
    Arc::new(move |event| match event {
        yt_dlp_shared::download::DownloadEvent::Progress {
            id,
            percent,
            speed,
            eta,
            downloaded,
            total,
        } => {
            let _ = app.emit(
                "download-progress",
                serde_json::json!({ "id": id, "percent": percent, "speed": speed, "eta": eta, "downloaded": downloaded, "total": total }),
            );
        }
        yt_dlp_shared::download::DownloadEvent::Log { id, line } => {
            let _ = app.emit(
                "download-log",
                serde_json::json!({ "id": id, "line": line }),
            );
        }
        yt_dlp_shared::download::DownloadEvent::Complete { id, output_file } => {
            let _ = app.emit(
                "download-complete",
                serde_json::json!({ "id": id, "outputFile": output_file }),
            );
        }
        yt_dlp_shared::download::DownloadEvent::Error { id, error } => {
            let _ = app.emit(
                "download-error",
                serde_json::json!({ "id": id, "error": error }),
            );
        }
    })
}

// ========== 下载命令 ==========

/// 启动下载任务
#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: tauri::State<'_, DownloadState>,
    params: DownloadParams,
) -> Result<(), String> {
    let ytdlp_path = utils::get_ytdlp_path(&app)?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("err_app_data_dir:{}", e))?;

    let emit = tauri_emit_callback(app);

    yt_dlp_shared::download::start_download_process(
        &ytdlp_path,
        &app_data,
        emit,
        state.processes.clone(),
        params,
    )
    .await?;

    Ok(())
}

// ========== 下载控制命令 ==========

/// 暂停下载任务（挂起子进程）
#[tauri::command]
pub async fn pause_download(
    state: tauri::State<'_, DownloadState>,
    id: String,
) -> Result<(), String> {
    let processes = state.processes.lock().map_err(|e| e.to_string())?;
    let info = processes.get(&id).ok_or("err_task_not_found")?;
    process::suspend_process(info.pid)
}

/// 继续下载任务（恢复子进程）
#[tauri::command]
pub async fn resume_download(
    state: tauri::State<'_, DownloadState>,
    id: String,
) -> Result<(), String> {
    let processes = state.processes.lock().map_err(|e| e.to_string())?;
    let info = processes.get(&id).ok_or("err_task_not_found")?;
    process::resume_process(info.pid)
}

/// 取消下载任务并可选删除已下载文件
#[tauri::command]
pub async fn cancel_download(
    state: tauri::State<'_, DownloadState>,
    id: String,
    delete_files: bool,
) -> Result<(), String> {
    let (pid, files) = {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        let info = processes.get_mut(&id).ok_or("err_task_not_found")?;
        info.cancelled = true;
        (info.pid, info.output_files.clone())
    };

    process::kill_process(pid)?;

    if delete_files {
        for file in &files {
            let _ = std::fs::remove_file(file);
            let _ = std::fs::remove_file(format!("{}.part", file));
        }
    }

    Ok(())
}

// ========== 文件检查 ==========

/// 批量检查文件是否存在
#[tauri::command]
pub fn check_files_exist(paths: Vec<String>) -> Vec<bool> {
    paths
        .iter()
        .map(|p| std::path::Path::new(p).exists())
        .collect()
}

/// 删除指定文件
#[tauri::command]
pub fn delete_file(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        std::fs::remove_file(p).map_err(|e| format!("err_delete_file:{}", e))?;
    }
    Ok(())
}

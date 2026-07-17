//! Tauri 命令模块
//!
//! 按功能域拆分:
//! - common: 共享辅助函数（薄封装 → yt_dlp_shared::common）
//! - setup: 平台信息、yt-dlp/Deno 安装管理
//! - video: 视频信息获取、Cookie 管理
//! - download: 下载任务控制（薄封装 → yt_dlp_shared::download）
//! - tools: 工具箱命令（封面、字幕、弹幕）

pub(crate) mod common;
mod download;
mod setup;
mod tools;
mod video;

// Re-export shared types so Tauri commands can reference them
pub use yt_dlp_shared::types::{
    DenoStatus, DownloadParams, DownloadProcessInfo, DownloadState, YtdlpStatus,
};
pub use yt_dlp_shared::types::*;

// 使用 glob 导出：Tauri generate_handler! 宏需要访问 __cmd__ 隐藏项
pub use download::*;
pub use setup::*;
pub use tools::*;
pub use video::*;

/// Windows: 隐藏控制台窗口标志
#[cfg(target_os = "windows")]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x08000000;

//! 路径辅助、下载地址和插件/JS 运行时参数构建
//!
//! 与 Tauri 版本不同：此模块不依赖 `AppHandle`，所有函数接受 `&Path`。

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

// ========== 二进制路径解析模式 ==========

#[derive(Clone, Copy)]
enum BinaryPathResolveMode {
    SystemPreferred,
    AppOnly,
}

static BINARY_PATH_RESOLVE_MODE: OnceLock<RwLock<BinaryPathResolveMode>> = OnceLock::new();

fn get_path_resolve_mode() -> BinaryPathResolveMode {
    let lock = BINARY_PATH_RESOLVE_MODE
        .get_or_init(|| RwLock::new(BinaryPathResolveMode::AppOnly));
    lock.read()
        .map(|v| *v)
        .unwrap_or(BinaryPathResolveMode::AppOnly)
}

pub fn set_binary_path_resolve_mode(mode: &str) -> Result<(), String> {
    let parsed = match mode {
        "system-preferred" => BinaryPathResolveMode::SystemPreferred,
        "app-only" => BinaryPathResolveMode::AppOnly,
        _ => return Err(format!("err_invalid_path_mode:{}", mode)),
    };
    let lock = BINARY_PATH_RESOLVE_MODE
        .get_or_init(|| RwLock::new(BinaryPathResolveMode::AppOnly));
    let mut guard = lock.write().map_err(|e| format!("err_set_path_mode:{}", e))?;
    *guard = parsed;
    Ok(())
}

// ========== YouTube extractor 参数 ==========

#[derive(Default, Clone)]
struct YoutubeExtractorArgs {
    po_token: String,
    visitor_data: String,
}

static YOUTUBE_EXTRACTOR_ARGS: OnceLock<RwLock<YoutubeExtractorArgs>> = OnceLock::new();

fn youtube_args_lock() -> &'static RwLock<YoutubeExtractorArgs> {
    YOUTUBE_EXTRACTOR_ARGS.get_or_init(|| RwLock::new(YoutubeExtractorArgs::default()))
}

pub fn set_youtube_extractor_args(po_token: &str, visitor_data: &str) -> Result<(), String> {
    let mut guard = youtube_args_lock()
        .write()
        .map_err(|e| format!("err_set_youtube_args:{}", e))?;
    guard.po_token = po_token.trim().to_string();
    guard.visitor_data = visitor_data.trim().to_string();
    Ok(())
}

pub fn build_youtube_extractor_args() -> Vec<String> {
    let guard = match youtube_args_lock().read() {
        Ok(g) => g,
        Err(_) => return vec![],
    };
    let mut parts: Vec<String> = Vec::new();
    if !guard.po_token.is_empty() {
        parts.push(format!("po_token={}", guard.po_token));
    }
    if !guard.visitor_data.is_empty() {
        parts.push(format!("visitor_data={}", guard.visitor_data));
    }
    if parts.is_empty() {
        return vec![];
    }
    vec![
        "--extractor-args".to_string(),
        format!("youtube:{}", parts.join(";")),
    ]
}

// ========== 可执行文件路径解析 ==========

fn find_system_executable(name: &str) -> Option<PathBuf> {
    which::which(name).ok().filter(|path| path.exists())
}

fn resolve_executable_path(managed_path: PathBuf, system_name: &str) -> PathBuf {
    match get_path_resolve_mode() {
        BinaryPathResolveMode::AppOnly => managed_path,
        BinaryPathResolveMode::SystemPreferred => {
            find_system_executable(system_name).unwrap_or(managed_path)
        }
    }
}

/// 获取应用管理的 yt-dlp 路径
pub fn get_managed_ytdlp_path(app_data_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        app_data_dir.join("yt-dlp.exe")
    } else {
        app_data_dir.join("yt-dlp")
    }
}

/// 获取 yt-dlp 可执行文件路径
pub fn get_ytdlp_path(app_data_dir: &Path) -> PathBuf {
    let managed_path = get_managed_ytdlp_path(app_data_dir);
    resolve_executable_path(managed_path, "yt-dlp")
}

/// 获取应用管理的 Deno 路径
pub fn get_managed_deno_path(app_data_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        app_data_dir.join("deno.exe")
    } else {
        app_data_dir.join("deno")
    }
}

/// 获取 Deno 可执行文件路径
pub fn get_deno_path(app_data_dir: &Path) -> PathBuf {
    let managed_path = get_managed_deno_path(app_data_dir);
    resolve_executable_path(managed_path, "deno")
}

/// 获取 Cookie 文件路径
pub fn get_cookie_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cookies.txt")
}

/// 获取 yt-dlp 下载地址（根据平台）
pub fn get_ytdlp_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
    }
}

/// 获取 yt-dlp 插件目录路径
pub fn get_plugin_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("yt-dlp-plugins")
}

/// 如果插件目录存在，返回 --plugin-dirs 参数
pub fn build_plugin_args(app_data_dir: &Path) -> Vec<String> {
    let plugin_dir = get_plugin_dir(app_data_dir);
    if plugin_dir.exists() {
        return vec![
            "--plugin-dirs".to_string(),
            plugin_dir.to_string_lossy().to_string(),
        ];
    }
    vec![]
}

/// 如果 Deno 已安装，返回 JS 运行时参数
pub fn build_js_runtime_args(app_data_dir: &Path) -> Vec<String> {
    let deno_path = get_deno_path(app_data_dir);
    if deno_path.exists() {
        return vec![
            "--js-runtimes".to_string(),
            format!("deno:{}", deno_path.to_string_lossy()),
        ];
    }
    vec![]
}

/// 获取 Deno 下载地址（根据平台和架构）
pub fn get_deno_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip"
        } else {
            "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-apple-darwin.zip"
        }
    } else {
        "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip"
    }
}

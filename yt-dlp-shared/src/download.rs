//! 下载核心逻辑：参数构建、输出处理、文件路径解析
//!
//! 不含 Tauri 或 HTTP 框架依赖。事件通过回调 / channel 发送，
//! 由调用方（Tauri app.emit 或 服务端 WebSocket）适配。

use crate::common::append_cookie_proxy_args;
use crate::parser;
use crate::types::{DownloadParams, DownloadProcessInfo};
use crate::utils;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ========== 下载事件（发送给调用方） ==========

/// 下载事件类型，与 Tauri 事件名称一一对应
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DownloadEvent {
    Progress {
        id: String,
        percent: f64,
        speed: String,
        eta: String,
        downloaded: String,
        total: String,
    },
    Log {
        id: String,
        line: String,
    },
    Complete {
        id: String,
        output_file: String,
    },
    Error {
        id: String,
        error: String,
    },
}

/// 事件回调：Tauri 端用 `app.emit()`，服务端用 broadcast channel
pub type EventCallback = Arc<dyn Fn(DownloadEvent) + Send + Sync>;

// ========== 辅助函数 ==========

/// 将秒数格式化为 HH:MM:SS
pub fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

/// 从 yt-dlp 输出行中解析目标文件路径（备选方案）
pub fn parse_destination(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("[download] Destination: ") {
        return Some(rest.trim().to_string());
    }
    if trimmed.starts_with("[download] ") && trimmed.ends_with("has already been downloaded") {
        let inner = trimmed
            .strip_prefix("[download] ")?
            .strip_suffix("has already been downloaded")?
            .trim();
        if !inner.is_empty() {
            return Some(inner.to_string());
        }
    }
    if trimmed.contains("[Merger] Merging formats into") {
        let start = trimmed.find('"')? + 1;
        let end = trimmed.rfind('"')?;
        if start < end {
            return Some(trimmed[start..end].to_string());
        }
    }
    None
}

/// 从临时文件中读取 yt-dlp --print-to-file 写出的最终文件路径
pub fn read_filepath_from_file(filepath_file: &str) -> Option<String> {
    let content = std::fs::read_to_string(filepath_file).ok()?;
    let last_line = content.trim().lines().last()?.trim().to_string();
    if last_line.is_empty() {
        None
    } else {
        Some(last_line)
    }
}

/// 解析最终输出文件路径
pub fn resolve_output_file(
    processes: &Arc<Mutex<HashMap<String, DownloadProcessInfo>>>,
    task_id: &str,
) -> (String, bool) {
    processes
        .lock()
        .ok()
        .map(|map| {
            map.get(task_id)
                .map(|info| {
                    let mut file = String::new();

                    if let Some(ref fp_file) = info.filepath_file {
                        if let Some(path) = read_filepath_from_file(fp_file) {
                            file = path;
                        }
                        let _ = std::fs::remove_file(fp_file);
                    }

                    if file.is_empty() {
                        file = info.output_files.last().cloned().unwrap_or_default();
                        if !file.is_empty() && !std::path::Path::new(&file).is_absolute() {
                            file = std::path::PathBuf::from(&info.download_dir)
                                .join(&file)
                                .to_string_lossy()
                                .to_string();
                        }
                    }

                    let has = !info.output_files.is_empty() || !file.is_empty();
                    (file, has)
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

// ========== 输出处理 ==========

/// 处理 yt-dlp 的一行输出：解析进度并调用回调发送事件
pub fn process_output_line(
    emit: &EventCallback,
    task_id: &str,
    processes: &Arc<Mutex<HashMap<String, DownloadProcessInfo>>>,
    line: &str,
) {
    // 解析 --progress-template 输出的 JSON 进度
    if let Some(info) = parser::parse_progress_json(line) {
        emit(DownloadEvent::Progress {
            id: task_id.to_string(),
            percent: info.percent,
            speed: info.speed,
            eta: info.eta,
            downloaded: info.downloaded,
            total: info.total,
        });
        return;
    }

    // 解析 ffmpeg 输出中的 time= 字段
    if line.contains("time=") && line.contains("frame=") {
        if let Some(current_secs) = parser::parse_ffmpeg_time(line) {
            let clip_dur = processes
                .lock()
                .ok()
                .and_then(|map| map.get(task_id).and_then(|info| info.clip_duration));
            if let Some(duration) = clip_dur {
                let percent = (current_secs / duration * 100.0).min(100.0);
                emit(DownloadEvent::Progress {
                    id: task_id.to_string(),
                    percent,
                    speed: String::new(),
                    eta: String::new(),
                    downloaded: format_duration(current_secs),
                    total: format_duration(duration),
                });
            }
        }
        return;
    }

    // 跟踪输出文件路径
    if let Some(dest) = parse_destination(line) {
        if let Ok(mut map) = processes.lock() {
            if let Some(info) = map.get_mut(task_id) {
                info.output_files.push(dest);
            }
        }
    }

    emit(DownloadEvent::Log {
        id: task_id.to_string(),
        line: line.to_string(),
    });
}

// ========== 下载参数构建 ==========

/// 构建 yt-dlp 下载参数字符串列表
pub fn build_download_args(
    app_data_dir: &Path,
    params: &DownloadParams,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--newline".to_string(),
        "--ignore-config".to_string(),
        "--color".to_string(),
        "never".to_string(),
        "--progress-template".to_string(),
        r#"download:PROGRESS_JSON:{"percent":"%(progress._percent_str|0%)s","speed":"%(progress._speed_str|)s","eta":"%(progress._eta_str|)s","downloaded":"%(progress._downloaded_bytes_str|)s","total":"%(progress._total_bytes_str|)s"}"#.to_string(),
    ];

    args.extend(utils::build_js_runtime_args(app_data_dir));
    args.extend(utils::build_plugin_args(app_data_dir));
    args.extend(utils::build_youtube_extractor_args());

    // 格式选择
    match params.download_mode.as_str() {
        "video" => {
            if let Some(ref vf) = params.video_format {
                if !vf.is_empty() {
                    args.push("-f".to_string());
                    args.push(vf.clone());
                }
            }
        }
        "audio" => {
            if let Some(ref af) = params.audio_format {
                if !af.is_empty() {
                    args.push("-f".to_string());
                    args.push(af.clone());
                }
            }
        }
        _ => {
            let vf = params.video_format.as_deref().filter(|s| !s.is_empty());
            let af = params.audio_format.as_deref().filter(|s| !s.is_empty());
            match (vf, af) {
                (Some(v), Some(a)) => {
                    args.push("-f".to_string());
                    args.push(format!("{}+{}", v, a));
                }
                (Some(v), None) => {
                    args.push("-f".to_string());
                    args.push(format!("{}+bestaudio", v));
                }
                (None, Some(a)) => {
                    args.push("-f".to_string());
                    args.push(format!("bestvideo+{}", a));
                }
                _ => {}
            }
        }
    }

    // 代理
    if let Some(ref proxy) = params.proxy {
        if !proxy.is_empty() {
            args.push("--proxy".to_string());
            args.push(proxy.clone());
        }
    }

    // 输出路径模板
    let template = params
        .output_template
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("%(title).200s.%(ext)s");
    let output_template = PathBuf::from(&params.download_dir)
        .join(template)
        .to_string_lossy()
        .to_string();
    args.push("-o".to_string());
    args.push(output_template);
    args.push("--windows-filenames".to_string());

    if params.no_overwrites {
        args.push("--no-overwrites".to_string());
    }

    if let Some(n) = params.concurrent_fragments {
        if n > 1 {
            args.push("--concurrent-fragments".to_string());
            args.push(n.to_string());
        }
    }

    append_cookie_proxy_args(
        &mut args,
        params.cookie_file.as_deref(),
        params.cookie_browser.as_deref(),
        None,
    );

    if params.embed_subs {
        args.push("--embed-subs".to_string());
    }
    if params.embed_thumbnail {
        args.push("--embed-thumbnail".to_string());
    }
    if params.embed_metadata {
        args.push("--embed-metadata".to_string());
    }
    if params.embed_chapters {
        args.push("--embed-chapters".to_string());
    }
    if params.sponsorblock_remove {
        args.push("--sponsorblock-remove".to_string());
        args.push("all".to_string());
    }
    if params.extract_audio {
        args.push("-x".to_string());
        if let Some(ref fmt) = params.audio_convert_format {
            if !fmt.is_empty() {
                args.push("--audio-format".to_string());
                args.push(fmt.clone());
            }
        }
    }
    if params.no_merge {
        args.push("--no-merge-output".to_string());
    }
    if let Some(ref fmt) = params.recode_format {
        if !fmt.is_empty() {
            args.push("--recode-video".to_string());
            args.push(fmt.clone());
        }
    }
    if let Some(ref rate) = params.limit_rate {
        if !rate.is_empty() {
            args.push("-r".to_string());
            args.push(rate.clone());
        }
    }
    if let Some(ref ffmpeg_args) = params.ffmpeg_args {
        if !ffmpeg_args.is_empty() {
            args.push("--postprocessor-args".to_string());
            args.push(ffmpeg_args.clone());
        }
    }

    if !params.subtitles.is_empty() {
        args.push("--write-subs".to_string());
        args.push("--sub-langs".to_string());
        args.push(params.subtitles.join(","));
    }

    let has_start = params.start_time.is_some_and(|t| t > 0.0);
    let has_end = params.end_time.is_some();
    if has_start || has_end {
        let start = params.start_time.unwrap_or(0.0);
        let end_str = params
            .end_time
            .map(|t| format!("{}", t))
            .unwrap_or_else(|| "inf".to_string());
        args.push("--download-sections".to_string());
        args.push(format!("*{}-{}", start, end_str));
    }

    if params.no_playlist {
        args.push("--no-playlist".to_string());
    } else if let Some(ref items) = params.playlist_items {
        if !items.is_empty() {
            args.push("--playlist-items".to_string());
            args.push(items.clone());
        }
    }

    args.push(params.url.clone());

    args
}

// ========== 异步输出读取 ==========

/// 启动异步任务读取子进程输出流，通过回调发送事件
pub fn spawn_output_reader<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    emit: EventCallback,
    task_id: String,
    processes: Arc<Mutex<HashMap<String, DownloadProcessInfo>>>,
    reader: R,
) {
    tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf_reader = tokio::io::BufReader::new(reader);
        const MAX_LINE_LEN: usize = 64 * 1024;
        let mut line_buf = Vec::with_capacity(1024);
        let mut byte_buf = [0u8; 1];

        loop {
            match buf_reader.read(&mut byte_buf).await {
                Ok(0) => {
                    if !line_buf.is_empty() {
                        let line = String::from_utf8_lossy(&line_buf).trim().to_string();
                        if !line.is_empty() {
                            process_output_line(&emit, &task_id, &processes, &line);
                        }
                    }
                    break;
                }
                Ok(_) => {
                    if byte_buf[0] == b'\n' || byte_buf[0] == b'\r' {
                        if !line_buf.is_empty() {
                            let line = String::from_utf8_lossy(&line_buf).trim().to_string();
                            if !line.is_empty() {
                                process_output_line(&emit, &task_id, &processes, &line);
                            }
                            line_buf.clear();
                        }
                    } else if line_buf.len() < MAX_LINE_LEN {
                        line_buf.push(byte_buf[0]);
                    }
                }
                Err(_) => break,
            }
        }
    });
}

/// 启动异步任务等待子进程完成并发送结果事件
pub fn spawn_completion_handler(
    emit: EventCallback,
    task_id: String,
    processes: Arc<Mutex<HashMap<String, DownloadProcessInfo>>>,
    mut child: tokio::process::Child,
) {
    tokio::spawn(async move {
        let status = child.wait().await;

        let was_cancelled = processes
            .lock()
            .ok()
            .and_then(|map| map.get(&task_id).map(|info| info.cancelled))
            .unwrap_or(false);

        let success = matches!(&status, Ok(s) if s.success());

        if success {
            let (output_file, _) = resolve_output_file(&processes, &task_id);
            emit(DownloadEvent::Complete {
                id: task_id.clone(),
                output_file,
            });
        } else if !was_cancelled {
            let _ = resolve_output_file(&processes, &task_id);
            let error_msg = status
                .as_ref()
                .map(|s| format!("err_exit_code:{}", s.code().unwrap_or(-1)))
                .unwrap_or_else(|e| e.to_string());
            emit(DownloadEvent::Error {
                id: task_id.clone(),
                error: error_msg,
            });
        }

        if let Ok(mut map) = processes.lock() {
            map.remove(&task_id);
        }
    });
}

// ========== 启动下载（组合入口） ==========

/// 启动 yt-dlp 下载进程，返回 PID。
/// 此函数构建参数、spawn 子进程、启动 stdout/stderr 读取器和完成处理器。
pub async fn start_download_process(
    ytdlp_path: &Path,
    app_data_dir: &Path,
    emit: EventCallback,
    processes: Arc<Mutex<HashMap<String, DownloadProcessInfo>>>,
    params: DownloadParams,
) -> Result<u32, String> {
    if !ytdlp_path.exists() {
        return Err("err_ytdlp_not_installed".to_string());
    }

    let args = build_download_args(app_data_dir, &params);

    let filepath_file = app_data_dir
        .join(format!("{}_filepath.txt", params.id))
        .to_string_lossy()
        .to_string();

    let mut full_args = args;
    full_args.push("--print-to-file".to_string());
    full_args.push("after_move:filepath".to_string());
    full_args.push(filepath_file.clone());

    let mut cmd = tokio::process::Command::new(ytdlp_path);
    cmd.args(&full_args)
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("err_start_download:{}", e))?;

    let pid = child.id().ok_or("err_get_pid")?;
    let task_id = params.id.clone();

    let clip_duration = match (params.start_time, params.end_time) {
        (Some(s), Some(e)) => Some(e - s),
        (None, Some(e)) => Some(e),
        _ => None,
    };

    {
        let mut map = processes.lock().map_err(|e| e.to_string())?;
        map.insert(
            task_id.clone(),
            DownloadProcessInfo {
                pid,
                cancelled: false,
                output_files: Vec::new(),
                download_dir: params.download_dir.clone(),
                filepath_file: Some(filepath_file),
                clip_duration,
            },
        );
    }

    let stdout = child.stdout.take().ok_or("err_capture_stdout")?;
    let stderr = child.stderr.take().ok_or("err_capture_stderr")?;

    spawn_output_reader(
        emit.clone(),
        task_id.clone(),
        processes.clone(),
        stdout,
    );
    spawn_output_reader(
        emit.clone(),
        task_id.clone(),
        processes.clone(),
        stderr,
    );
    spawn_completion_handler(emit, task_id, processes.clone(), child);

    Ok(pid)
}

/**
 * API 抽象层入口 — 自动检测本地/远程模式
 *
 * - 桌面端（Tauri webview + Windows/macOS/Linux）：本地 Tauri IPC
 * - 移动端（Tauri webview + iOS/Android）或无 Tauri：HTTP + WebSocket 远程连接
 * - 桌面端开启 LAN 访问后，内嵌 HTTP 服务供移动端连接
 */

import type { UnlistenFn } from "./types";
import { createLocalClient } from "./local";
import { createRemoteClient } from "./remote";

let _client: ReturnType<typeof createLocalClient> | ReturnType<typeof createRemoteClient> | null = null;
let _serverUrl = "";

/** 获取 Tauri invoke（local 模式已初始化时直接返回，否则动态 import） */
let _tauriInvoke: Function | null = null;

async function getTauriInvoke() {
  if (!_tauriInvoke) {
    const { invoke } = await import("@tauri-apps/api/core");
    _tauriInvoke = invoke;
  }
  return _tauriInvoke;
}

/** 初始化 API（App.vue 启动时调用一次） */
export async function initApi(): Promise<void> {
  const isTauri = !!(window as any).__TAURI_INTERNALS__;

  if (isTauri) {
    // 检测平台
    const invokeFn = await getTauriInvoke();
    const platform: string = await invokeFn("get_platform").catch(() => "unknown");

    if (["ios", "android"].includes(platform)) {
      // 移动端 Tauri → 需远程服务器
      _client = createRemoteClient(_serverUrl);
    } else {
      // 桌面端 → 本地模式
      _client = createLocalClient();
    }
  } else {
    // 非 Tauri 环境 → 远程模式
    _client = createRemoteClient(_serverUrl);
  }
}

/** 设置远程服务器地址并切换到远程模式 */
export function setServerUrl(url: string): void {
  _serverUrl = url;
  if (_client) {
    _client.disconnect();
  }
  _client = createRemoteClient(url);
}

/** 获取远程服务器地址 */
export function getServerUrl(): string {
  return _serverUrl;
}

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  // 如果 client 未初始化，先初始化
  if (!_client) {
    await initApi();
  }
  return _client!.invoke<T>(cmd, args);
}

export function on<T = unknown>(
  event: string,
  handler: (payload: T) => void,
): UnlistenFn {
  if (!_client) {
    // 未初始化时延迟注册（等 initApi 完成后生效）
    initApi().then(() => {
      _client?.on<T>(event, handler);
    });
    return () => {};
  }
  return _client.on<T>(event, handler);
}

export function disconnect(): void {
  if (_client) {
    _client.disconnect();
    _client = null;
  }
}

/** 测试服务器连接 */
export async function testConnection(serverUrl: string): Promise<boolean> {
  try {
    const resp = await fetch(`${serverUrl}/api/ping`, {
      method: "GET",
      signal: AbortSignal.timeout(5000),
    });
    return resp.ok;
  } catch {
    return false;
  }
}

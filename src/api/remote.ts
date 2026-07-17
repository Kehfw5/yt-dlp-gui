/**
 * 远程模式：通过 HTTP + WebSocket 与 yt-dlp-gui-server 通信
 */

import type { ApiClient, ConnectionMode, UnlistenFn } from "./types";

export function createRemoteClient(serverUrl: string): ApiClient {
  let mode: ConnectionMode = "remote";
  let ws: WebSocket | null = null;
  const eventHandlers = new Map<string, Set<(payload: unknown) => void>>();
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let wsUrl = "";

  // 将 HTTP URL 转为 WebSocket URL
  const buildWsUrl = (httpUrl: string): string => {
    const url = new URL(httpUrl);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    url.pathname = "/ws";
    return url.toString();
  };

  // WebSocket 连接管理
  const connectWs = () => {
    if (ws && ws.readyState === WebSocket.OPEN) return;
    wsUrl = buildWsUrl(serverUrl);

    try {
      ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        console.log("[remote] WebSocket connected");
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
      };

      ws.onmessage = (msg) => {
        try {
          const data = JSON.parse(msg.data);
          // 事件格式: { type: "download-progress", ...payload }
          const eventType = data.type;
          if (eventType && eventHandlers.has(eventType)) {
            const handlers = eventHandlers.get(eventType)!;
            handlers.forEach((h) => h(data));
          }
        } catch {
          // 忽略解析失败的消息
        }
      };

      ws.onclose = () => {
        // 自动重连（5 秒后）
        if (reconnectTimer) clearTimeout(reconnectTimer);
        reconnectTimer = setTimeout(connectWs, 5000);
      };

      ws.onerror = () => {
        ws?.close();
      };
    } catch {
      // WebSocket 创建失败，稍后重试
      if (reconnectTimer) clearTimeout(reconnectTimer);
      reconnectTimer = setTimeout(connectWs, 5000);
    }
  };

  // 初始化 WebSocket
  connectWs();

  return {
    async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      const resp = await fetch(`${serverUrl}/api/${cmd}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: args ? JSON.stringify(args) : undefined,
      });

      if (!resp.ok) {
        const text = await resp.text().catch(() => resp.statusText);
        throw new Error(text || `HTTP ${resp.status}`);
      }

      // 对于 204 No Content，返回 undefined
      if (resp.status === 204) return undefined as T;

      return resp.json();
    },

    on<T>(event: string, handler: (payload: T) => void): UnlistenFn {
      if (!eventHandlers.has(event)) {
        eventHandlers.set(event, new Set());
      }
      const handlers = eventHandlers.get(event)!;
      const wrappedHandler = handler as (payload: unknown) => void;
      handlers.add(wrappedHandler);

      // 确保 WebSocket 已连接
      if (!ws || ws.readyState !== WebSocket.OPEN) {
        connectWs();
      }

      return () => {
        handlers.delete(wrappedHandler);
      };
    },

    getMode(): ConnectionMode {
      return mode;
    },

    setMode(m: ConnectionMode, url?: string) {
      mode = m;
      if (url && url !== serverUrl) {
        // URL 变了，重连
        if (ws) {
          ws.close();
          ws = null;
        }
        // 注意：这里 serverUrl 是 const 参数，实际切换由 api/index.ts 处理
      }
    },

    disconnect() {
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
      if (ws) {
        ws.onclose = null; // 防止触发重连
        ws.close();
        ws = null;
      }
    },
  };
}

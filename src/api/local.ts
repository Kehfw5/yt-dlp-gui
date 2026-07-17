/**
 * 本地模式：通过 Tauri IPC 与后端通信
 */

import type { ApiClient, ConnectionMode, UnlistenFn } from "./types";

export function createLocalClient(): ApiClient {
  let mode: ConnectionMode = "local";

  async function getTauriInvoke() {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke;
  }

  async function getTauriListen() {
    const { listen } = await import("@tauri-apps/api/event");
    return listen;
  }

  return {
    async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      const invokeFn = await getTauriInvoke();
      return invokeFn<T>(cmd, args);
    },

    on<T>(event: string, handler: (payload: T) => void): UnlistenFn {
      let unlisten: UnlistenFn = () => {};
      let cancelled = false;

      getTauriListen().then((listenFn) => {
        if (cancelled) return;
        listenFn<T>(event, (e) => {
          // Tauri 事件的 payload 在 e.payload 中
          handler((e as unknown as { payload: T }).payload);
        }).then((fn) => {
          unlisten = fn;
        });
      });

      return () => {
        cancelled = true;
        unlisten();
      };
    },

    getMode(): ConnectionMode {
      return mode;
    },

    setMode(m: ConnectionMode) {
      mode = m;
    },

    disconnect() {
      // 本地模式无需断开
    },
  };
}

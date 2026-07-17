/** 连接模式 */
export type ConnectionMode = "local" | "remote";

/** API 客户端接口（本地和远程实现共用） */
export interface ApiClient {
  /** 调用后端命令 */
  invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  /** 注册事件监听，返回取消监听函数 */
  on<T = unknown>(event: string, handler: (payload: T) => void): () => void;
  /** 获取当前连接模式 */
  getMode(): ConnectionMode;
  /** 切换连接模式 */
  setMode(mode: ConnectionMode, serverUrl?: string): void;
  /** 断开远程连接 */
  disconnect(): void;
}

/** 取消监听函数 */
export type UnlistenFn = () => void;

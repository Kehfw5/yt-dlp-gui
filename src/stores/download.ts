import { defineStore } from "pinia";
import { invoke, on } from "@/api";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import localforage from "localforage";
import type { DownloadTask } from "@/types";
import { useSettingStore } from "@/stores/setting";
import i18n from "@/locales";

const storage = localforage.createInstance({
  name: "yt-dlp-gui",
  storeName: "downloads",
});

const STORAGE_KEY = "download_tasks";

interface ProgressPayload {
  id: string;
  percent: number;
  speed: string;
  eta: string;
  downloaded: string;
  total: string;
}

export const useDownloadStore = defineStore("download", () => {
  const tasks = ref<DownloadTask[]>([]);
  const loaded = ref(false);
  let listenersSetup = false;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  /** 当前正在下载的任务数 */
  const activeCount = computed(() => tasks.value.filter((t) => t.status === "downloading").length);

  /** 尝试启动队列中的下一个任务 */
  const tryStartNext = async () => {
    const settingStore = useSettingStore();
    const max = settingStore.maxConcurrentDownloads;
    if (max > 0 && activeCount.value >= max) return;

    const next = tasks.value.find((t) => t.status === "queued");
    if (!next) return;

    next.status = "downloading";
    try {
      await invoke("start_download", {
        params: { id: next.id, ...next.params },
      });
    } catch {
      next.status = "error";
      next.error = i18n.global.t("downloads.startFailed");
    }
  };

  /** 判断是否需要排队，返回 true 表示可以直接下载 */
  const canStartNow = (): boolean => {
    const settingStore = useSettingStore();
    const max = settingStore.maxConcurrentDownloads;
    return max <= 0 || activeCount.value < max;
  };

  const notify = async (title: string, body: string) => {
    const settingStore = useSettingStore();
    const mode = settingStore.notifyMode;
    if (mode === "none") return;

    if (mode === "app" || mode === "all") {
      window.$notification.create({ title, content: body, duration: 5000 });
    }

    if (mode === "system" || mode === "all") {
      try {
        let granted = await isPermissionGranted();
        if (!granted) {
          const permission = await requestPermission();
          granted = permission === "granted";
        }
        if (granted) {
          sendNotification({ title, body });
        }
      } catch {
        // 移动端/远程模式下通知 API 不可用，静默忽略
      }
    }
  };

  /** 防抖保存任务列表到 IndexedDB */
  const saveTasks = () => {
    if (!loaded.value) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      storage.setItem(STORAGE_KEY, JSON.parse(JSON.stringify(tasks.value)));
    }, 500);
  };

  /** 从 IndexedDB 恢复任务列表 */
  const loadTasks = async () => {
    const saved = await storage.getItem<DownloadTask[]>(STORAGE_KEY);
    if (saved && Array.isArray(saved)) {
      for (const task of saved) {
        if (task.status === "downloading" || task.status === "paused" || task.status === "queued") {
          task.status = "error";
          task.error = i18n.global.t("downloads.appRestarted");
          task.speed = "";
        }
        if (!Array.isArray(task.logs)) task.logs = [];
        if (!task.createdAt) task.createdAt = Date.now();
      }

      const completedWithFile = saved.filter((t) => t.status === "completed" && t.outputFile);
      if (completedWithFile.length > 0) {
        try {
          const paths = completedWithFile.map((t) => t.outputFile!);
          const exists = await invoke<boolean[]>("check_files_exist", { paths });
          const missingIds = new Set<string>();
          completedWithFile.forEach((t, i) => {
            if (!exists[i]) missingIds.add(t.id);
          });
          if (missingIds.size > 0) {
            const filtered = saved.filter((t) => !missingIds.has(t.id));
            tasks.value = filtered;
            loaded.value = true;
            return;
          }
        } catch {
          // 检查失败则保留全部
        }
      }

      tasks.value = saved;
    }
    loaded.value = true;
  };

  watch(tasks, saveTasks, { deep: true });

  /** 更新任务栏进度条（仅桌面端） */
  let _windowApi: typeof import("@tauri-apps/api/window") | null = null;
  const getWindowApi = async () => {
    if (!_windowApi) {
      try {
        _windowApi = await import("@tauri-apps/api/window");
      } catch {
        _windowApi = null;
      }
    }
    return _windowApi;
  };

  const updateTaskbarProgress = async () => {
    const winApi = await getWindowApi();
    if (!winApi) return;

    const settingStore = useSettingStore();
    const appWindow = winApi.getCurrentWindow();

    if (!settingStore.showTaskbarProgress) {
      appWindow.setProgressBar({ status: winApi.ProgressBarStatus.None });
      return;
    }

    const downloading = tasks.value.filter((t) => t.status === "downloading");
    const paused = tasks.value.filter((t) => t.status === "paused");

    if (downloading.length > 0) {
      const avg = Math.round(
        downloading.reduce((sum, t) => sum + (t.percent || 0), 0) / downloading.length,
      );
      appWindow.setProgressBar({ status: winApi.ProgressBarStatus.Normal, progress: avg });
    } else if (paused.length > 0) {
      const avg = Math.round(paused.reduce((sum, t) => sum + (t.percent || 0), 0) / paused.length);
      appWindow.setProgressBar({ status: winApi.ProgressBarStatus.Paused, progress: avg });
    } else {
      appWindow.setProgressBar({ status: winApi.ProgressBarStatus.None });
    }
  };

  /** 注册事件监听，仅初始化一次 */
  const setupListeners = () => {
    if (listenersSetup) return;
    listenersSetup = true;

    on<ProgressPayload>("download-progress", (event) => {
      const task = tasks.value.find((t) => t.id === event.id);
      if (task && task.status === "downloading") {
        task.percent = event.percent;
        task.speed = event.speed;
        task.eta = event.eta;
        if (event.downloaded) task.downloaded = event.downloaded;
        if (event.total) task.total = event.total;
      }
      updateTaskbarProgress();
    });

    on<{ id: string; line: string }>("download-log", (event) => {
      const task = tasks.value.find((t) => t.id === event.id);
      if (task) {
        task.logs.push(event.line);
      }
    });

    on<{ id: string; outputFile: string }>("download-complete", (event) => {
      const task = tasks.value.find((t) => t.id === event.id);
      if (task) {
        task.status = "completed";
        task.percent = 100;
        task.speed = "";
        if (event.outputFile) task.outputFile = event.outputFile;
        notify(
          i18n.global.t("downloads.notifyComplete"),
          task.title || i18n.global.t("downloads.notifyCompleteBody"),
        );
      }
      updateTaskbarProgress();
      tryStartNext();
    });

    on<{ id: string; error: string }>("download-error", (event) => {
      const task = tasks.value.find((t) => t.id === event.id);
      if (task && task.status !== "cancelled") {
        task.status = "error";
        task.error = event.error;
        task.speed = "";
      }
      updateTaskbarProgress();
      tryStartNext();
    });
  };

  loadTasks();
  setupListeners();

  /** 添加新的下载任务到列表顶部 */
  const addTask = (task: DownloadTask) => {
    tasks.value.unshift(task);
  };

  /** 暂停指定下载任务 */
  const pauseTask = async (id: string) => {
    await invoke("pause_download", { id });
    const task = tasks.value.find((t) => t.id === id);
    if (task) {
      task.status = "paused";
      task.speed = "";
    }
    updateTaskbarProgress();
  };

  /** 恢复指定已暂停的下载任务 */
  const resumeTask = async (id: string) => {
    await invoke("resume_download", { id });
    const task = tasks.value.find((t) => t.id === id);
    if (task) {
      task.status = "downloading";
    }
    updateTaskbarProgress();
  };

  /** 取消下载任务并删除已下载的文件 */
  const cancelTask = async (id: string) => {
    const task = tasks.value.find((t) => t.id === id);
    if (!task) return;

    const wasQueued = task.status === "queued";
    task.status = "cancelled";

    if (!wasQueued) {
      try {
        await invoke("cancel_download", { id, deleteFiles: true });
      } catch {
        // 进程可能已退出
      }
    }

    updateTaskbarProgress();
    tryStartNext();
  };

  /** 重新下载失败或已取消的任务 */
  const retryTask = async (id: string) => {
    const task = tasks.value.find((t) => t.id === id);
    if (!task) return;

    const newId = `dl_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    task.id = newId;
    task.percent = 0;
    task.speed = "";
    task.eta = "";
    task.downloaded = "";
    task.total = "";
    task.logs = [];
    task.error = undefined;

    if (canStartNow()) {
      task.status = "downloading";
      await invoke("start_download", {
        params: { id: newId, ...task.params },
      });
    } else {
      task.status = "queued";
    }
  };

  /** 从列表中移除指定任务 */
  const removeTask = (id: string) => {
    const idx = tasks.value.findIndex((t) => t.id === id);
    if (idx !== -1) tasks.value.splice(idx, 1);
  };

  /** 清空所有已完成、失败、已取消的任务 */
  const clearFinished = () => {
    tasks.value = tasks.value.filter(
      (t) => t.status !== "completed" && t.status !== "error" && t.status !== "cancelled",
    );
  };

  return {
    tasks,
    loaded,
    activeCount,
    canStartNow,
    addTask,
    pauseTask,
    resumeTask,
    cancelTask,
    retryTask,
    removeTask,
    clearFinished,
  };
});

import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { showErrorDialog } from "@/utils/format";
import { useSettingStore } from "@/stores/setting";
import { useStatusStore } from "@/stores/status";
import type { VideoInfo, VideoFormat, PlaylistEntry, DenoStatus } from "@/types";

type SubtitleMap = NonNullable<PlaylistEntry["subtitles"]>;

/** 聚合 playlist 各 entry 的字幕到一个并集；同语言取首个出现的 entry 的 tracks */
const aggregateSubtitleMap = (
  entries: PlaylistEntry[],
  field: "subtitles" | "automatic_captions",
): SubtitleMap => {
  const merged: SubtitleMap = {};
  for (const entry of entries) {
    const map = entry[field];
    if (!map) continue;
    for (const [lang, tracks] of Object.entries(map)) {
      if (!merged[lang] && tracks?.length) merged[lang] = tracks;
    }
  }
  return merged;
};

export const useVideoStore = defineStore("video", () => {
  const url = ref("");
  const videoInfo = ref<VideoInfo | null>(null);
  const videoFormats = ref<VideoFormat[]>([]);
  const audioFormats = ref<VideoFormat[]>([]);
  const fetching = ref(false);

  // 播放列表
  const isPlaylist = ref(false);
  const playlistEntries = ref<PlaylistEntry[]>([]);
  const selectedPlaylistItems = ref<number[]>([]);

  /** 获取当前有效的 Cookie 参数 */
  const getCookieArgs = async (): Promise<{
    cookieFile: string | null;
    cookieBrowser: string | null;
  }> => {
    const settingStore = useSettingStore();
    const { cookieMode, cookieText, cookieFile, cookieBrowser } = settingStore;
    if (cookieMode === "text" && cookieText.trim()) {
      const path = await invoke<string>("save_cookie_text", { text: cookieText });
      return { cookieFile: path, cookieBrowser: null };
    }
    if (cookieMode === "file" && cookieFile) {
      return { cookieFile, cookieBrowser: null };
    }
    if (cookieMode === "browser" && cookieBrowser) {
      return { cookieFile: null, cookieBrowser };
    }
    return { cookieFile: null, cookieBrowser: null };
  };

  /** 解析视频信息并填充 store，返回是否成功 */
  const fetchVideoInfo = async (targetUrl: string): Promise<boolean> => {
    const settingStore = useSettingStore();
    fetching.value = true;
    try {
      const { cookieFile, cookieBrowser } = await getCookieArgs();
      const info = await invoke<VideoInfo>("fetch_video_info", {
        url: targetUrl,
        cookieFile,
        cookieBrowser,
        proxy: settingStore.proxy || null,
      });

      url.value = targetUrl;

      if (info._type === "playlist" && info.entries?.length) {
        isPlaylist.value = true;
        playlistEntries.value = info.entries.map((e, i) => ({
          id: e.id || String(i + 1),
          title: e.title || `第 ${i + 1} P`,
          duration: e.duration ?? null,
          url: e.url || "",
        }));
        selectedPlaylistItems.value = playlistEntries.value.map((_, i) => i + 1);
        const firstEntry = info.entries[0];
        const formats: VideoFormat[] = firstEntry?.formats || info.formats || [];
        // 合集字幕：yt-dlp -J 对 playlist 不会在 root 暴露 subtitles，
        // 必须从各 entry 聚合。同语言的 tracks 取首个出现该语言的 entry，
        // 这样默认数据来源跟随 P1（与现有「分 P 默认全选」一致）。
        videoInfo.value = {
          ...info,
          title: info.title || firstEntry?.title || "",
          thumbnail: info.thumbnail || firstEntry?.thumbnail || "",
          duration: info.duration || firstEntry?.duration || 0,
          formats,
          subtitles: aggregateSubtitleMap(info.entries, "subtitles"),
          automatic_captions: aggregateSubtitleMap(info.entries, "automatic_captions"),
        };
      } else {
        isPlaylist.value = false;
        playlistEntries.value = [];
        selectedPlaylistItems.value = [];
        videoInfo.value = info;
      }

      const formats: VideoFormat[] = videoInfo.value.formats || [];
      videoFormats.value = formats
        .filter((f) => f.vcodec && f.vcodec !== "none" && (!f.acodec || f.acodec === "none"))
        .sort((a, b) => (b.height || 0) - (a.height || 0));
      audioFormats.value = formats
        .filter((f) => f.acodec && f.acodec !== "none" && (!f.vcodec || f.vcodec === "none"))
        .sort((a, b) => (b.abr || 0) - (a.abr || 0));

      // YouTube URL 且 Deno 未安装时提示
      if (/youtube\.com|youtu\.be/i.test(targetUrl)) {
        try {
          const denoStatus = await invoke<DenoStatus>("get_deno_status");
          if (!denoStatus.installed) {
            const statusStore = useStatusStore();
            statusStore.showDenoSetupModal = true;
          }
        } catch {
          // ignore
        }
      }

      return true;
    } catch (e: unknown) {
      const raw = e instanceof Error ? e.message : String(e) || "获取视频信息失败";
      if (/err_ytdlp_not_installed/.test(raw)) {
        const statusStore = useStatusStore();
        statusStore.showYtdlpSetupModal = true;
      } else if (/Could not copy.*cookie database/i.test(raw)) {
        showErrorDialog(raw);
      } else if (/sign in|cookies/i.test(raw)) {
        const statusStore = useStatusStore();
        statusStore.showCookieModal = true;
      } else {
        showErrorDialog(raw);
      }
      return false;
    } finally {
      fetching.value = false;
    }
  };

  const clear = () => {
    url.value = "";
    videoInfo.value = null;
    videoFormats.value = [];
    audioFormats.value = [];
    isPlaylist.value = false;
    playlistEntries.value = [];
    selectedPlaylistItems.value = [];
  };

  return {
    url,
    videoInfo,
    videoFormats,
    audioFormats,
    fetching,
    isPlaylist,
    playlistEntries,
    selectedPlaylistItems,
    fetchVideoInfo,
    getCookieArgs,
    clear,
  };
});

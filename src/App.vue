<script setup lang="ts">
import { invoke, initApi, setServerUrl } from "@/api";
import { listen } from "@tauri-apps/api/event";
import { exit } from "@tauri-apps/plugin-process";
import { onOpenUrl, getCurrent as getCurrentDeepLink } from "@tauri-apps/plugin-deep-link";
import IconMdiHome from "~icons/mdi/home";
import IconMdiPlaylistPlay from "~icons/mdi/playlist-play";
import IconMdiDownload from "~icons/mdi/download";
import IconMdiToolbox from "~icons/mdi/toolbox";
import type { Component } from "vue";
import { useThemeVars } from "naive-ui";
import { useI18n } from "vue-i18n";
import { useSettingStore } from "@/stores/setting";
import { useDownloadStore } from "@/stores/download";
import { usePendingStore } from "@/stores/pending";
import { useStatusStore } from "@/stores/status";
import { localeEntries } from "@/locales";

const { t } = useI18n();
const router = useRouter();
const route = useRoute();
const settingStore = useSettingStore();
const downloadStore = useDownloadStore();
const pendingStore = usePendingStore();
const themeVars = useThemeVars();

// 平台检测：通过 invoke 获取平台信息
const platform = ref<"windows" | "macos" | "linux" | "ios" | "android">("linux");
const isDesktop = computed(() => ["windows", "macos", "linux"].includes(platform.value));
invoke<string>("get_platform")
  .then((p) => {
    platform.value = p as typeof platform.value;
  })
  .catch(() => {
    // 降级：无法获取平台时默认为 desktop
    platform.value = "linux";
  });

const navBadgeCounts = computed<Record<string, number>>(() => ({
  pending: pendingStore.items.length,
  downloads: downloadStore.tasks.filter(
    (t) => t.status === "downloading" || t.status === "queued" || t.status === "paused",
  ).length,
}));

/** 同步托盘菜单语言（仅桌面端） */
const syncTrayMenu = () => {
  if (!isDesktop.value) return;
  invoke("update_tray_menu", {
    showLabel: t("tray.show"),
    quitLabel: t("tray.quit"),
  }).catch(() => {});
};

watch(() => settingStore.locale, syncTrayMenu);

/** 处理退出请求，有下载任务时弹出确认框 */
const handleQuitRequest = () => {
  if (downloadStore.activeCount > 0) {
    window.$dialog.warning({
      title: t("tray.quitConfirmTitle"),
      content: t("tray.quitConfirmContent"),
      positiveText: t("common.cancel"),
      negativeText: t("tray.quit"),
      onNegativeClick: () => exit(0),
    });
  } else {
    exit(0);
  }
};

const localeOptions = localeEntries.map((e) => ({ label: `${e.flag} ${e.label}`, value: e.code }));

const currentRoute = computed(() => {
  const name = (route.name as string) ?? "";
  if (name.startsWith("toolbox")) return "toolbox";
  return name;
});

const navItems: { key: string; icon: Component; labelKey: string }[] = [
  { key: "home", icon: IconMdiHome, labelKey: "nav.home" },
  { key: "pending", icon: IconMdiPlaylistPlay, labelKey: "nav.pending" },
  { key: "downloads", icon: IconMdiDownload, labelKey: "nav.downloads" },
  { key: "toolbox", icon: IconMdiToolbox, labelKey: "nav.toolbox" },
];

// 窗口关闭行为（仅桌面端，移动端由系统管理）
if (isDesktop.value) {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();

    win.onCloseRequested(async (event) => {
      if (settingStore.closeToTray) {
        event.preventDefault();
        await win.hide();
      } else {
        event.preventDefault();
        handleQuitRequest();
      }
    });
  } catch {
    // 桌面端 API 不可用，忽略
  }
}

// 监听托盘退出请求（仅桌面端）
if (isDesktop.value) {
  listen("tray-quit-requested", () => handleQuitRequest());
}

/** 同一 URL 短时间内重复送达时去重，避免 onOpenUrl + getCurrent 同时触发 */
let lastDeepLink = "";
let lastDeepLinkAt = 0;
const handleDeepLink = (deepLinkUrl: string) => {
  const now = Date.now();
  if (deepLinkUrl === lastDeepLink && now - lastDeepLinkAt < 1500) return;
  lastDeepLink = deepLinkUrl;
  lastDeepLinkAt = now;
  try {
    const url = new URL(deepLinkUrl);
    if (url.host !== "download") return;
    const videoUrl = url.searchParams.get("url");
    if (!videoUrl) return;
    const cookies = url.searchParams.get("cookies");
    if (cookies) {
      try {
        settingStore.cookieText = decodeURIComponent(atob(cookies));
        settingStore.cookieMode = "text";
      } catch {
        // Cookie 解码失败，忽略
      }
    }
    router.push({ name: "home", query: { url: videoUrl } });
  } catch {
    // 无效的深链接 URL，忽略
  }
};

/** 启动时自动检查应用更新（仅桌面端） */
const checkAppUpdate = async () => {
  if (!isDesktop.value) return;
  try {
    const statusStore = useStatusStore();
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    if (update) {
      statusStore.updateVersion = update.version;
      statusStore.updateNotes = update.body || "";
      statusStore.showUpdateModal = true;
    }
  } catch {
    // 静默失败，不打扰用户
  }
};

onMounted(async () => {
  // 自动检测平台并初始化 API
  await initApi();

  // 获取平台信息
  const p = await invoke<string>("get_platform").catch(() => "unknown");
  platform.value = p as typeof platform.value;

  // 移动端自动检测：如果没有配置过服务端地址，弹窗提示
  if (["ios", "android"].includes(platform.value)) {
    if (!settingStore.serverUrl) {
      const statusStore = useStatusStore();
      statusStore.showServerSetupModal = true;
    } else {
      setServerUrl(settingStore.serverUrl);
    }
  }

  // 桌面端：如开启了 LAN 访问，启动内嵌 HTTP 服务
  if (isDesktop.value && settingStore.lanEnabled) {
    invoke("start_lan_server", { port: settingStore.lanPort })
      .then(() => console.log("LAN server started on port", settingStore.lanPort))
      .catch((e) => console.warn("Failed to start LAN server:", e));
  }

  // 桌面端：显示窗口
  if (isDesktop.value) {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      getCurrentWindow().show();
    } catch {}
  }
  syncTrayMenu();
  if (settingStore.autoCheckUpdate) {
    checkAppUpdate();
  }
  // 深链接处理
  try {
    const initial = await getCurrentDeepLink();
    if (initial?.length) {
      for (const u of initial) handleDeepLink(u);
    }
  } catch {
    // 插件不可用时静默忽略
  }
  onOpenUrl((urls) => {
    for (const u of urls) handleDeepLink(u);
  });
  listen<string>("deep-link-url", (event) => {
    handleDeepLink(event.payload);
  });
});
</script>

<template>
  <Provider>
    <CookieModal />
    <UpdateModal />
    <SetupModal />
    <ServerSetupModal />
    <n-layout style="height: 100vh">
      <n-layout-header bordered class="app-header">
        <div class="header-side">
          <div class="logo" @click="router.push({ name: 'home' })">
            <img src="/app-icon.svg" alt="" class="logo-img" />
            <span class="logo-text">YDL GUI</span>
          </div>
        </div>
        <div class="header-nav">
          <n-badge
            v-for="item in navItems"
            :key="item.key"
            :value="navBadgeCounts[item.key] || 0"
            :max="99"
            :show="(navBadgeCounts[item.key] || 0) > 0"
            :color="themeVars.primaryColor"
            :offset="[-6, 4]"
          >
            <n-button
              :quaternary="currentRoute !== item.key"
              :type="currentRoute === item.key ? 'primary' : 'default'"
              :secondary="currentRoute === item.key"
              :focusable="false"
              round
              @click="router.push({ name: item.key })"
            >
              <template #icon>
                <n-icon>
                  <component :is="item.icon" />
                </n-icon>
              </template>
              <span class="nav-label" :class="{ expanded: currentRoute === item.key }">
                {{ $t(item.labelKey) }}
              </span>
            </n-button>
          </n-badge>
        </div>
        <div class="header-side header-side-right">
          <n-button
            :focusable="false"
            quaternary
            circle
            tag="a"
            href="https://github.com/imsyy/yt-dlp-gui"
            target="_blank"
          >
            <template #icon>
              <n-icon>
                <icon-mdi-github />
              </n-icon>
            </template>
          </n-button>
          <n-popselect v-model:value="settingStore.locale" :options="localeOptions" trigger="click">
            <n-button :focusable="false" quaternary circle>
              <template #icon>
                <n-icon>
                  <icon-mdi-translate />
                </n-icon>
              </template>
            </n-button>
          </n-popselect>
          <n-button
            :type="currentRoute === 'settings' ? 'primary' : 'default'"
            :secondary="currentRoute === 'settings'"
            :quaternary="currentRoute !== 'settings'"
            :focusable="false"
            circle
            @click="router.push({ name: 'settings' })"
          >
            <template #icon>
              <n-icon>
                <icon-mdi-cog />
              </n-icon>
            </template>
          </n-button>
        </div>
      </n-layout-header>
      <n-layout
        position="absolute"
        style="top: 56px"
        content-style="padding: 16px; display: flex; flex-direction: column; min-height: 100%;"
        :native-scrollbar="false"
      >
        <div style="flex: 1">
          <router-view v-slot="{ Component: RouteComponent }">
            <Transition name="fade-slide" mode="out-in">
              <component :is="RouteComponent" />
            </Transition>
          </router-view>
        </div>
        <n-flex justify="center" align="center" :size="4" class="app-footer">
          <n-text depth="3" style="font-size: 12px">
            © {{ new Date().getFullYear() }}
            <n-button
              text
              tag="a"
              href="https://github.com/imsyy"
              target="_blank"
              size="tiny"
              style="font-size: 12px"
            >
              imsyy
            </n-button>
            ·
            <n-button
              text
              tag="a"
              href="https://github.com/imsyy/yt-dlp-gui"
              target="_blank"
              size="tiny"
              style="font-size: 12px"
            >
              YDL GUI
            </n-button>
          </n-text>
        </n-flex>
      </n-layout>
    </n-layout>
  </Provider>
</template>

<style scoped lang="scss">
.app-header {
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 16px;

  .header-side {
    width: 120px;
    flex-shrink: 0;
    display: flex;
    align-items: center;

    &.header-side-right {
      justify-content: flex-end;
      gap: 4px;
    }
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 8px;
    user-select: none;
    cursor: pointer;

    .logo-img {
      width: 26px;
      height: 26px;
      transition: transform 0.3s;
    }

    .logo-text {
      font-weight: 700;
      font-size: 16px;
      letter-spacing: 0.5px;
    }

    &:hover .logo-img {
      transform: scale(1.06);
    }
  }

  .header-nav {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;

    :deep(.n-button) {
      .n-button__content {
        transition:
          max-width 0.2s ease,
          opacity 0.2s ease;
      }

      .n-button__icon {
        margin-right: 0;
      }

      &:not(.n-button--color) .n-button__icon {
        margin-left: 0;
      }
    }

    .nav-label {
      display: inline-block;
      max-width: 0;
      opacity: 0;
      overflow: hidden;
      transition:
        max-width 0.2s ease,
        opacity 0.2s ease,
        margin 0.2s ease;
      margin-left: 0;

      &.expanded {
        max-width: 80px;
        opacity: 1;
        margin-left: 4px;
      }
    }
  }
}

.app-footer {
  padding: 24px 0 4px;
  flex-shrink: 0;
}
</style>

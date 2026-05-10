<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open as openExternal } from "@tauri-apps/plugin-shell";
import IconMdiOpenInApp from "~icons/mdi/open-in-app";
import IconMdiDownloadOutline from "~icons/mdi/download-outline";
import IconMdiCursorDefaultClick from "~icons/mdi/cursor-default-click";
import IconMdiShieldCheckOutline from "~icons/mdi/shield-check-outline";
import IconMdiWeb from "~icons/mdi/web";
import IconMdiOpenInNew from "~icons/mdi/open-in-new";
import IconMdiFolderOpenOutline from "~icons/mdi/folder-open-outline";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const show = defineModel<boolean>("show", { default: false });

const REPO_FOLDER_URL = "https://github.com/imsyy/yt-dlp-gui/tree/master/browser-extension";

const SUPPORTED_SITES = [
  "YouTube", "Bilibili", "Twitch", "Vimeo", "Dailymotion", "Niconico",
  "Twitter / X", "Instagram", "TikTok", "Facebook", "Reddit",
  "SoundCloud", "Bandcamp", "Crunchyroll",
];

const sections = [
  {
    icon: IconMdiDownloadOutline,
    color: "#18a058",
    bg: "rgba(24,160,88,0.10)",
    titleKey: "browserExt.installHeading",
    items: [
      "browserExt.install1",
      "browserExt.install2",
      "browserExt.install3",
      "browserExt.install4",
    ],
    ordered: true,
  },
  {
    icon: IconMdiCursorDefaultClick,
    color: "#2080f0",
    bg: "rgba(32,128,240,0.10)",
    titleKey: "browserExt.usageHeading",
    items: [
      "browserExt.usage1",
      "browserExt.usage2",
      "browserExt.usage3",
    ],
    ordered: false,
  },
];

const revealing = ref(false);

const revealLocalFolder = async () => {
  revealing.value = true;
  try {
    await invoke<string>("reveal_browser_extension");
  } catch (e: unknown) {
    window.$message.error(String(e));
  } finally {
    revealing.value = false;
  }
};

const openRepo = () => openExternal(REPO_FOLDER_URL).catch(() => {});
</script>

<template>
  <n-modal
    v-model:show="show"
    preset="card"
    :title="t('browserExt.title')"
    size="small"
    :bordered="false"
    :style="{ width: '520px' }"
  >
    <n-flex vertical :size="14">
      <n-flex align="flex-start" :size="12" :wrap="false" class="intro">
        <div class="intro-icon">
          <n-icon :size="20" color="#06b6d4">
            <icon-mdi-open-in-app />
          </n-icon>
        </div>
        <n-text depth="2" style="font-size: 13px; line-height: 1.6; flex: 1; min-width: 0">
          {{ t("browserExt.intro") }}
        </n-text>
      </n-flex>

      <n-flex v-for="(sec, i) in sections" :key="i" vertical :size="6" class="block">
        <n-flex align="center" :size="8">
          <n-icon :size="16" :color="sec.color">
            <component :is="sec.icon" />
          </n-icon>
          <n-text strong>{{ t(sec.titleKey) }}</n-text>
        </n-flex>
        <component
          :is="sec.ordered ? 'ol' : 'ul'"
          class="sec-list"
          :class="{ 'sec-list-bullet': !sec.ordered }"
        >
          <li v-for="(key, j) in sec.items" :key="j">
            <n-text depth="2" style="font-size: 12.5px; line-height: 1.6">
              {{ t(key) }}
            </n-text>
          </li>
        </component>
      </n-flex>

      <n-flex align="flex-start" :size="8" :wrap="false" class="block privacy">
        <n-icon :size="16" color="#f0a020" style="margin-top: 2px">
          <icon-mdi-shield-check-outline />
        </n-icon>
        <n-text depth="3" style="font-size: 12.5px; line-height: 1.6">
          {{ t("browserExt.privacyNote") }}
        </n-text>
      </n-flex>

      <n-flex vertical :size="6" class="block">
        <n-flex align="center" :size="8">
          <n-icon :size="16" color="#8b5cf6"><icon-mdi-web /></n-icon>
          <n-text strong>{{ t("browserExt.supportedHeading") }}</n-text>
        </n-flex>
        <n-flex :size="6" :wrap="true">
          <n-tag v-for="s in SUPPORTED_SITES" :key="s" size="small" round :bordered="false">
            {{ s }}
          </n-tag>
        </n-flex>
      </n-flex>
    </n-flex>

    <template #action>
      <n-flex justify="space-between" align="center" style="width: 100%">
        <n-button text size="small" @click="openRepo">
          <template #icon>
            <n-icon><icon-mdi-open-in-new /></n-icon>
          </template>
          {{ t("browserExt.viewSource") }}
        </n-button>
        <n-button type="primary" :loading="revealing" @click="revealLocalFolder">
          <template #icon>
            <n-icon><icon-mdi-folder-open-outline /></n-icon>
          </template>
          {{ t("browserExt.openLocalFolder") }}
        </n-button>
      </n-flex>
    </template>
  </n-modal>
</template>

<style scoped lang="scss">
.intro {
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(6, 182, 212, 0.08);

  .intro-icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: rgba(6, 182, 212, 0.12);
  }
}

.block {
  padding: 0 4px;
}

.privacy {
  padding: 8px 12px;
  border-radius: 6px;
  background: rgba(240, 160, 32, 0.06);
}

.sec-list {
  margin: 0;
  padding-left: 22px;

  li {
    margin: 0;
    padding: 2px 0;
  }

  &.sec-list-bullet {
    list-style: disc;
  }
}
</style>

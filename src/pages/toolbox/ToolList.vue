<script setup lang="ts">
import type { Component } from "vue";
import IconMdiImageOutline from "~icons/mdi/image-outline";
import IconMdiSubtitlesOutline from "~icons/mdi/subtitles-outline";
import IconMdiMessageTextOutline from "~icons/mdi/message-text-outline";
import IconMdiPuzzleOutline from "~icons/mdi/puzzle-outline";
import IconMdiOpenInApp from "~icons/mdi/open-in-app";
import { useI18n } from "vue-i18n";

useI18n();
const router = useRouter();

interface ToolItem {
  key: string;
  icon: Component;
  color: string;
  bg: string;
  titleKey: string;
  descKey: string;
  tagKey?: string;
  /** "route" 跳转工具子页（默认）；"modal" 在当前页弹出说明 */
  kind?: "route" | "modal";
}

const tools: ToolItem[] = [
  {
    key: "thumbnail",
    icon: IconMdiImageOutline,
    color: "#18a058",
    bg: "rgba(24,160,88,0.1)",
    titleKey: "toolbox.thumbnailTitle",
    descKey: "toolbox.thumbnailDesc",
  },
  {
    key: "subtitles",
    icon: IconMdiSubtitlesOutline,
    color: "#2080f0",
    bg: "rgba(32,128,240,0.1)",
    titleKey: "toolbox.subtitlesTitle",
    descKey: "toolbox.subtitlesDesc",
  },
  {
    key: "livechat",
    icon: IconMdiMessageTextOutline,
    color: "#f0a020",
    bg: "rgba(240,160,32,0.1)",
    titleKey: "toolbox.livechatTitle",
    descKey: "toolbox.livechatDesc",
    tagKey: "toolbox.youtubeOnly",
  },
  {
    key: "plugins",
    icon: IconMdiPuzzleOutline,
    color: "#8b5cf6",
    bg: "rgba(139,92,246,0.1)",
    titleKey: "plugins.title",
    descKey: "plugins.desc",
  },
  {
    key: "browser-extension",
    icon: IconMdiOpenInApp,
    color: "#06b6d4",
    bg: "rgba(6,182,212,0.1)",
    titleKey: "toolbox.browserExtTitle",
    descKey: "toolbox.browserExtDesc",
    tagKey: "browserExt.tagBeta",
    kind: "modal",
  },
];

const showBrowserExtModal = ref(false);

const handleToolClick = (tool: ToolItem) => {
  if (tool.kind === "modal") {
    if (tool.key === "browser-extension") showBrowserExtModal.value = true;
    return;
  }
  router.push({ name: `toolbox-${tool.key}` });
};
</script>

<template>
  <div class="tools-grid">
    <n-card
      v-for="tool in tools"
      :key="tool.key"
      size="small"
      hoverable
      class="tool-card"
      @click="handleToolClick(tool)"
    >
      <n-flex align="center" :size="10" :wrap="false">
        <div class="tool-icon" :style="{ background: tool.bg }">
          <n-icon :size="20" :color="tool.color">
            <component :is="tool.icon" />
          </n-icon>
        </div>
        <n-flex vertical :size="2" class="tool-info">
          <n-flex align="center" :size="6" :wrap="false">
            <n-text strong class="tool-title">{{ $t(tool.titleKey) }}</n-text>
            <n-tag v-if="tool.tagKey" size="small" round :bordered="false" type="warning">
              {{ $t(tool.tagKey) }}
            </n-tag>
          </n-flex>
          <n-text depth="3" class="tool-desc">{{ $t(tool.descKey) }}</n-text>
        </n-flex>
        <n-icon :size="16" class="tool-arrow" :depth="3">
          <icon-mdi-chevron-right />
        </n-icon>
      </n-flex>
    </n-card>
  </div>

  <BrowserExtensionModal v-model:show="showBrowserExtModal" />
</template>

<style scoped lang="scss">
.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 8px;
}

.tool-card {
  cursor: pointer;
  transition: transform 0.15s;
}

.tool-icon {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.tool-info {
  flex: 1;
  min-width: 0;

  .tool-title {
    flex: 0 1 auto;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tool-desc {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
}

.tool-arrow {
  flex-shrink: 0;
  opacity: 0.4;
  transition: opacity 0.15s, transform 0.15s;
}

.tool-card:hover .tool-arrow {
  opacity: 0.8;
  transform: translateX(2px);
}
</style>

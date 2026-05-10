<script setup lang="ts">
import type { Component } from "vue";
import IconMdiImageOutline from "~icons/mdi/image-outline";
import IconMdiSubtitlesOutline from "~icons/mdi/subtitles-outline";
import IconMdiMessageTextOutline from "~icons/mdi/message-text-outline";
import IconMdiPuzzleOutline from "~icons/mdi/puzzle-outline";
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
];

const navigateTo = (key: string) => {
  router.push({ name: `toolbox-${key}` });
};
</script>

<template>
  <n-flex vertical :size="8">
    <n-card
      v-for="tool in tools"
      :key="tool.key"
      size="small"
      hoverable
      class="tool-card"
      @click="navigateTo(tool.key)"
    >
      <n-flex align="center" :size="12" :wrap="false">
        <div class="tool-icon" :style="{ background: tool.bg }">
          <n-icon :size="20" :color="tool.color">
            <component :is="tool.icon" />
          </n-icon>
        </div>
        <n-flex vertical :size="2" class="tool-info">
          <n-flex align="center" :size="6">
            <n-text strong>{{ $t(tool.titleKey) }}</n-text>
            <n-tag v-if="tool.tagKey" size="small" round :bordered="false" type="warning">
              {{ $t(tool.tagKey) }}
            </n-tag>
          </n-flex>
          <n-text depth="3" class="tool-desc">{{ $t(tool.descKey) }}</n-text>
        </n-flex>
        <n-button type="primary" secondary size="small" @click.stop="navigateTo(tool.key)">
          {{ $t("toolbox.use") }}
          <template #icon>
            <n-icon><icon-mdi-chevron-right /></n-icon>
          </template>
        </n-button>
      </n-flex>
    </n-card>
  </n-flex>
</template>

<style scoped lang="scss">
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

  .tool-desc {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
}
</style>

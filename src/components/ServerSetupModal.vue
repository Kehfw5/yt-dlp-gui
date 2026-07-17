<script setup lang="ts">
import { useSettingStore } from "@/stores/setting";
import { useStatusStore } from "@/stores/status";
import { setServerUrl, testConnection, getServerUrl } from "@/api";

const settingStore = useSettingStore();
const statusStore = useStatusStore();

const serverUrl = ref(settingStore.serverUrl || "http://");
const testing = ref(false);
const testResult = ref<"" | "success" | "error">("");

const handleSave = () => {
  settingStore.serverUrl = serverUrl.value;
  setServerUrl(serverUrl.value);
  statusStore.showServerSetupModal = false;
};

const handleSkip = () => {
  statusStore.showServerSetupModal = false;
};

const handleTest = async () => {
  testing.value = true;
  testResult.value = "";
  try {
    const ok = await testConnection(serverUrl.value);
    testResult.value = ok ? "success" : "error";
  } catch {
    testResult.value = "error";
  } finally {
    testing.value = false;
  }
};

const canClose = computed(() => !!getServerUrl());
</script>

<template>
  <n-modal
    v-model:show="statusStore.showServerSetupModal"
    preset="card"
    :title="$t('setup.serverTitle')"
    size="small"
    :bordered="false"
    :mask-closable="canClose"
    :closable="canClose"
    :style="{ width: '460px' }"
  >
    <n-flex vertical :size="16">
      <n-alert type="warning" :bordered="false">
        {{ $t("setup.serverDesc") }}
      </n-alert>

      <n-flex vertical :size="8">
        <n-text depth="3" style="font-size: 13px">
          {{ $t("settings.serverUrl") }}
        </n-text>
        <n-input
          v-model:value="serverUrl"
          :placeholder="$t('settings.serverUrlPlaceholder')"
          size="medium"
          clearable
        />
      </n-flex>

      <n-flex :size="8">
        <n-button
          :loading="testing"
          strong
          secondary
          size="small"
          @click="handleTest"
        >
          {{ $t("settings.testConnection") }}
        </n-button>
        <n-text
          v-if="testResult === 'success'"
          type="success"
          style="font-size: 13px"
        >
          {{ $t("settings.connectionSuccess") }}
        </n-text>
        <n-text
          v-if="testResult === 'error'"
          type="error"
          style="font-size: 13px"
        >
          {{ $t("settings.connectionFailed") }}
        </n-text>
      </n-flex>

      <n-text depth="3" style="font-size: 12px">
        {{ $t("setup.serverHint") }}
      </n-text>
    </n-flex>

    <template #footer>
      <n-flex justify="end" :size="8">
        <n-button strong secondary @click="handleSkip">
          {{ $t("common.later") }}
        </n-button>
        <n-button type="primary" strong secondary @click="handleSave">
          {{ $t("common.save") }}
        </n-button>
      </n-flex>
    </template>
  </n-modal>
</template>

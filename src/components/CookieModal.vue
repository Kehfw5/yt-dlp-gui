<script setup lang="ts">
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
</script>

<template>
  <n-modal
    v-model:show="statusStore.showCookieModal"
    preset="card"
    :title="$t('cookie.verificationRequired')"
    size="small"
    :bordered="false"
    :style="{ width: '480px' }"
  >
    <n-flex vertical :size="12">
      <n-alert type="warning" :bordered="false">{{ $t("cookie.verificationDesc") }}</n-alert>
      <CookieCard />
    </n-flex>
  </n-modal>
</template>

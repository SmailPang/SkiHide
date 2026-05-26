<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';

defineProps<{
  open: boolean;
  draft: string;
  error: string;
}>();

const emit = defineEmits<{
  'update:draft': [value: string];
  save: [];
  cancel: [];
}>();

const { t } = useI18n();

function openExternalUrl(url: string) { void invoke('open_external_url', { url }); }
</script>

<template>
  <Transition name="dialog-fade">
    <div v-if="open" class="dialog-overlay" @click="emit('cancel')">
      <div class="dialog-panel" @click.stop>
        <div class="dialog-title">{{ t('settings.mirrorChanSdkDialogTitle') }}</div>
        <div class="dialog-description">{{ t('settings.mirrorChanSdkDialogDesc') }}</div>
        <input
          :value="draft"
          class="dialog-input"
          type="text"
          :placeholder="t('settings.mirrorChanSdkPlaceholder')"
          @input="emit('update:draft', ($event.target as HTMLInputElement).value)"
        />
        <div v-if="error" class="dialog-error">{{ error }}</div>
        <button class="dialog-link-button" type="button" @click="openExternalUrl('https://mirrorchyan.com/zh/get-start?source=skihide-client')">
          {{ t('settings.getMirrorChanSdk') }}
        </button>
        <div class="dialog-actions">
          <button class="dialog-action-button primary" type="button" @click="emit('save')">{{ t('common.confirm') }}</button>
          <button class="dialog-action-button secondary" type="button" @click="emit('cancel')">{{ t('common.cancel') }}</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

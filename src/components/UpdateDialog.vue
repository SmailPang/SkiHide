<script setup lang="ts">
import { useI18n } from 'vue-i18n';

const props = defineProps<{
  open: boolean;
  latestVersion: string;
  renderedChangelog: string;
  updateInProgress: boolean;
  updateProgress: number;
  updateClosePromptOpen: boolean;
  mirrorChanSdkConfigured: boolean;
}>();

const emit = defineEmits<{
  close: [];
  'update-now': [];
  'fill-mirror-cdk': [];
  'changelog-link-click': [event: MouseEvent];
}>();

const { t } = useI18n();
</script>

<template>
  <Transition name="dialog-fade">
    <div v-if="open" class="dialog-overlay" @click="emit('close')">
      <div class="dialog-panel update-dialog-panel" @click.stop>
        <div class="dialog-title">{{ t('updateDialog.title') }}</div>
        <div class="update-dialog-version">{{ t('updateDialog.latestVersion') }} {{ latestVersion }}</div>
        <div class="update-dialog-log">
          <div class="update-dialog-log-title">{{ t('updateDialog.changelog') }}</div>
          <div class="update-dialog-markdown" v-html="renderedChangelog" @click="emit('changelog-link-click', $event)" />
        </div>
        <div v-if="updateInProgress || updateProgress > 0" class="update-progress-block">
          <div class="update-progress-track">
            <div class="update-progress-fill" :style="{ width: `${updateProgress}%` }" />
          </div>
          <div class="update-progress-text">{{ t('updateDialog.downloading') }} {{ updateProgress }}%</div>
        </div>
        <button v-if="!mirrorChanSdkConfigured" class="update-dialog-sdk-button" type="button" @click="emit('fill-mirror-cdk')">
          {{ t('updateDialog.fillMirrorChanSdk') }}
        </button>
        <div class="dialog-actions">
          <button class="dialog-action-button primary" type="button" :disabled="updateInProgress || updateClosePromptOpen" @click="emit('update-now')">
            {{ updateInProgress ? t('updateDialog.updatingNow') : t('updateDialog.updateNow') }}
          </button>
          <button class="dialog-action-button secondary" type="button" :disabled="updateInProgress || updateClosePromptOpen" @click="emit('close')">
            {{ t('updateDialog.cancelUpdate') }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

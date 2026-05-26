<script setup lang="ts">
import { useI18n } from 'vue-i18n';

defineProps<{ open: boolean }>();
const emit = defineEmits<{ accept: []; reject: []; 'open-privacy-policy': [] }>();
const { t } = useI18n();
</script>

<template>
  <Transition name="dialog-fade">
    <div v-if="open" class="dialog-overlay" @click.stop>
      <div class="dialog-panel privacy-dialog-panel" @click.stop>
        <div class="dialog-title">{{ t('privacyDialog.title') }}</div>
        <div class="dialog-description">
          {{ t('privacyDialog.prefix') }}
          <button class="privacy-inline-link" type="button" @click="emit('open-privacy-policy')">{{ t('privacyDialog.policyAndDisclaimer') }}</button>
          {{ t('privacyDialog.suffix') }}
        </div>
        <div class="dialog-actions">
          <button class="dialog-action-button primary" type="button" @click="emit('accept')">{{ t('privacyDialog.accept') }}</button>
          <button class="dialog-action-button secondary" type="button" @click="emit('reject')">{{ t('privacyDialog.reject') }}</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';

defineProps<{ open: boolean }>();
const emit = defineEmits<{ confirm: []; cancel: [] }>();
const { t } = useI18n();
</script>

<template>
  <Transition name="dialog-fade">
    <div v-if="open" class="dialog-overlay" @click.stop>
      <div class="dialog-panel" @click.stop>
        <div class="dialog-title">{{ t('toolbox.cacheConfirmTitle') }}</div>
        <div class="dialog-description">{{ t('toolbox.cacheConfirmDesc') }}</div>
        <div class="dialog-actions">
          <button class="dialog-action-button primary" type="button" @click="emit('confirm')">{{ t('toolbox.cacheConfirmAction') }}</button>
          <button class="dialog-action-button secondary" type="button" @click="emit('cancel')">{{ t('common.cancel') }}</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

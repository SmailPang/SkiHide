<script setup lang="ts">
import { useI18n } from 'vue-i18n';

defineProps<{ open: boolean }>();
const emit = defineEmits<{ continue: [] }>();
const { t } = useI18n();
</script>

<template>
  <Transition name="dialog-fade">
    <div v-if="open" class="dialog-overlay" @click.stop>
      <div class="dialog-panel" @click.stop>
        <div class="dialog-title">{{ t('toolbox.dangerTitle') }}</div>
        <div class="dialog-description dialog-description-preline">{{ t('toolbox.dangerWarning') }}</div>
        <div class="dialog-actions dialog-actions-triple">
          <button class="dialog-action-button danger" type="button" @click="emit('continue')">{{ t('toolbox.dangerContinue') }}</button>
          <button class="dialog-action-button secondary" type="button" @click="emit('continue')">{{ t('toolbox.dangerContinue') }}</button>
          <button class="dialog-action-button secondary" type="button" @click="emit('continue')">{{ t('toolbox.dangerContinue') }}</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

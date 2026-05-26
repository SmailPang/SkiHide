<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { WindowInfo } from '../types';

const FOREGROUND_WINDOW_HWND = 0;

const props = defineProps<{
  windows: WindowInfo[];
  selectedWindowId: number | null;
  isListening: boolean;
  listenSettingsOpen: boolean;
  listenHotkey: string;
  listenMouseSideButton: boolean;
  autoListenOnStartup: boolean;
  recordingHotkey: boolean;
  listenSettingsError: string;
  windowsLoading: boolean;
}>();

const emit = defineEmits<{
  'select-window': [hwnd: number];
  'select-foreground-window': [];
  'toggle-listening': [];
  'toggle-listen-settings': [];
  'close-listen-settings': [];
  'toggle-hotkey-recording': [];
  'clear-hotkey': [];
  'toggle-mouse-side-button': [];
  'toggle-auto-listen-on-startup': [];
  'refresh': [];
}>();

const { t } = useI18n();

const hotkeyButtonLabel = computed(() =>
  props.recordingHotkey ? t('home.recordingHotkey') : props.listenHotkey || t('home.bindHotkey'),
);
</script>

<template>
  <section class="page-card home-page">
    <button
      v-if="listenSettingsOpen"
      class="home-overlay"
      type="button"
      :aria-label="t('home.closeListenSettings')"
      @click="emit('close-listen-settings')"
    />
    <div class="home-actions">
      <div class="window-list-panel">
        <div class="window-list-header">{{ t('home.windowListTitle') }}</div>
        <div class="window-list-card">
          <button
            :class="['window-list-item', 'foreground-window-item', { active: selectedWindowId === FOREGROUND_WINDOW_HWND }]"
            type="button"
            @click="emit('select-foreground-window')"
          >
            <span class="window-list-name">{{ t('home.currentForegroundWindow') }}</span>
            <span class="window-list-pid">{{ t('home.currentForegroundWindow') }}</span>
          </button>
          <button
            v-for="item in windows"
            :key="item.hwnd"
            :class="['window-list-item', { active: item.hwnd === selectedWindowId }]"
            type="button"
            @click="emit('select-window', item.hwnd)"
          >
            <span class="window-list-name">{{ item.title }}</span>
            <span class="window-list-pid">PID {{ item.hwnd }}</span>
          </button>
          <div v-if="!windows.length && !windowsLoading" class="window-list-empty">{{ t('home.noWindows') }}</div>
          <div v-if="windowsLoading" class="window-list-empty">{{ t('home.loadingWindows') }}</div>
        </div>
      </div>
      <div class="home-secondary-actions">
        <button
          :class="['listen-settings-button', { active: listenSettingsOpen }]"
          type="button"
          @click.stop="emit('toggle-listen-settings')"
        >{{ t('home.listenSettings') }}</button>
        <button class="window-refresh-button" type="button" @click="emit('refresh')">{{ t('home.refresh') }}</button>
      </div>
      <button
        :class="['listen-button', { listening: isListening }]"
        type="button"
        @click="emit('toggle-listening')"
      >
        <span class="listen-button-text">{{ isListening ? t('home.stopListening') : t('home.startListening') }}</span>
      </button>
    </div>

    <Transition name="listen-settings-popup">
      <div v-if="listenSettingsOpen" class="listen-settings-popup" @click.stop>
        <div class="listen-settings-group">
          <span class="listen-settings-label">{{ t('home.hotkey') }}</span>
          <div class="listen-hotkey-actions">
            <button
              :class="['listen-hotkey-trigger', { recording: recordingHotkey }]"
              type="button"
              @click="emit('toggle-hotkey-recording')"
            >{{ hotkeyButtonLabel }}</button>
            <button v-if="listenHotkey" class="listen-hotkey-clear" type="button" @click="emit('clear-hotkey')">
              {{ t('common.clear') }}
            </button>
          </div>
        </div>
        <label class="listen-checkbox-row" @click.prevent="emit('toggle-mouse-side-button')">
          <input :checked="listenMouseSideButton" class="listen-checkbox-input" type="checkbox" />
          <span class="listen-checkbox-box" />
          <span class="listen-checkbox-label">{{ t('home.mouseSideButton') }}</span>
        </label>
        <label class="listen-checkbox-row" @click.prevent="emit('toggle-auto-listen-on-startup')">
          <input :checked="autoListenOnStartup" class="listen-checkbox-input" type="checkbox" />
          <span class="listen-checkbox-box" />
          <span class="listen-checkbox-label">
            {{ t('home.autoListenOnStartup') }}
            <span class="settings-hint" tabindex="0" @click.stop>
              <span class="settings-hint-icon" aria-hidden="true">i</span>
              <span class="settings-hint-tooltip">{{ t('home.autoListenOnStartupHint') }}</span>
            </span>
          </span>
        </label>
        <p v-if="listenSettingsError" class="listen-settings-error">{{ listenSettingsError }}</p>
      </div>
    </Transition>
  </section>
</template>

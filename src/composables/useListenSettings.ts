import { ref, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import type { AppConfig } from '../types';
import type { NoticeType } from './useNotify';

type NotifyFn = (options: { title: string; content?: string; type: NoticeType }) => void;

const FOREGROUND_WINDOW_HWND = 0;

export function useListenSettings(opts: {
  notify: NotifyFn;
  selectedHomeWindowId: Ref<number | null>;
}) {
  const { notify, selectedHomeWindowId } = opts;
  const { t } = useI18n();

  const listenHotkey = ref('');
  const listenMouseSideButton = ref(false);
  const autoListenOnStartup = ref(false);
  const isListening = ref(false);
  const listenSettingsOpen = ref(false);
  const recordingHotkey = ref(false);
  const listenSettingsError = ref('');

  async function syncHotkeyWhileListening() {
    const hasHotkey = listenHotkey.value.trim().length > 0;
    const listenerEnabled = hasHotkey || listenMouseSideButton.value;
    try {
      await invoke<AppConfig>('update_config', {
        patch: {
          hotkey: listenHotkey.value.trim(),
          language: null,
          last_selected_hwnd: selectedHomeWindowId.value,
          mouse_side_button_listener: listenMouseSideButton.value,
          auto_listen_on_startup: autoListenOnStartup.value,
        },
      });
      if (isListening.value) {
        await invoke('set_hotkey_enabled', { enabled: listenerEnabled });
      }
    } catch (error) {
      listenSettingsError.value = String(error);
    }
  }

  async function setHotkeyEnabledState(enabled: boolean) {
    try {
      await invoke('set_hotkey_enabled', { enabled });
    } catch (error) {
      listenSettingsError.value = String(error);
    }
  }

  async function toggleListening() {
    if (isListening.value) {
      try {
        await invoke('set_hotkey_enabled', { enabled: false });
        isListening.value = false;
        listenSettingsError.value = '';
      } catch (error) {
        listenSettingsError.value = String(error);
        listenSettingsOpen.value = true;
      }
      return;
    }
    const hasHotkey = listenHotkey.value.trim().length > 0;
    const listenerEnabled = hasHotkey || listenMouseSideButton.value;
    if (!listenerEnabled) {
      listenSettingsOpen.value = true;
      listenSettingsError.value = t('home.requireListenerConfig');
      return;
    }
    try {
      await invoke<AppConfig>('update_config', {
        patch: {
          hotkey: listenHotkey.value.trim(),
          language: null,
          last_selected_hwnd: selectedHomeWindowId.value,
          mouse_side_button_listener: listenMouseSideButton.value,
        },
      });
      await invoke('set_hotkey_enabled', { enabled: listenerEnabled });
      listenSettingsError.value = '';
      isListening.value = true;
    } catch (error) {
      listenSettingsOpen.value = true;
      listenSettingsError.value = String(error);
    }
  }

  function toggleListenSettings() {
    listenSettingsOpen.value = !listenSettingsOpen.value;
    if (!listenSettingsOpen.value) {
      recordingHotkey.value = false;
      void syncHotkeyWhileListening();
      listenSettingsError.value = '';
    }
  }

  function closeListenSettings() {
    listenSettingsOpen.value = false;
    recordingHotkey.value = false;
    void syncHotkeyWhileListening();
    listenSettingsError.value = '';
  }

  function toggleHotkeyRecording() {
    const nextRecording = !recordingHotkey.value;
    recordingHotkey.value = nextRecording;
    if (nextRecording) {
      listenSettingsError.value = '';
      listenSettingsOpen.value = true;
      if (isListening.value) void setHotkeyEnabledState(false);
      return;
    }
    void syncHotkeyWhileListening();
  }

  function clearHotkey() {
    if (isListening.value && !listenMouseSideButton.value) {
      listenSettingsOpen.value = true;
      listenSettingsError.value = t('home.requireAtLeastOneListener');
      return;
    }
    listenHotkey.value = '';
    recordingHotkey.value = false;
    listenSettingsError.value = '';
    void syncHotkeyWhileListening();
  }

  function toggleMouseSideButton() {
    const nextValue = !listenMouseSideButton.value;
    if (isListening.value && !nextValue && listenHotkey.value.trim().length === 0) {
      listenSettingsOpen.value = true;
      listenSettingsError.value = t('home.requireAtLeastOneListener');
      return;
    }
    listenMouseSideButton.value = nextValue;
    listenSettingsError.value = '';
    void syncHotkeyWhileListening();
  }

  function toggleAutoListenOnStartup() {
    autoListenOnStartup.value = !autoListenOnStartup.value;
    void syncHotkeyWhileListening();
  }

  async function initListeningState() {
    if (autoListenOnStartup.value) {
      const hasHotkey = listenHotkey.value.trim().length > 0;
      const hasMouseListener = listenMouseSideButton.value;
      if (hasHotkey || hasMouseListener) {
        if (selectedHomeWindowId.value === null) {
          selectedHomeWindowId.value = FOREGROUND_WINDOW_HWND;
          await invoke<AppConfig>('update_config', {
            patch: { last_selected_hwnd: FOREGROUND_WINDOW_HWND },
          });
        }
        isListening.value = true;
      } else {
        await invoke('set_hotkey_enabled', { enabled: false });
      }
    } else {
      await invoke('set_hotkey_enabled', { enabled: false });
    }
  }

  return {
    listenHotkey,
    listenMouseSideButton,
    autoListenOnStartup,
    isListening,
    listenSettingsOpen,
    recordingHotkey,
    listenSettingsError,
    syncHotkeyWhileListening,
    setHotkeyEnabledState,
    toggleListening,
    toggleListenSettings,
    closeListenSettings,
    toggleHotkeyRecording,
    clearHotkey,
    toggleMouseSideButton,
    toggleAutoListenOnStartup,
    initListeningState,
  };
}

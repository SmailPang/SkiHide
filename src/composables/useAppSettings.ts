import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import type { AppConfig } from '../types';
import type { NoticeType } from './useNotify';

type NotifyFn = (options: { title: string; content?: string; type: NoticeType }) => void;
type AnimateThemeFn = (theme: 'light' | 'dark') => void;
type ResolveThemeFn = (mode: string) => 'light' | 'dark';

export type LanguageValue = 'zh_CN' | 'zh_TW' | 'en_US' | 'ja_JP';

const languageOptionValues: LanguageValue[] = ['zh_CN', 'zh_TW', 'en_US', 'ja_JP'];
const themeOptionValues = ['system', 'light', 'dark'] as const;
const fontSizeOptionValues = ['small', 'medium', 'large', 'xlarge'] as const;
const updateSourceOptionValues = ['mirror_chan', 'skihide', 'cloudflare'] as const;
const updateChannelOptionValues = ['stable', 'beta'] as const;
const downloadSourceOptionValues = ['mirror_chan', 'cloudflare', 'github', 'cnb'] as const;

export {
  languageOptionValues,
  themeOptionValues,
  fontSizeOptionValues,
  updateSourceOptionValues,
  updateChannelOptionValues,
  downloadSourceOptionValues,
};

export function mapMirrorError(code: number, message: string | null): string {
  switch (code) {
    case 7001: return 'Mirror酱 CDK 已过期';
    case 7002: return 'Mirror酱 CDK 错误';
    case 7003: return 'Mirror酱 CDK 今日下载次数已达上限';
    case 7004: return 'Mirror酱 CDK 与当前资源不匹配';
    case 7005: return 'Mirror酱 CDK 已被封禁';
    case 8001: return 'Mirror酱 资源不存在';
    case 8002: return 'Mirror酱 系统参数无效';
    case 8003: return 'Mirror酱 架构参数无效';
    case 8004: return 'Mirror酱 更新通道参数无效';
    default: return message || `Mirror酱返回错误码 ${code}`;
  }
}

export function useAppSettings(opts: {
  notify: NotifyFn;
  animateThemeChange: AnimateThemeFn;
  resolveTheme: ResolveThemeFn;
}) {
  const { notify, animateThemeChange, resolveTheme } = opts;
  const { t, locale } = useI18n();

  const language = ref<LanguageValue>('zh_CN');
  const savedLanguage = ref<LanguageValue>('zh_CN');
  const theme = ref<'system' | 'light' | 'dark'>('system');
  const savedTheme = ref<'system' | 'light' | 'dark'>('system');
  const fontFamily = ref('');
  const savedFontFamily = ref('');
  const systemFonts = ref<string[]>([]);
  const fontSize = ref<'small' | 'medium' | 'large' | 'xlarge'>('medium');
  const savedFontSize = ref<'small' | 'medium' | 'large' | 'xlarge'>('medium');
  const autoStart = ref(false);
  const savedAutoStart = ref(false);
  const silentStart = ref(false);
  const savedSilentStart = ref(false);
  const muteOnHide = ref(false);
  const savedMuteOnHide = ref(false);
  const pauseOnHide = ref(false);
  const savedPauseOnHide = ref(false);
  const pauseHotkey = ref('');
  const savedPauseHotkey = ref('');
  const pauseHotkeyError = ref('');
  const recordingPauseHotkey = ref(false);
  const updateSource = ref<'mirror_chan' | 'skihide' | 'cloudflare'>('mirror_chan');
  const savedUpdateSource = ref<'mirror_chan' | 'skihide' | 'cloudflare'>('mirror_chan');
  const updateChannel = ref<'stable' | 'beta'>('stable');
  const savedUpdateChannel = ref<'stable' | 'beta'>('stable');
  const mirrorChanSdk = ref('');
  const savedMirrorChanSdk = ref('');
  const downloadSource = ref<'mirror_chan' | 'cloudflare' | 'github' | 'cnb'>('cnb');
  const savedDownloadSource = ref<'mirror_chan' | 'cloudflare' | 'github' | 'cnb'>('cnb');
  const autoCheckUpdates = ref(true);
  const savedAutoCheckUpdates = ref(true);
  const autoListenOnStartup = ref(false);
  const savedAutoListenOnStartup = ref(false);
  const privacyConsentAccepted = ref(false);

  const mirrorChanSdkConfigured = computed(() => mirrorChanSdk.value.trim().length > 0);
  const silentStartDisabled = computed(() => !autoStart.value);
  const settingsDirty = computed(
    () =>
      language.value !== savedLanguage.value ||
      theme.value !== savedTheme.value ||
      fontFamily.value !== savedFontFamily.value ||
      fontSize.value !== savedFontSize.value ||
      autoStart.value !== savedAutoStart.value ||
      silentStart.value !== savedSilentStart.value ||
      muteOnHide.value !== savedMuteOnHide.value ||
      pauseOnHide.value !== savedPauseOnHide.value ||
      pauseHotkey.value !== savedPauseHotkey.value ||
      updateSource.value !== savedUpdateSource.value ||
      updateChannel.value !== savedUpdateChannel.value ||
      mirrorChanSdk.value !== savedMirrorChanSdk.value ||
      downloadSource.value !== savedDownloadSource.value ||
      autoCheckUpdates.value !== savedAutoCheckUpdates.value ||
      autoListenOnStartup.value !== savedAutoListenOnStartup.value,
  );
  const fontScale = computed(() => {
    switch (fontSize.value) {
      case 'small': return 0.92;
      case 'large': return 1.08;
      case 'xlarge': return 1.16;
      default: return 1;
    }
  });

  async function loadSystemFonts() {
    try {
      systemFonts.value = await invoke<string[]>('list_system_fonts');
    } catch (error) {
      notify({ title: t('home.initFailed'), content: String(error), type: 'false' });
    }
  }

  async function loadConfigFromBackend() {
    const config = await invoke<AppConfig>('get_config');

    const nextLanguage = languageOptionValues.includes(config.language as LanguageValue)
      ? (config.language as LanguageValue)
      : 'zh_CN';
    language.value = nextLanguage;
    savedLanguage.value = nextLanguage;
    locale.value = nextLanguage;

    const nextTheme = themeOptionValues.includes(config.theme as 'system' | 'light' | 'dark')
      ? (config.theme as 'system' | 'light' | 'dark')
      : 'system';
    theme.value = nextTheme;
    savedTheme.value = nextTheme;
    animateThemeChange(resolveTheme(nextTheme));

    const nextFontFamily = config.font_family?.trim() ?? '';
    fontFamily.value = nextFontFamily;
    savedFontFamily.value = nextFontFamily;

    const nextFontSize = fontSizeOptionValues.includes(config.font_size as typeof fontSizeOptionValues[number])
      ? (config.font_size as typeof fontSizeOptionValues[number])
      : 'medium';
    fontSize.value = nextFontSize;
    savedFontSize.value = nextFontSize;

    const nextAutoStart = Boolean(config.auto_start);
    const nextSilentStart = nextAutoStart ? Boolean(config.silent_start) : false;
    autoStart.value = nextAutoStart;
    savedAutoStart.value = nextAutoStart;
    silentStart.value = nextSilentStart;
    savedSilentStart.value = nextSilentStart;

    muteOnHide.value = Boolean(config.mute_on_hide);
    savedMuteOnHide.value = muteOnHide.value;
    pauseOnHide.value = Boolean(config.pause_on_hide);
    savedPauseOnHide.value = pauseOnHide.value;
    pauseHotkey.value = config.pause_hotkey ?? '';
    savedPauseHotkey.value = pauseHotkey.value;
    pauseHotkeyError.value = '';

    const nextUpdateSource = updateSourceOptionValues.includes(config.update_source as typeof updateSourceOptionValues[number])
      ? (config.update_source as typeof updateSourceOptionValues[number])
      : 'mirror_chan';
    updateSource.value = nextUpdateSource;
    savedUpdateSource.value = nextUpdateSource;

    const nextUpdateChannel = updateChannelOptionValues.includes(config.update_channel as typeof updateChannelOptionValues[number])
      ? (config.update_channel as typeof updateChannelOptionValues[number])
      : 'stable';
    updateChannel.value = nextUpdateChannel;
    savedUpdateChannel.value = nextUpdateChannel;

    mirrorChanSdk.value = config.mirror_chan_sdk ?? '';
    savedMirrorChanSdk.value = mirrorChanSdk.value;

    const migratedDownloadSource = config.download_source === 'rainyun_cdn' ? 'cnb' : config.download_source;
    const nextDownloadSource = downloadSourceOptionValues.includes(migratedDownloadSource as typeof downloadSourceOptionValues[number])
      ? (migratedDownloadSource as typeof downloadSourceOptionValues[number])
      : 'cnb';
    const normalizedDownloadSource =
      nextDownloadSource === 'mirror_chan' && !mirrorChanSdk.value.trim() ? 'cnb' : nextDownloadSource;

    if (nextUpdateChannel === 'beta') {
      if (nextUpdateSource === 'skihide') updateSource.value = 'mirror_chan';
      downloadSource.value = normalizedDownloadSource;
    } else {
      downloadSource.value = normalizedDownloadSource;
    }
    savedDownloadSource.value = downloadSource.value;

    autoCheckUpdates.value = config.auto_check_updates ?? true;
    savedAutoCheckUpdates.value = autoCheckUpdates.value;
    autoListenOnStartup.value = config.auto_listen_on_startup ?? false;
    savedAutoListenOnStartup.value = autoListenOnStartup.value;
    listenHotkeyRef.value = config.hotkey ?? '';
    listenMouseSideButtonRef.value = Boolean(config.mouse_side_button_listener);
    privacyConsentAccepted.value = Boolean(config.privacy_consent);
  }

  // These are cross-composable refs that need to be readable by saveSettings.
  // Owned by useListenSettings but accessible here via passed refs.
  const listenHotkeyRef = ref('');
  const listenMouseSideButtonRef = ref(false);

  async function saveSettings(opts: {
    listenHotkey: string;
    selectedWindowId: number | null;
    listenMouseSideButton: boolean;
  }) {
    try {
      if (!mirrorChanSdkConfigured.value && downloadSource.value === 'mirror_chan') {
        downloadSource.value = 'cnb';
      }
      if (pauseOnHide.value && !pauseHotkey.value.trim()) {
        pauseHotkeyError.value = t('settings.pauseHotkeyRequired');
        notify({ title: t('common.saveFailed'), content: pauseHotkeyError.value, type: 'warn' });
        return;
      }
      const normalizedSilentStart = autoStart.value ? silentStart.value : false;
      silentStart.value = normalizedSilentStart;
      await invoke<AppConfig>('update_config', {
        patch: {
          hotkey: opts.listenHotkey,
          language: language.value,
          last_selected_hwnd: opts.selectedWindowId,
          theme: theme.value,
          font_family: fontFamily.value,
          font_size: fontSize.value,
          auto_start: autoStart.value,
          silent_start: normalizedSilentStart,
          mute_on_hide: muteOnHide.value,
          pause_on_hide: pauseOnHide.value,
          pause_hotkey: pauseHotkey.value.trim(),
          update_source: updateSource.value,
          update_channel: updateChannel.value,
          download_source: downloadSource.value,
          mirror_chan_sdk: mirrorChanSdk.value,
          auto_check_updates: autoCheckUpdates.value,
          mouse_side_button_listener: opts.listenMouseSideButton,
        },
      });
      savedLanguage.value = language.value;
      savedTheme.value = theme.value;
      savedFontFamily.value = fontFamily.value;
      savedFontSize.value = fontSize.value;
      savedAutoStart.value = autoStart.value;
      savedSilentStart.value = normalizedSilentStart;
      silentStart.value = normalizedSilentStart;
      savedMuteOnHide.value = muteOnHide.value;
      savedPauseOnHide.value = pauseOnHide.value;
      pauseHotkey.value = pauseHotkey.value.trim();
      savedPauseHotkey.value = pauseHotkey.value;
      pauseHotkeyError.value = '';
      recordingPauseHotkey.value = false;
      savedUpdateSource.value = updateSource.value;
      savedUpdateChannel.value = updateChannel.value;
      savedMirrorChanSdk.value = mirrorChanSdk.value;
      savedDownloadSource.value = downloadSource.value;
      savedAutoCheckUpdates.value = autoCheckUpdates.value;
      notify({ title: t('common.saveSuccess'), type: 'true' });
    } catch (error) {
      notify({ title: t('common.saveFailed'), content: String(error), type: 'false' });
    }
  }

  function discardSettings() {
    language.value = savedLanguage.value;
    theme.value = savedTheme.value;
    fontFamily.value = savedFontFamily.value;
    fontSize.value = savedFontSize.value;
    autoStart.value = savedAutoStart.value;
    silentStart.value = savedSilentStart.value;
    muteOnHide.value = savedMuteOnHide.value;
    pauseOnHide.value = savedPauseOnHide.value;
    pauseHotkey.value = savedPauseHotkey.value;
    pauseHotkeyError.value = '';
    recordingPauseHotkey.value = false;
    updateSource.value = savedUpdateSource.value;
    updateChannel.value = savedUpdateChannel.value;
    mirrorChanSdk.value = savedMirrorChanSdk.value;
    downloadSource.value = savedDownloadSource.value;
    autoCheckUpdates.value = savedAutoCheckUpdates.value;
    if (!mirrorChanSdkConfigured.value && downloadSource.value === 'mirror_chan') {
      downloadSource.value = 'cnb';
    }
    locale.value = savedLanguage.value;
    animateThemeChange(resolveTheme(savedTheme.value));
  }

  function toggleAutoStart() {
    autoStart.value = !autoStart.value;
    if (!autoStart.value) silentStart.value = false;
  }
  function toggleSilentStart() {
    if (!autoStart.value) return;
    silentStart.value = !silentStart.value;
  }
  function togglePauseOnHide() {
    pauseOnHide.value = !pauseOnHide.value;
    if (pauseOnHide.value) pauseHotkeyError.value = '';
  }
  function togglePauseHotkeyRecording() {
    recordingPauseHotkey.value = !recordingPauseHotkey.value;
    if (recordingPauseHotkey.value) pauseHotkeyError.value = '';
  }
  function clearPauseHotkey() {
    pauseHotkey.value = '';
    recordingPauseHotkey.value = false;
    pauseHotkeyError.value = '';
  }
  function selectUpdateChannel(value: 'stable' | 'beta') {
    updateChannel.value = value;
    if (value === 'beta' && updateSource.value === 'skihide') updateSource.value = 'mirror_chan';
  }

  return {
    language,
    savedLanguage,
    theme,
    savedTheme,
    fontFamily,
    savedFontFamily,
    systemFonts,
    fontSize,
    savedFontSize,
    autoStart,
    silentStart,
    muteOnHide,
    pauseOnHide,
    pauseHotkey,
    pauseHotkeyError,
    recordingPauseHotkey,
    updateSource,
    updateChannel,
    mirrorChanSdk,
    savedMirrorChanSdk,
    downloadSource,
    autoCheckUpdates,
    autoListenOnStartup,
    privacyConsentAccepted,
    mirrorChanSdkConfigured,
    silentStartDisabled,
    settingsDirty,
    fontScale,
    // cross-refs loaded from config (shared with useListenSettings via loadConfigFromBackend)
    listenHotkeyRef,
    listenMouseSideButtonRef,
    loadConfigFromBackend,
    loadSystemFonts,
    saveSettings,
    discardSettings,
    toggleAutoStart,
    toggleSilentStart,
    togglePauseOnHide,
    togglePauseHotkeyRecording,
    clearPauseHotkey,
    selectUpdateChannel,
  };
}

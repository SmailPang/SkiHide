<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import HomePageView from './components/HomePageView.vue';
import ToolboxPageView from './components/ToolboxPageView.vue';
import SettingsPageView from './components/SettingsPageView.vue';
import PrivacyDialog from './components/PrivacyDialog.vue';
import MirrorCdkDialog from './components/MirrorCdkDialog.vue';
import UpdateDialog from './components/UpdateDialog.vue';
import DangerDialog from './components/DangerDialog.vue';

import { useNotify } from './composables/useNotify';
import { useTheme } from './composables/useTheme';
import { useMarkdown } from './composables/useMarkdown';
import { useAppSettings, mapMirrorError } from './composables/useAppSettings';
import { useListenSettings } from './composables/useListenSettings';
import { useMemoryCleanup } from './composables/useMemoryCleanup';
import { useCacheCleanup } from './composables/useCacheCleanup';

import type { AppConfig, MirrorCdkValidationInfo, MirrorDownloadInfo, UpdateCheckInfo, UpdateDownloadResult, WindowInfo } from './types';

type PageKey = 'home' | 'toolbox' | 'settings';

const BUILTIN_FONT_STACK = '"HarmonyOS Sans SC", "Microsoft YaHei UI", "Segoe UI", sans-serif';
const OPEN_SETTINGS_EVENT = 'skihide://open-settings';
const UPDATE_DOWNLOAD_PROGRESS_EVENT = 'skihide://update-download-progress';
const PRIVACY_POLICY_URL = 'https://skihide.xyz/guide/privacy';

const { t, locale } = useI18n();

// ── Composables ──────────────────────────────────────────────────────────────
const { notices, notify, removeSaveNotice, clearAll: clearNotices } = useNotify();

const settingsRef = ref<InstanceType<typeof SettingsPageView> | null>(null);
const {
  appShellRef, themeRipple, activeTheme, renderedTheme,
  resolveTheme, animateThemeChange, handleThemeRippleEnd, init: initTheme, cleanup: cleanupTheme,
} = useTheme(() => settingsRef.value?.themeTriggerRef ?? null);

const settings = useAppSettings({ notify, animateThemeChange, resolveTheme });
const { renderSimpleMarkdown } = useMarkdown();

// selectedHomeWindowId lives here because it is shared between home page, listen settings, and settings save
const selectedHomeWindowId = ref<number | null>(null);
const FOREGROUND_WINDOW_HWND = 0;

const listenSettings = useListenSettings({ notify, selectedHomeWindowId });
const memory = useMemoryCleanup(notify);
const cache = useCacheCleanup(notify);

// ── App-level state ───────────────────────────────────────────────────────────
const currentPage = ref<PageKey>('home');
const homeWindows = ref<WindowInfo[]>([]);
const windowsLoading = ref(false);
const appVersion = ref('');
const startupUpdateChecked = ref(false);

// Update dialog state
const latestVersion = ref('');
const updateChangelogMarkdown = ref('');
const updateDownloadUrl = ref('');
const updateDownloadCandidates = ref<string[]>([]);
const updateExpectedSha256 = ref('');
const downloadedUpdatePath = ref('');
const updateProgress = ref(0);
const updateInProgress = ref(false);
const updateDialogOpen = ref(false);
const updateClosePromptOpen = ref(false);

// MirrorCDK dialog state
const mirrorChanSdkDialogOpen = ref(false);
const mirrorChanSdkDraft = ref('');
const mirrorChanSdkError = ref('');
const returnToUpdateAfterMirrorDialog = ref(false);

// Danger dialog
const dangerDialogOpen = ref(false);

let openSettingsUnlisten: UnlistenFn | null = null;
let updateProgressUnlisten: UnlistenFn | null = null;

// ── Computed ──────────────────────────────────────────────────────────────────
const fontScale = computed(() => settings.fontScale.value);
const appFontFamilyCss = computed(() => {
  const name = settings.fontFamily.value.trim();
  if (!name) return BUILTIN_FONT_STACK;
  const escaped = name.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return `"${escaped}", "Microsoft YaHei UI", "Segoe UI", sans-serif`;
});
const renderedUpdateChangelog = computed(() => renderSimpleMarkdown(updateChangelogMarkdown.value || ''));

// ── Font application ──────────────────────────────────────────────────────────
function applyAppFontFamily() {
  const stack = appFontFamilyCss.value;
  document.documentElement.style.setProperty('--app-font-family', stack);
  if (appShellRef.value) appShellRef.value.style.setProperty('--app-font-family', stack);
}
watch(appFontFamilyCss, applyAppFontFamily, { immediate: true });

// ── Window/listening watchers ─────────────────────────────────────────────────
watch([memory.memoryAutoCleanup, memory.memoryCleanupInterval, memory.memoryCleanupUnit], () => {
  memory.scheduleMemoryCleanup();
});

// ── Home page actions ─────────────────────────────────────────────────────────
async function refreshHomeWindows() {
  if (windowsLoading.value) return;
  windowsLoading.value = true;
  try {
    const windows = await invoke<WindowInfo[]>('list_windows');
    homeWindows.value = windows;
    if (selectedHomeWindowId.value !== null && !windows.some((w) => w.hwnd === selectedHomeWindowId.value)) {
      selectedHomeWindowId.value = null;
    }
  } catch (error) {
    notify({ title: t('home.refreshFailed'), content: String(error), type: 'false' });
  } finally {
    windowsLoading.value = false;
  }
}

async function selectHomeWindow(hwnd: number) {
  selectedHomeWindowId.value = hwnd;
  try {
    await invoke<AppConfig>('update_config', { patch: { hotkey: null, language: null, last_selected_hwnd: hwnd } });
  } catch (error) {
    notify({ title: t('home.saveSelectedWindowFailed'), content: String(error), type: 'warn' });
  }
}

async function selectForegroundWindow() {
  selectedHomeWindowId.value = FOREGROUND_WINDOW_HWND;
  try {
    await invoke<AppConfig>('update_config', { patch: { hotkey: null, language: null, last_selected_hwnd: FOREGROUND_WINDOW_HWND } });
  } catch (error) {
    notify({ title: t('home.saveSelectedWindowFailed'), content: String(error), type: 'warn' });
  }
}

// ── Keyboard handlers ─────────────────────────────────────────────────────────
function formatHotkeyFromEvent(event: KeyboardEvent): string {
  const parts: string[] = [];
  let modifierCount = 0;
  if (event.ctrlKey) { parts.push('Ctrl'); modifierCount++; }
  if (event.altKey) { parts.push('Alt'); modifierCount++; }
  if (event.shiftKey) { parts.push('Shift'); modifierCount++; }
  if (event.metaKey) { parts.push('Win'); modifierCount++; }
  const ignored = ['Control', 'Shift', 'Alt', 'Meta'];
  let terminalKey = '';
  if (!ignored.includes(event.key)) {
    terminalKey = event.key.length === 1 ? event.key.toUpperCase() : event.key;
    parts.push(terminalKey);
  }
  if (modifierCount === 0 || !terminalKey) return '';
  return parts.join('+');
}

function formatPauseHotkeyFromEvent(event: KeyboardEvent): string {
  const parts: string[] = [];
  let hasTerminalKey = false;
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Win');
  const ignored = ['Control', 'Shift', 'Alt', 'Meta'];
  if (!ignored.includes(event.key)) {
    parts.push(event.key.length === 1 ? event.key.toUpperCase() : event.key);
    hasTerminalKey = true;
  }
  return hasTerminalKey ? parts.join('+') : '';
}

function handleHotkeyRecord(event: KeyboardEvent) {
  if (!listenSettings.recordingHotkey.value && !settings.recordingPauseHotkey.value) return;
  event.preventDefault();
  event.stopPropagation();

  if (listenSettings.recordingHotkey.value && event.key === 'Escape') {
    listenSettings.recordingHotkey.value = false;
    void listenSettings.syncHotkeyWhileListening();
    return;
  }

  if (settings.recordingPauseHotkey.value) {
    const nextPauseHotkey = formatPauseHotkeyFromEvent(event);
    if (!nextPauseHotkey) {
      settings.pauseHotkeyError.value = t('settings.pauseHotkeyHint');
      return;
    }
    settings.pauseHotkey.value = nextPauseHotkey;
    settings.recordingPauseHotkey.value = false;
    settings.pauseHotkeyError.value = '';
    return;
  }

  const nextHotkey = formatHotkeyFromEvent(event);
  if (!nextHotkey) {
    listenSettings.listenSettingsError.value = t('home.hotkeyHint');
    return;
  }
  listenSettings.listenHotkey.value = nextHotkey;
  listenSettings.recordingHotkey.value = false;
  listenSettings.listenSettingsError.value = '';
  void listenSettings.syncHotkeyWhileListening();
}

function preventWebviewRefresh(event: KeyboardEvent) {
  const key = event.key.toLowerCase();
  if ((event.ctrlKey || event.metaKey) && key === 'r') { event.preventDefault(); event.stopPropagation(); }
  if (event.key === 'F5') { event.preventDefault(); event.stopPropagation(); }
}

function handleDocumentClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  if (!target?.closest('.listen-settings-popup') && !target?.closest('.listen-settings-button') && !target?.closest('.listen-button')) {
    listenSettings.closeListenSettings();
  }
}

// ── Mirror CDK dialog ─────────────────────────────────────────────────────────
function openMirrorChanSdkDialog() {
  mirrorChanSdkDraft.value = settings.mirrorChanSdk.value;
  mirrorChanSdkError.value = '';
  returnToUpdateAfterMirrorDialog.value = false;
  mirrorChanSdkDialogOpen.value = true;
}

function openMirrorChanSdkDialogFromUpdate() {
  updateDialogOpen.value = false;
  mirrorChanSdkDraft.value = settings.mirrorChanSdk.value;
  mirrorChanSdkError.value = '';
  returnToUpdateAfterMirrorDialog.value = true;
  mirrorChanSdkDialogOpen.value = true;
}

function cancelMirrorChanSdkDialog() {
  const shouldReturn = returnToUpdateAfterMirrorDialog.value;
  mirrorChanSdkDialogOpen.value = false;
  mirrorChanSdkDraft.value = settings.mirrorChanSdk.value;
  mirrorChanSdkError.value = '';
  returnToUpdateAfterMirrorDialog.value = false;
  if (shouldReturn) updateDialogOpen.value = true;
}

async function saveMirrorChanSdk() {
  const shouldStartUpdate = returnToUpdateAfterMirrorDialog.value;
  const nextSdk = mirrorChanSdkDraft.value.trim();
  if (!nextSdk) { mirrorChanSdkError.value = t('settings.mirrorChanSdkRequired'); return; }

  try {
    const validationContext = returnToUpdateAfterMirrorDialog.value ? '更新流程' : '设置';
    const validation = await invoke<MirrorCdkValidationInfo>('validate_mirror_cdk', { cdk: nextSdk, context: validationContext });
    if (!validation.valid || validation.mirror_code !== null) {
      const message = validation.mirror_code !== null
        ? mapMirrorError(validation.mirror_code, validation.mirror_message)
        : t('common.saveFailed');
      mirrorChanSdkError.value = message;
      notify({ title: t('common.saveFailed'), content: message, type: 'warn' });
      return;
    }
  } catch (error) {
    const message = String(error);
    mirrorChanSdkError.value = message;
    notify({ title: t('common.saveFailed'), content: message, type: 'false' });
    return;
  }

  settings.mirrorChanSdk.value = nextSdk;
  mirrorChanSdkDraft.value = nextSdk;
  mirrorChanSdkError.value = '';
  settings.downloadSource.value = 'mirror_chan';
  mirrorChanSdkDialogOpen.value = false;
  returnToUpdateAfterMirrorDialog.value = false;

  try {
    await invoke<AppConfig>('update_config', {
      patch: { mirror_chan_sdk: nextSdk, download_source: settings.downloadSource.value, last_selected_hwnd: selectedHomeWindowId.value },
    });
    settings.savedMirrorChanSdk.value = settings.mirrorChanSdk.value;
  } catch (error) {
    notify({ title: t('common.saveFailed'), content: String(error), type: 'false' });
    return;
  }

  if (shouldStartUpdate) {
    updateDialogOpen.value = true;
    updateClosePromptOpen.value = false;
    await runImmediateUpdate();
  }
}

// ── Update flow ───────────────────────────────────────────────────────────────
async function openUpdateDialog() {
  if (!updateInProgress.value) updateProgress.value = 0;
  updateClosePromptOpen.value = false;
  try {
    const info = await invoke<UpdateCheckInfo>('check_for_updates');
    if (info.mirror_code !== null) {
      notify({ title: t('toolbox.checkUpdates'), content: mapMirrorError(info.mirror_code, info.mirror_message), type: 'warn' });
      return;
    }
    if (!info.has_update) {
      notify({ title: t('toolbox.checkUpdates'), content: '当前已是最新版本', type: 'true' });
      return;
    }
    latestVersion.value = info.latest_version;
    updateChangelogMarkdown.value = info.changelog || '';
    updateDownloadUrl.value = info.download_url ?? '';
    updateDownloadCandidates.value = info.download_candidates ?? [];
    updateExpectedSha256.value = info.sha256 ?? '';
    updateDialogOpen.value = true;
  } catch (error) {
    notify({ title: t('toolbox.checkUpdates'), content: String(error), type: 'false' });
  }
}

async function runStartupUpdateCheck() {
  if (startupUpdateChecked.value || !settings.privacyConsentAccepted.value || !settings.autoCheckUpdates.value) return;
  startupUpdateChecked.value = true;
  notify({ title: t('toolbox.checkUpdates'), content: '正在检查更新', type: 'info' });
  await openUpdateDialog();
}

function closeUpdateDialog() {
  if (updateInProgress.value || updateClosePromptOpen.value) return;
  updateDialogOpen.value = false;
}

async function runImmediateUpdate() {
  if (updateInProgress.value) return;
  const useMirrorDownload =
    settings.updateSource.value === 'mirror_chan' && settings.downloadSource.value === 'mirror_chan';

  if (!useMirrorDownload && !updateDownloadUrl.value && updateDownloadCandidates.value.length === 0) {
    await openUpdateDialog();
    if (!updateDownloadUrl.value && updateDownloadCandidates.value.length === 0) {
      notify({ title: t('updateDialog.updateNow'), content: '当前没有可用下载链接', type: 'warn' });
      return;
    }
  }

  updateInProgress.value = true;
  updateProgress.value = 0;
  updateClosePromptOpen.value = false;

  try {
    let urls = updateDownloadCandidates.value.length > 0
      ? [...updateDownloadCandidates.value]
      : updateDownloadUrl.value
        ? [updateDownloadUrl.value]
        : [];
    let expectedSha = updateExpectedSha256.value || null;

    if (useMirrorDownload) {
      const mirrorInfo = await invoke<MirrorDownloadInfo>('resolve_mirror_download_url');
      if (mirrorInfo.mirror_code !== null) {
        notify({ title: t('updateDialog.updateNow'), content: mapMirrorError(mirrorInfo.mirror_code, mirrorInfo.mirror_message), type: 'warn' });
        updateInProgress.value = false;
        return;
      }
      if (!mirrorInfo.url) {
        notify({ title: t('updateDialog.updateNow'), content: 'Mirror酱未返回可用下载链接', type: 'warn' });
        updateInProgress.value = false;
        return;
      }
      urls = [mirrorInfo.url];
      if (mirrorInfo.sha256) expectedSha = mirrorInfo.sha256;
    }

    const result = await invoke<UpdateDownloadResult>('download_update_package', {
      urls, expected_sha256: expectedSha, version: latestVersion.value,
    });
    downloadedUpdatePath.value = result.file_path;
    if (result.fallback_used) {
      notify({ title: t('toolbox.checkUpdates'), content: '主下载源不可用，已自动切换到备用下载源', type: 'warn' });
    }
    updateProgress.value = 100;
    updateInProgress.value = false;
    updateClosePromptOpen.value = true;
  } catch (error) {
    updateInProgress.value = false;
    notify({ title: t('updateDialog.updateNow'), content: String(error), type: 'false' });
  }
}

async function confirmUpdateComplete() {
  if (!downloadedUpdatePath.value) {
    notify({ title: t('updateDialog.updateNow'), content: '未找到已下载的更新包', type: 'false' });
    return;
  }
  try {
    await invoke('apply_downloaded_update', { filePath: downloadedUpdatePath.value });
  } catch (error) {
    notify({ title: t('updateDialog.updateNow'), content: String(error), type: 'false' });
  }
}

function handleChangelogLinkClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  const anchor = target?.closest('a.md-link');
  if (!(anchor instanceof HTMLAnchorElement)) return;
  const href = anchor.getAttribute('href');
  if (!href) return;
  event.preventDefault();
  void invoke('open_external_url', { url: href });
}

// ── Privacy ───────────────────────────────────────────────────────────────────
const privacyDialogOpen = ref(false);

async function acceptPrivacyConsent() {
  try {
    await invoke<AppConfig>('update_config', { patch: { privacy_consent: true } });
    settings.privacyConsentAccepted.value = true;
    privacyDialogOpen.value = false;
    await runStartupUpdateCheck();
  } catch (error) {
    notify({ title: t('common.saveFailed'), content: String(error), type: 'false' });
  }
}

async function rejectPrivacyConsent() { await invoke('exit_app'); }

// ── Danger ────────────────────────────────────────────────────────────────────
function continueDangerAction() {
  dangerDialogOpen.value = false;
  notify({ title: t('toolbox.dangerAccepted'), content: t('toolbox.dangerAcceptedDesc'), type: 'info' });
}

// ── Settings save/discard wiring ──────────────────────────────────────────────
async function handleSaveSettings() {
  await settings.saveSettings({
    listenHotkey: listenSettings.listenHotkey.value,
    selectedWindowId: selectedHomeWindowId.value,
    listenMouseSideButton: listenSettings.listenMouseSideButton.value,
  });
}

function handleDiscardSettings() {
  settings.discardSettings();
}

// ── Lifecycle ─────────────────────────────────────────────────────────────────
onMounted(async () => {
  initTheme(() => {
    if (settings.theme.value === 'system') animateThemeChange(resolveTheme('system'));
  });
  renderedTheme.value = resolveTheme(settings.theme.value);

  document.addEventListener('click', handleDocumentClick);
  window.addEventListener('keydown', handleHotkeyRecord, true);
  window.addEventListener('keydown', preventWebviewRefresh, true);

  try {
    appVersion.value = await getVersion();
    await settings.loadConfigFromBackend();

    // Sync refs loaded by loadConfigFromBackend into useListenSettings
    listenSettings.listenHotkey.value = settings.listenHotkeyRef.value;
    listenSettings.listenMouseSideButton.value = settings.listenMouseSideButtonRef.value;
    listenSettings.autoListenOnStartup.value = settings.autoListenOnStartup.value;

    await settings.loadSystemFonts();
    applyAppFontFamily();

    privacyDialogOpen.value = !settings.privacyConsentAccepted.value;
    await runStartupUpdateCheck();
    await listenSettings.initListeningState();
    await refreshHomeWindows();
    memory.scheduleMemoryCleanup();

    updateProgressUnlisten = await listen<number>(UPDATE_DOWNLOAD_PROGRESS_EVENT, (event) => {
      updateProgress.value = Number(event.payload) || 0;
    });
    openSettingsUnlisten = await listen(OPEN_SETTINGS_EVENT, async () => {
      currentPage.value = 'settings';
    });
  } catch (error) {
    notify({ title: t('home.initFailed'), content: String(error), type: 'false' });
  }
});

onBeforeUnmount(() => {
  cleanupTheme();
  clearNotices();
  document.removeEventListener('click', handleDocumentClick);
  window.removeEventListener('keydown', handleHotkeyRecord, true);
  window.removeEventListener('keydown', preventWebviewRefresh, true);
  memory.clearMemoryCleanupScheduler();
  updateProgressUnlisten?.();
  updateProgressUnlisten = null;
  openSettingsUnlisten?.();
  openSettingsUnlisten = null;
});
</script>

<template>
  <div
    ref="appShellRef"
    :class="['app-shell', `theme-${activeTheme}`]"
    :style="{ '--font-scale': String(fontScale) }"
    @contextmenu.prevent
  >
    <!-- Theme ripple overlay -->
    <div
      v-if="themeRipple.visible"
      :class="['theme-ripple-overlay', `target-${themeRipple.target}`]"
      :style="{ width: `${themeRipple.size}px`, height: `${themeRipple.size}px`, left: `${themeRipple.x}px`, top: `${themeRipple.y}px` }"
      @animationend="handleThemeRippleEnd"
    />

    <!-- Notification stack -->
    <TransitionGroup name="save-notice" tag="div" class="save-notice-stack">
      <div v-for="notice in notices" :key="notice.id" :class="['save-notice', notice.type]">
        <div class="save-notice-body">
          <span class="save-notice-title">{{ notice.title }}</span>
          <span v-if="notice.content" class="save-notice-content">{{ notice.content }}</span>
        </div>
        <button class="save-notice-close" type="button" @click="removeSaveNotice(notice.id)">{{ t('common.dismiss') }}</button>
      </div>
    </TransitionGroup>

    <!-- Navigation -->
    <header class="dynamic-island">
      <button :class="['island-button', { active: currentPage === 'home' }]" type="button" @click="currentPage = 'home'">{{ t('nav.home') }}</button>
      <button :class="['island-button', { active: currentPage === 'toolbox' }]" type="button" @click="currentPage = 'toolbox'">{{ t('nav.toolbox') }}</button>
      <button :class="['island-button', { active: currentPage === 'settings' }]" type="button" @click="currentPage = 'settings'">{{ t('nav.settings') }}</button>
    </header>

    <!-- Pages -->
    <main class="page-panel">
      <Transition name="page-switch" mode="out-in">
        <HomePageView
          v-if="currentPage === 'home'"
          key="home"
          :windows="homeWindows"
          :selected-window-id="selectedHomeWindowId"
          :is-listening="listenSettings.isListening.value"
          :listen-settings-open="listenSettings.listenSettingsOpen.value"
          :listen-hotkey="listenSettings.listenHotkey.value"
          :listen-mouse-side-button="listenSettings.listenMouseSideButton.value"
          :auto-listen-on-startup="listenSettings.autoListenOnStartup.value"
          :recording-hotkey="listenSettings.recordingHotkey.value"
          :listen-settings-error="listenSettings.listenSettingsError.value"
          :windows-loading="windowsLoading"
          @select-window="selectHomeWindow"
          @select-foreground-window="selectForegroundWindow"
          @toggle-listening="listenSettings.toggleListening"
          @toggle-listen-settings="listenSettings.toggleListenSettings"
          @close-listen-settings="listenSettings.closeListenSettings"
          @toggle-hotkey-recording="listenSettings.toggleHotkeyRecording"
          @clear-hotkey="listenSettings.clearHotkey"
          @toggle-mouse-side-button="listenSettings.toggleMouseSideButton"
          @toggle-auto-listen-on-startup="listenSettings.toggleAutoListenOnStartup"
          @refresh="refreshHomeWindows"
        />

        <ToolboxPageView
          v-else-if="currentPage === 'toolbox'"
          key="toolbox"
          :memory-auto-cleanup="memory.memoryAutoCleanup.value"
          :memory-cleanup-interval="memory.memoryCleanupInterval.value"
          :memory-cleanup-unit="memory.memoryCleanupUnit.value"
          :memory-cleanup-running="memory.memoryCleanupRunning.value"
          :memory-cleanup-interval-invalid="memory.memoryCleanupIntervalInvalid.value"
          :cache-selections="cache.cacheSelections.value"
          :cache-cleanup-running="cache.cacheCleanupRunning.value"
          :app-version="appVersion"
          @run-memory-cleanup="memory.runMemoryCleanup()"
          @toggle-memory-auto-cleanup="memory.toggleMemoryAutoCleanup"
          @memory-interval-input="memory.handleMemoryIntervalInput"
          @finalize-memory-interval="memory.finalizeMemoryIntervalInput"
          @select-memory-unit="memory.selectMemoryCleanupUnit"
          @toggle-cache-selection="cache.toggleCacheSelection"
          @run-cache-cleanup="cache.runCacheCleanup"
          @open-update-dialog="openUpdateDialog"
          @open-danger-dialog="dangerDialogOpen = true"
        />

        <SettingsPageView
          v-else
          ref="settingsRef"
          key="settings"
          :language="settings.language.value"
          :theme="settings.theme.value"
          :font-family="settings.fontFamily.value"
          :font-size="settings.fontSize.value"
          :auto-start="settings.autoStart.value"
          :silent-start="settings.silentStart.value"
          :mute-on-hide="settings.muteOnHide.value"
          :pause-on-hide="settings.pauseOnHide.value"
          :pause-hotkey="settings.pauseHotkey.value"
          :update-source="settings.updateSource.value"
          :update-channel="settings.updateChannel.value"
          :mirror-chan-sdk="settings.mirrorChanSdk.value"
          :download-source="settings.downloadSource.value"
          :auto-check-updates="settings.autoCheckUpdates.value"
          :settings-dirty="settings.settingsDirty.value"
          :silent-start-disabled="settings.silentStartDisabled.value"
          :pause-hotkey-error="settings.pauseHotkeyError.value"
          :recording-pause-hotkey="settings.recordingPauseHotkey.value"
          :system-fonts="settings.systemFonts.value"
          @update:language="(v) => { settings.language.value = v; locale = v; }"
          @update:theme="(v) => { settings.theme.value = v; animateThemeChange(resolveTheme(v)); }"
          @update:font-family="(v) => settings.fontFamily.value = v"
          @update:font-size="(v) => settings.fontSize.value = v"
          @toggle-auto-start="settings.toggleAutoStart"
          @toggle-silent-start="settings.toggleSilentStart"
          @toggle-mute-on-hide="() => settings.muteOnHide.value = !settings.muteOnHide.value"
          @toggle-pause-on-hide="settings.togglePauseOnHide"
          @toggle-auto-check-updates="() => settings.autoCheckUpdates.value = !settings.autoCheckUpdates.value"
          @select-update-source="(v) => settings.updateSource.value = v"
          @select-update-channel="settings.selectUpdateChannel"
          @select-download-source="(v) => settings.downloadSource.value = v"
          @open-mirror-cdk-dialog="openMirrorChanSdkDialog"
          @toggle-pause-hotkey-recording="settings.togglePauseHotkeyRecording"
          @clear-pause-hotkey="settings.clearPauseHotkey"
          @save="handleSaveSettings"
          @discard="handleDiscardSettings"
        />
      </Transition>
    </main>

    <!-- Dialogs -->
    <PrivacyDialog
      :open="privacyDialogOpen"
      @accept="acceptPrivacyConsent"
      @reject="rejectPrivacyConsent"
      @open-privacy-policy="invoke('open_external_url', { url: PRIVACY_POLICY_URL })"
    />

    <MirrorCdkDialog
      :open="mirrorChanSdkDialogOpen"
      :draft="mirrorChanSdkDraft"
      :error="mirrorChanSdkError"
      @update:draft="mirrorChanSdkDraft = $event"
      @save="saveMirrorChanSdk"
      @cancel="cancelMirrorChanSdkDialog"
    />

    <UpdateDialog
      :open="updateDialogOpen"
      :latest-version="latestVersion"
      :rendered-changelog="renderedUpdateChangelog"
      :update-in-progress="updateInProgress"
      :update-progress="updateProgress"
      :update-close-prompt-open="updateClosePromptOpen"
      :mirror-chan-sdk-configured="settings.mirrorChanSdkConfigured.value"
      @close="closeUpdateDialog"
      @update-now="runImmediateUpdate"
      @fill-mirror-cdk="openMirrorChanSdkDialogFromUpdate"
      @changelog-link-click="handleChangelogLinkClick"
    />

    <!-- Update close prompt (inline, no separate component needed) -->
    <Transition name="dialog-fade">
      <div v-if="updateClosePromptOpen" class="dialog-overlay" @click.stop>
        <div class="dialog-panel update-complete-panel" @click.stop>
          <div class="dialog-title">{{ t('updateDialog.closeRequiredTitle') }}</div>
          <div class="dialog-description">{{ t('updateDialog.closeRequiredDesc') }}</div>
          <div class="dialog-actions">
            <button class="dialog-action-button primary" type="button" @click="confirmUpdateComplete">{{ t('common.confirm') }}</button>
          </div>
        </div>
      </div>
    </Transition>

    <DangerDialog :open="dangerDialogOpen" @continue="continueDangerAction" />
  </div>
</template>

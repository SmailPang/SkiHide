<script setup lang="ts">
import appIconUrl from './assets/app-icon.svg';
import mirrorchyanUrl from './assets/mirrorchyan.png';
import raincloudUrl from './assets/raincloud.png';
import avatarUrl from './assets/touxiang.jpg';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AppConfig, CacheCleanupOptions, CacheCleanupReport, MemoryCleanupReport, MirrorCdkValidationInfo, MirrorDownloadInfo, UpdateCheckInfo, UpdateDownloadResult, WindowInfo } from './types';

type PageKey = 'home' | 'toolbox' | 'settings';
type NoticeType = 'true' | 'false' | 'warn' | 'info';
type OptionValue =
  | 'system'
  | 'light'
  | 'dark'
  | 'small'
  | 'medium'
  | 'large'
  | 'xlarge'
  | 'mirror_chan'
  | 'skihide'
  | 'github'
  | 'rainyun_cdn';
type LanguageValue = 'zh_CN' | 'zh_TW' | 'en_US' | 'ja_JP';
type ToolboxCardKey = 'memory' | 'cache' | 'feedback' | 'about';
type AppNotice = { id: number; type: NoticeType; title: string; content?: string };

const { t, locale } = useI18n();

const languageOptionValues: LanguageValue[] = ['zh_CN', 'zh_TW', 'en_US', 'ja_JP'];
const languageOptionLabels: Record<LanguageValue, string> = {
  zh_CN: '简体中文',
  zh_TW: '繁體中文',
  en_US: 'English',
  ja_JP: '日本語',
};
const themeOptionValues: Array<'system' | 'light' | 'dark'> = ['system', 'light', 'dark'];
const fontSizeOptionValues: Array<'small' | 'medium' | 'large' | 'xlarge'> = ['small', 'medium', 'large', 'xlarge'];
const updateSourceOptionValues: Array<'mirror_chan' | 'skihide'> = ['mirror_chan', 'skihide'];
const downloadSourceOptionValues: Array<'mirror_chan' | 'github' | 'rainyun_cdn'> = ['mirror_chan', 'github', 'rainyun_cdn'];

const currentPage = ref<PageKey>('home');
const homeWindows = ref<WindowInfo[]>([]);
const selectedHomeWindowId = ref<number | null>(null);
const isListening = ref(false);
const listenSettingsOpen = ref(false);
const activeToolboxCard = ref<ToolboxCardKey | null>(null);

const language = ref<LanguageValue>((locale.value as LanguageValue) || 'zh_CN');
const savedLanguage = ref<LanguageValue>(language.value);
const theme = ref<'system' | 'light' | 'dark'>('system');
const savedTheme = ref<'system' | 'light' | 'dark'>('system');
const fontSize = ref<'small' | 'medium' | 'large' | 'xlarge'>('medium');
const savedFontSize = ref<'small' | 'medium' | 'large' | 'xlarge'>('medium');
const autoStart = ref(false);
const savedAutoStart = ref(false);
const silentStart = ref(false);
const savedSilentStart = ref(false);
const muteOnHide = ref(false);
const savedMuteOnHide = ref(false);
const updateSource = ref<'mirror_chan' | 'skihide'>('mirror_chan');
const savedUpdateSource = ref<'mirror_chan' | 'skihide'>('mirror_chan');
const mirrorChanSdk = ref('');
const savedMirrorChanSdk = ref('');
const downloadSource = ref<'mirror_chan' | 'github' | 'rainyun_cdn'>('rainyun_cdn');
const savedDownloadSource = ref<'mirror_chan' | 'github' | 'rainyun_cdn'>('rainyun_cdn');
const autoCheckUpdates = ref(true);
const savedAutoCheckUpdates = ref(true);
const memoryAutoCleanup = ref(false);
const memoryCleanupInterval = ref('5');
const memoryCleanupUnit = ref<'seconds' | 'minutes' | 'hours'>('minutes');
const memoryCleanupRunning = ref(false);
const cacheSelections = ref({ systemCache: false, tempFiles: false, thumbnailCache: false, appCache: false, recycleBin: false });
const cacheCleanupRunning = ref(false);
const languageOpen = ref(false);
const themeOpen = ref(false);
const fontSizeOpen = ref(false);
const updateSourceOpen = ref(false);
const downloadSourceOpen = ref(false);
const mirrorChanSdkDialogOpen = ref(false);
const updateDialogOpen = ref(false);
const updateClosePromptOpen = ref(false);
const dangerDialogOpen = ref(false);
const returnToUpdateAfterMirrorDialog = ref(false);
const languageMenuUp = ref(false);
const themeMenuUp = ref(false);
const fontSizeMenuUp = ref(false);
const updateSourceMenuUp = ref(false);
const downloadSourceMenuUp = ref(false);
const mirrorChanSdkDraft = ref('');
const mirrorChanSdkError = ref('');
const listenHotkey = ref('');
const listenMouseSideButton = ref(false);
const recordingHotkey = ref(false);
const listenSettingsError = ref('');
const prefersDark = ref(false);
const renderedTheme = ref<'light' | 'dark'>('light');
const currentLanguageLabel = computed(() => languageOptionLabels[language.value]);
const currentThemeLabel = computed(() => t(`optionLabels.theme.${theme.value}`));
const currentFontSizeLabel = computed(() => t(`optionLabels.fontSize.${fontSize.value}`));
const currentUpdateSourceLabel = computed(() => t(`optionLabels.updateSource.${updateSource.value}`));
const currentDownloadSourceLabel = computed(() => t(`optionLabels.downloadSource.${downloadSource.value}`));
const availableDownloadSourceOptionValues = computed(() =>
  mirrorChanSdk.value.trim()
    ? downloadSourceOptionValues
    : downloadSourceOptionValues.filter((option) => option !== 'mirror_chan'),
);
const mirrorChanSdkConfigured = computed(() => mirrorChanSdk.value.trim().length > 0);
const activeTheme = computed(() => renderedTheme.value);
const settingsDirty = computed(
  () =>
    language.value !== savedLanguage.value ||
    theme.value !== savedTheme.value ||
    fontSize.value !== savedFontSize.value ||
    autoStart.value !== savedAutoStart.value ||
    silentStart.value !== savedSilentStart.value ||
    muteOnHide.value !== savedMuteOnHide.value ||
    updateSource.value !== savedUpdateSource.value ||
    mirrorChanSdk.value !== savedMirrorChanSdk.value ||
    downloadSource.value !== savedDownloadSource.value ||
    autoCheckUpdates.value !== savedAutoCheckUpdates.value,
);
const silentStartDisabled = computed(() => !autoStart.value);
const fontScale = computed(() => { switch (fontSize.value) { case 'small': return 0.92; case 'large': return 1.08; case 'xlarge': return 1.16; default: return 1; } });
const hotkeyButtonLabel = computed(() => (recordingHotkey.value ? t('home.recordingHotkey') : listenHotkey.value || t('home.bindHotkey')));
const memoryCleanupIntervalInvalid = computed(() => { if (!memoryAutoCleanup.value) return false; const parsedValue = Number(memoryCleanupInterval.value); return !Number.isFinite(parsedValue) || parsedValue <= 0; });
const appShellRef = ref<HTMLElement | null>(null);
const themeTriggerRef = ref<HTMLElement | null>(null);
const toolboxScrollRef = ref<HTMLElement | null>(null);
const toolboxScrollbarRef = ref<HTMLElement | null>(null);
const themeRipple = ref({ visible: false, target: 'light' as 'light' | 'dark', x: 0, y: 0, size: 0 });
const toolboxScrollbarActive = ref(false);
const toolboxThumbHeight = ref(0);
const toolboxThumbOffset = ref(0);
const notices = ref<AppNotice[]>([]);
let colorSchemeQuery: MediaQueryList | null = null;
let themeSwitchTimer: number | null = null;
let memoryCleanupTimer: number | null = null;
let toolboxDragStartY = 0;
let toolboxDragStartScrollTop = 0;
let toolboxDragging = false;
let noticeIdSeed = 0;
let openSettingsUnlisten: UnlistenFn | null = null;
let updateProgressUnlisten: UnlistenFn | null = null;
const noticeTimers = new Map<number, number>();
const latestVersion = ref('');
const updateChangelogMarkdown = ref('');
const updateDownloadUrl = ref('');
const updateDownloadCandidates = ref<string[]>([]);
const updateExpectedSha256 = ref('');
const downloadedUpdatePath = ref('');
const updateProgress = ref(0);
const updateInProgress = ref(false);
const windowsLoading = ref(false);
const privacyConsentAccepted = ref(false);
const privacyDialogOpen = ref(false);
const startupUpdateChecked = ref(false);

const OPEN_SETTINGS_EVENT = 'skihide://open-settings';
const UPDATE_DOWNLOAD_PROGRESS_EVENT = 'skihide://update-download-progress';
const PRIVACY_POLICY_URL = 'https://skihide.xyz/guide/privacy';

function resolveTheme(mode: string): 'light' | 'dark' { if (mode === 'system') return prefersDark.value ? 'dark' : 'light'; return mode === 'dark' ? 'dark' : 'light'; }
function shouldOpenMenuUp(trigger: HTMLElement | null, optionCount: number) {
  if (!trigger) return false;
  const triggerRect = trigger.getBoundingClientRect();
  const settingsPage = trigger.closest('.settings-page');
  const containerRect = settingsPage instanceof HTMLElement ? settingsPage.getBoundingClientRect() : null;
  const topBoundary = containerRect ? containerRect.top : 0;
  const bottomBoundary = containerRect ? containerRect.bottom - 58 : window.innerHeight - 58;
  const estimatedHeight = Math.min(optionCount * 38 + 12, 188);
  const spaceBelow = bottomBoundary - triggerRect.bottom;
  const spaceAbove = triggerRect.top - topBoundary;
  return spaceBelow < estimatedHeight && spaceAbove > spaceBelow;
}
function toggleLanguageMenu(event: MouseEvent) { const nextOpen = !languageOpen.value; languageMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement | null, languageOptionValues.length); languageOpen.value = nextOpen; themeOpen.value = false; fontSizeOpen.value = false; updateSourceOpen.value = false; downloadSourceOpen.value = false; }
function selectLanguage(value: LanguageValue) { language.value = value; locale.value = value; languageOpen.value = false; }
function toggleThemeMenu(event: MouseEvent) { const nextOpen = !themeOpen.value; themeMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement | null, themeOptionValues.length); themeOpen.value = nextOpen; languageOpen.value = false; fontSizeOpen.value = false; updateSourceOpen.value = false; downloadSourceOpen.value = false; }
function toggleFontSizeMenu(event: MouseEvent) { const nextOpen = !fontSizeOpen.value; fontSizeMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement | null, fontSizeOptionValues.length); fontSizeOpen.value = nextOpen; languageOpen.value = false; themeOpen.value = false; updateSourceOpen.value = false; downloadSourceOpen.value = false; }
function selectFontSize(value: 'small' | 'medium' | 'large' | 'xlarge') { fontSize.value = value; fontSizeOpen.value = false; }
function toggleUpdateSourceMenu(event: MouseEvent) { const nextOpen = !updateSourceOpen.value; updateSourceMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement | null, updateSourceOptionValues.length); updateSourceOpen.value = nextOpen; languageOpen.value = false; themeOpen.value = false; fontSizeOpen.value = false; downloadSourceOpen.value = false; }
function selectUpdateSource(value: 'mirror_chan' | 'skihide') { updateSource.value = value; updateSourceOpen.value = false; }
function toggleDownloadSourceMenu(event: MouseEvent) { const nextOpen = !downloadSourceOpen.value; downloadSourceMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement | null, availableDownloadSourceOptionValues.value.length); downloadSourceOpen.value = nextOpen; languageOpen.value = false; themeOpen.value = false; fontSizeOpen.value = false; updateSourceOpen.value = false; }
function selectDownloadSource(value: 'mirror_chan' | 'github' | 'rainyun_cdn') { if (value === 'mirror_chan' && !mirrorChanSdkConfigured.value) return; downloadSource.value = value; downloadSourceOpen.value = false; }
function openMirrorChanSdkDialog() { mirrorChanSdkDraft.value = mirrorChanSdk.value; mirrorChanSdkError.value = ''; returnToUpdateAfterMirrorDialog.value = false; mirrorChanSdkDialogOpen.value = true; }
function openMirrorChanSdkDialogFromUpdate() { updateDialogOpen.value = false; mirrorChanSdkDraft.value = mirrorChanSdk.value; mirrorChanSdkError.value = ''; returnToUpdateAfterMirrorDialog.value = true; mirrorChanSdkDialogOpen.value = true; }
function cancelMirrorChanSdkDialog() {
  const shouldReturn = returnToUpdateAfterMirrorDialog.value;
  mirrorChanSdkDialogOpen.value = false;
  mirrorChanSdkDraft.value = mirrorChanSdk.value;
  mirrorChanSdkError.value = '';
  returnToUpdateAfterMirrorDialog.value = false;
  if (shouldReturn) updateDialogOpen.value = true;
}
async function saveMirrorChanSdk() {
  const shouldStartUpdate = returnToUpdateAfterMirrorDialog.value;
  const nextSdk = mirrorChanSdkDraft.value.trim();
  if (!nextSdk) {
    mirrorChanSdkError.value = t('settings.mirrorChanSdkRequired');
    return;
  }

  try {
    const validation = await invoke<MirrorCdkValidationInfo>('validate_mirror_cdk', { cdk: nextSdk });
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

  mirrorChanSdk.value = nextSdk;
  mirrorChanSdkDraft.value = nextSdk;
  mirrorChanSdkError.value = '';
  downloadSource.value = 'mirror_chan';
  mirrorChanSdkDialogOpen.value = false;
  returnToUpdateAfterMirrorDialog.value = false;
  try {
    await invoke<AppConfig>('update_config', {
      patch: {
        mirror_chan_sdk: nextSdk,
        download_source: downloadSource.value,
        last_selected_hwnd: selectedHomeWindowId.value,
      },
    });
    savedMirrorChanSdk.value = mirrorChanSdk.value;
    savedDownloadSource.value = downloadSource.value;
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
function mapMirrorError(code: number, message: string | null): string {
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
  if (startupUpdateChecked.value || !privacyConsentAccepted.value || !autoCheckUpdates.value) return;
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
  if (!updateDownloadUrl.value && updateDownloadCandidates.value.length === 0) {
    await openUpdateDialog();
    if (!updateDownloadUrl.value && updateDownloadCandidates.value.length === 0) {
      if (updateSource.value === 'mirror_chan' && downloadSource.value === 'mirror_chan') {
        if (!mirrorChanSdk.value.trim()) {
          notify({ title: t('updateDialog.updateNow'), content: t('settings.mirrorChanSdkRequired'), type: 'warn' });
          return;
        }
      } else {
        notify({ title: t('updateDialog.updateNow'), content: '当前没有可用下载链接', type: 'warn' });
        return;
      }
    }
  }

  updateInProgress.value = true;
  updateProgress.value = 0;
  updateClosePromptOpen.value = false;

  try {
    let urls = updateDownloadCandidates.value.length > 0 ? [...updateDownloadCandidates.value] : [updateDownloadUrl.value];
    let expectedSha = updateExpectedSha256.value || null;
    if (updateSource.value === 'mirror_chan' && downloadSource.value === 'mirror_chan') {
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
      if (mirrorInfo.sha256) {
        expectedSha = mirrorInfo.sha256;
      }
    }

    const result = await invoke<UpdateDownloadResult>('download_update_package', {
      urls,
      expected_sha256: expectedSha,
      version: latestVersion.value,
    });
    downloadedUpdatePath.value = result.file_path;
    if (result.fallback_used) {
      notify({
        title: t('toolbox.checkUpdates'),
        content: '主下载源不可用，已自动切换到备用下载源',
        type: 'warn',
      });
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
function openDangerDialog() { dangerDialogOpen.value = true; }
function closeDangerDialog() { dangerDialogOpen.value = false; }
function continueDangerAction() {
  dangerDialogOpen.value = false;
  notify({ title: t('toolbox.dangerAccepted'), content: t('toolbox.dangerAcceptedDesc'), type: 'info' });
}
function renderSimpleMarkdown(markdown: string) {
  const escapeHtml = (value: string) =>
    value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  const inlineFormat = (value: string) =>
    value.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  return markdown
    .trim()
    .split('\n')
    .map((line) => {
      const escaped = inlineFormat(escapeHtml(line.trim()));
      if (!escaped) return '<div class="md-spacer"></div>';
      const heading = escaped.match(/^(#{1,6})\s+(.+)$/);
      if (heading) {
        const level = Math.min(6, Math.max(1, heading[1].length));
        return `<h${level}>${heading[2]}</h${level}>`;
      }
      if (escaped.startsWith('- ')) return `<li>${escaped.slice(2)}</li>`;
      return `<p>${escaped}</p>`;
    })
    .join('')
    .replace(/(<li>.*?<\/li>)+/g, (match) => `<ul>${match}</ul>`);
}
const renderedUpdateChangelog = computed(() => renderSimpleMarkdown(updateChangelogMarkdown.value || ''));
function selectTheme(value: 'system' | 'light' | 'dark') { theme.value = value; themeOpen.value = false; animateThemeChange(resolveTheme(value)); }
function removeSaveNotice(id: number) {
  const timer = noticeTimers.get(id);
  if (timer !== undefined) {
    window.clearTimeout(timer);
    noticeTimers.delete(id);
  }
  notices.value = notices.value.filter((notice) => notice.id !== id);
}
function notify(options: { title: string; content?: string; type: NoticeType }) {
  const id = ++noticeIdSeed;
  notices.value = [...notices.value, { id, ...options }];
  const timer = window.setTimeout(() => removeSaveNotice(id), 3000);
  noticeTimers.set(id, timer);
}
async function saveSettings() {
  try {
    if (!mirrorChanSdkConfigured.value && downloadSource.value === 'mirror_chan') downloadSource.value = 'rainyun_cdn';
    const normalizedSilentStart = autoStart.value ? silentStart.value : false;
    silentStart.value = normalizedSilentStart;
    await invoke<AppConfig>('update_config', {
      patch: {
        hotkey: listenHotkey.value,
        language: language.value,
        last_selected_hwnd: selectedHomeWindowId.value,
        theme: theme.value,
        font_size: fontSize.value,
        auto_start: autoStart.value,
        silent_start: normalizedSilentStart,
        mute_on_hide: muteOnHide.value,
        update_source: updateSource.value,
        download_source: downloadSource.value,
        mirror_chan_sdk: mirrorChanSdk.value,
        auto_check_updates: autoCheckUpdates.value,
        mouse_side_button_listener: listenMouseSideButton.value,
      },
    });
    savedLanguage.value = language.value;
    savedTheme.value = theme.value;
    savedFontSize.value = fontSize.value;
    savedAutoStart.value = autoStart.value;
    savedSilentStart.value = normalizedSilentStart;
    savedMuteOnHide.value = muteOnHide.value;
    savedUpdateSource.value = updateSource.value;
    savedMirrorChanSdk.value = mirrorChanSdk.value;
    savedDownloadSource.value = downloadSource.value;
    savedAutoCheckUpdates.value = autoCheckUpdates.value;
    silentStart.value = savedSilentStart.value;
    languageOpen.value = false;
    themeOpen.value = false;
    fontSizeOpen.value = false;
    updateSourceOpen.value = false;
    downloadSourceOpen.value = false;
    mirrorChanSdkDialogOpen.value = false;
    notify({ title: t('common.saveSuccess'), type: 'true' });
  } catch (error) {
    notify({ title: t('common.saveFailed'), content: String(error), type: 'false' });
  }
}
function discardSettings() {
  language.value = savedLanguage.value;
  theme.value = savedTheme.value;
  fontSize.value = savedFontSize.value;
  autoStart.value = savedAutoStart.value;
  silentStart.value = savedSilentStart.value;
  muteOnHide.value = savedMuteOnHide.value;
  updateSource.value = savedUpdateSource.value;
  mirrorChanSdk.value = savedMirrorChanSdk.value;
  mirrorChanSdkDraft.value = savedMirrorChanSdk.value;
  downloadSource.value = savedDownloadSource.value;
  autoCheckUpdates.value = savedAutoCheckUpdates.value;
  if (!mirrorChanSdkConfigured.value && downloadSource.value === 'mirror_chan') downloadSource.value = 'rainyun_cdn';
  locale.value = savedLanguage.value;
  languageOpen.value = false;
  themeOpen.value = false;
  fontSizeOpen.value = false;
  updateSourceOpen.value = false;
  downloadSourceOpen.value = false;
  mirrorChanSdkDialogOpen.value = false;
  animateThemeChange(resolveTheme(savedTheme.value));
}
function toggleAutoStart() { autoStart.value = !autoStart.value; if (!autoStart.value) silentStart.value = false; }
function toggleSilentStart() { if (!autoStart.value) return; silentStart.value = !silentStart.value; }
function handleDocumentClick(event: MouseEvent) { const target = event.target as HTMLElement | null; if (!target?.closest('.custom-select')) { languageOpen.value = false; themeOpen.value = false; fontSizeOpen.value = false; updateSourceOpen.value = false; downloadSourceOpen.value = false; } if (!target?.closest('.listen-settings-popup') && !target?.closest('.listen-settings-button') && !target?.closest('.listen-button')) closeListenSettings(); }
function handleColorSchemeChange(event: MediaQueryListEvent) { prefersDark.value = event.matches; if (theme.value === 'system') animateThemeChange(resolveTheme('system')); }
function animateThemeChange(nextTheme: 'light' | 'dark') { if (renderedTheme.value === nextTheme || !appShellRef.value) { renderedTheme.value = nextTheme; return; } const shellRect = appShellRef.value.getBoundingClientRect(); const triggerRect = themeTriggerRef.value?.getBoundingClientRect(); const centerX = triggerRect ? triggerRect.left - shellRect.left + triggerRect.width / 2 : shellRect.width / 2; const centerY = triggerRect ? triggerRect.top - shellRect.top + triggerRect.height / 2 : shellRect.height / 2; const radius = Math.max(Math.hypot(centerX, centerY), Math.hypot(shellRect.width - centerX, centerY), Math.hypot(centerX, shellRect.height - centerY), Math.hypot(shellRect.width - centerX, shellRect.height - centerY)); if (themeSwitchTimer !== null) window.clearTimeout(themeSwitchTimer); themeRipple.value = { visible: false, target: nextTheme, x: centerX - radius, y: centerY - radius, size: radius * 2 }; requestAnimationFrame(() => { themeRipple.value = { ...themeRipple.value, visible: true }; }); themeSwitchTimer = window.setTimeout(() => { renderedTheme.value = nextTheme; themeSwitchTimer = null; }, 170); }
function handleThemeRippleEnd() { themeRipple.value = { ...themeRipple.value, visible: false }; }
async function refreshHomeWindows() {
  if (windowsLoading.value) return;
  windowsLoading.value = true;
  try {
    const windows = await invoke<WindowInfo[]>('list_windows');
    homeWindows.value = windows;
    if (selectedHomeWindowId.value !== null && !windows.some((item) => item.hwnd === selectedHomeWindowId.value)) {
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
    await invoke<AppConfig>('update_config', {
      patch: {
        hotkey: null,
        language: null,
        last_selected_hwnd: hwnd,
      },
    });
  } catch (error) {
    notify({ title: t('home.saveSelectedWindowFailed'), content: String(error), type: 'warn' });
  }
}
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
async function loadConfigFromBackend() {
  const config = await invoke<AppConfig>('get_config');

  listenHotkey.value = config.hotkey ?? '';

  const nextLanguage = languageOptionValues.includes(config.language as LanguageValue) ? (config.language as LanguageValue) : 'zh_CN';
  language.value = nextLanguage;
  savedLanguage.value = nextLanguage;
  locale.value = nextLanguage;

  const nextTheme = themeOptionValues.includes(config.theme as 'system' | 'light' | 'dark') ? (config.theme as 'system' | 'light' | 'dark') : 'system';
  theme.value = nextTheme;
  savedTheme.value = nextTheme;
  animateThemeChange(resolveTheme(nextTheme));

  const nextFontSize = fontSizeOptionValues.includes(config.font_size as 'small' | 'medium' | 'large' | 'xlarge') ? (config.font_size as 'small' | 'medium' | 'large' | 'xlarge') : 'medium';
  fontSize.value = nextFontSize;
  savedFontSize.value = nextFontSize;

  const nextAutoStart = Boolean(config.auto_start);
  const nextSilentStart = nextAutoStart ? Boolean(config.silent_start) : false;
  autoStart.value = nextAutoStart;
  savedAutoStart.value = nextAutoStart;
  silentStart.value = nextSilentStart;
  savedSilentStart.value = nextSilentStart;

  const nextMuteOnHide = Boolean(config.mute_on_hide);
  muteOnHide.value = nextMuteOnHide;
  savedMuteOnHide.value = nextMuteOnHide;

  const nextUpdateSource = updateSourceOptionValues.includes(config.update_source as 'mirror_chan' | 'skihide') ? (config.update_source as 'mirror_chan' | 'skihide') : 'mirror_chan';
  updateSource.value = nextUpdateSource;
  savedUpdateSource.value = nextUpdateSource;

  mirrorChanSdk.value = config.mirror_chan_sdk ?? '';
  savedMirrorChanSdk.value = mirrorChanSdk.value;
  mirrorChanSdkDraft.value = mirrorChanSdk.value;

  const nextDownloadSource = downloadSourceOptionValues.includes(config.download_source as 'mirror_chan' | 'github' | 'rainyun_cdn') ? (config.download_source as 'mirror_chan' | 'github' | 'rainyun_cdn') : 'rainyun_cdn';
  const normalizedDownloadSource = nextDownloadSource === 'mirror_chan' && !mirrorChanSdk.value.trim() ? 'rainyun_cdn' : nextDownloadSource;
  downloadSource.value = normalizedDownloadSource;
  savedDownloadSource.value = normalizedDownloadSource;
  autoCheckUpdates.value = config.auto_check_updates ?? true;
  savedAutoCheckUpdates.value = autoCheckUpdates.value;
  listenMouseSideButton.value = Boolean(config.mouse_side_button_listener);
  privacyConsentAccepted.value = Boolean(config.privacy_consent);

  selectedHomeWindowId.value = config.last_selected_hwnd ?? null;
}
function toggleToolboxCard(card: ToolboxCardKey) { activeToolboxCard.value = activeToolboxCard.value === card ? null : card; void nextTick(() => requestAnimationFrame(updateToolboxScrollbar)); }
function memoryCleanupIntervalMs() {
  if (!memoryAutoCleanup.value || memoryCleanupIntervalInvalid.value) return null;
  const value = Number(memoryCleanupInterval.value);
  if (!Number.isFinite(value) || value <= 0) return null;
  if (memoryCleanupUnit.value === 'hours') return value * 60 * 60 * 1000;
  if (memoryCleanupUnit.value === 'minutes') return value * 60 * 1000;
  return value * 1000;
}
function clearMemoryCleanupScheduler() {
  if (memoryCleanupTimer !== null) {
    window.clearInterval(memoryCleanupTimer);
    memoryCleanupTimer = null;
  }
}
function scheduleMemoryCleanup() {
  clearMemoryCleanupScheduler();
  const intervalMs = memoryCleanupIntervalMs();
  if (intervalMs === null) return;
  memoryCleanupTimer = window.setInterval(() => {
    void runMemoryCleanup(true);
  }, intervalMs);
}
async function runMemoryCleanup(isAutoTrigger = false) {
  if (memoryCleanupRunning.value) return;
  if (isAutoTrigger && memoryCleanupIntervalInvalid.value) return;
  memoryCleanupRunning.value = true;
  try {
    const report = await invoke<MemoryCleanupReport>('cleanup_memory');
    if (!isAutoTrigger) {
      const reclaimedMb = (report.reclaimed_bytes / 1024 / 1024).toFixed(2);
      notify({
        title: t('toolbox.memoryTitle'),
        content: t('toolbox.memoryResult', {
          cleaned: report.cleaned,
          scanned: report.scanned,
          reclaimedMb,
        }),
        type: 'true',
      });
    }
  } catch (error) {
    notify({
      title: t('toolbox.memoryTitle'),
      content: String(error),
      type: 'false',
    });
  } finally {
    memoryCleanupRunning.value = false;
  }
}
function toggleMemoryAutoCleanup() { memoryAutoCleanup.value = !memoryAutoCleanup.value; if (memoryAutoCleanup.value && (memoryCleanupInterval.value === '' || Number(memoryCleanupInterval.value) <= 0)) memoryCleanupInterval.value = '1'; scheduleMemoryCleanup(); }
function handleMemoryIntervalInput(event: Event) { const target = event.target as HTMLInputElement; memoryCleanupInterval.value = target.value.replace(/[^\d]/g, ''); scheduleMemoryCleanup(); }
function finalizeMemoryIntervalInput() { if (!memoryAutoCleanup.value) return; if (memoryCleanupInterval.value === '' || Number(memoryCleanupInterval.value) <= 0) memoryCleanupInterval.value = '1'; scheduleMemoryCleanup(); }
function selectMemoryCleanupUnit(unit: 'seconds' | 'minutes' | 'hours') { if (!memoryAutoCleanup.value) return; memoryCleanupUnit.value = unit; scheduleMemoryCleanup(); }
function toggleCacheSelection(key: keyof typeof cacheSelections.value) { cacheSelections.value[key] = !cacheSelections.value[key]; }
async function runCacheCleanup() {
  if (cacheCleanupRunning.value) return;
  const options: CacheCleanupOptions = {
    system_cache: cacheSelections.value.systemCache,
    temp_files: cacheSelections.value.tempFiles,
    thumbnail_cache: cacheSelections.value.thumbnailCache,
    app_cache: cacheSelections.value.appCache,
    recycle_bin: cacheSelections.value.recycleBin,
  };
  const selectedCount = Object.values(options).filter(Boolean).length;
  if (selectedCount === 0) {
    notify({
      title: t('toolbox.cacheTitle'),
      content: t('toolbox.cacheSelectLabel'),
      type: 'warn',
    });
    return;
  }

  cacheCleanupRunning.value = true;
  try {
    const report = await invoke<CacheCleanupReport>('cleanup_cache', { options });
    const reclaimedMb = (report.reclaimed_bytes / 1024 / 1024).toFixed(2);
    notify({
      title: t('toolbox.cacheTitle'),
      content: t('toolbox.cacheResult', {
        cleaned: report.cleaned,
        reclaimedMb,
      }),
      type: 'true',
    });
  } catch (error) {
    notify({
      title: t('toolbox.cacheTitle'),
      content: String(error),
      type: 'false',
    });
  } finally {
    cacheCleanupRunning.value = false;
  }
}
function updateToolboxScrollbar() { const container = toolboxScrollRef.value; if (!container) { toolboxScrollbarActive.value = false; return; } const { scrollTop, scrollHeight, clientHeight } = container; const hasOverflow = scrollHeight > clientHeight + 1; toolboxScrollbarActive.value = hasOverflow; if (!hasOverflow) { toolboxThumbHeight.value = 0; toolboxThumbOffset.value = 0; return; } const thumbHeight = Math.max((clientHeight / scrollHeight) * clientHeight, 28); const maxThumbOffset = Math.max(clientHeight - thumbHeight, 0); const maxScrollTop = Math.max(scrollHeight - clientHeight, 1); toolboxThumbHeight.value = thumbHeight; toolboxThumbOffset.value = (scrollTop / maxScrollTop) * maxThumbOffset; }
function handleToolboxScroll() { updateToolboxScrollbar(); }
function handleToolboxExpandTransitionEnd() { requestAnimationFrame(updateToolboxScrollbar); }
function handleToolboxScrollbarDragStart(event: MouseEvent) {
  if (!toolboxScrollbarActive.value || !toolboxScrollRef.value) return;
  toolboxDragging = true;
  toolboxDragStartY = event.clientY;
  toolboxDragStartScrollTop = toolboxScrollRef.value.scrollTop;
  event.preventDefault();
}
function handleToolboxScrollbarTrackClick(event: MouseEvent) {
  if (!toolboxScrollbarActive.value || !toolboxScrollRef.value || !toolboxScrollbarRef.value || toolboxDragging) return;
  const trackRect = toolboxScrollbarRef.value.getBoundingClientRect();
  const clickOffset = event.clientY - trackRect.top - toolboxThumbHeight.value / 2;
  const maxThumbOffset = Math.max(trackRect.height - toolboxThumbHeight.value, 0);
  const thumbOffset = Math.max(0, Math.min(clickOffset, maxThumbOffset));
  const maxScrollTop = Math.max(toolboxScrollRef.value.scrollHeight - toolboxScrollRef.value.clientHeight, 0);
  toolboxScrollRef.value.scrollTop = maxThumbOffset > 0 ? (thumbOffset / maxThumbOffset) * maxScrollTop : 0;
  updateToolboxScrollbar();
}
function handleToolboxGlobalMouseMove(event: MouseEvent) {
  if (!toolboxDragging || !toolboxScrollRef.value || !toolboxScrollbarRef.value) return;
  const trackHeight = toolboxScrollbarRef.value.clientHeight;
  const maxThumbOffset = Math.max(trackHeight - toolboxThumbHeight.value, 1);
  const maxScrollTop = Math.max(toolboxScrollRef.value.scrollHeight - toolboxScrollRef.value.clientHeight, 0);
  const deltaY = event.clientY - toolboxDragStartY;
  const nextScrollTop = toolboxDragStartScrollTop + (deltaY / maxThumbOffset) * maxScrollTop;
  toolboxScrollRef.value.scrollTop = Math.max(0, Math.min(nextScrollTop, maxScrollTop));
  updateToolboxScrollbar();
}
function handleToolboxGlobalMouseUp() { toolboxDragging = false; }
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
function formatHotkeyFromEvent(event: KeyboardEvent): string { const parts: string[] = []; let modifierCount = 0; if (event.ctrlKey) { parts.push('Ctrl'); modifierCount += 1; } if (event.altKey) { parts.push('Alt'); modifierCount += 1; } if (event.shiftKey) { parts.push('Shift'); modifierCount += 1; } if (event.metaKey) { parts.push('Win'); modifierCount += 1; } const ignored = ['Control', 'Shift', 'Alt', 'Meta']; let terminalKey = ''; if (!ignored.includes(event.key)) { terminalKey = event.key.length === 1 ? event.key.toUpperCase() : event.key; parts.push(terminalKey); } if (modifierCount === 0 || !terminalKey) return ''; return parts.join('+'); }
function handleHotkeyRecord(event: KeyboardEvent) {
  if (!recordingHotkey.value) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === 'Escape') {
    recordingHotkey.value = false;
    void syncHotkeyWhileListening();
    return;
  }
  const nextHotkey = formatHotkeyFromEvent(event);
  if (!nextHotkey) {
    listenSettingsError.value = t('home.hotkeyHint');
    return;
  }
  listenHotkey.value = nextHotkey;
  recordingHotkey.value = false;
  listenSettingsError.value = '';
  void syncHotkeyWhileListening();
}
function preventWebviewRefresh(event: KeyboardEvent) {
  const key = event.key.toLowerCase();
  const isCtrlOrMetaR = (event.ctrlKey || event.metaKey) && key === 'r';
  const isF5 = event.key === 'F5';
  const isCtrlF5 = (event.ctrlKey || event.metaKey) && isF5;
  if (isCtrlOrMetaR || isF5 || isCtrlF5) {
    event.preventDefault();
    event.stopPropagation();
  }
}
watch([memoryAutoCleanup, memoryCleanupInterval, memoryCleanupUnit], () => {
  scheduleMemoryCleanup();
});
function optionLabel(path: string, value: OptionValue) { return t(`${path}.${value}`); }
function languageLabel(value: LanguageValue) { return languageOptionLabels[value]; }
function openExternalUrl(url: string) { void invoke('open_external_url', { url }); }
function openGithubProfile() { openExternalUrl('https://github.com/SmailPang'); }
function openPrivacyPolicy() { openExternalUrl(PRIVACY_POLICY_URL); }
async function acceptPrivacyConsent() {
  try {
    await invoke<AppConfig>('update_config', {
      patch: {
        privacy_consent: true,
      },
    });
    privacyConsentAccepted.value = true;
    privacyDialogOpen.value = false;
    await runStartupUpdateCheck();
  } catch (error) {
    notify({ title: t('common.saveFailed'), content: String(error), type: 'false' });
  }
}
async function rejectPrivacyConsent() {
  await invoke('exit_app');
}

onMounted(async () => {
  colorSchemeQuery = window.matchMedia('(prefers-color-scheme: dark)');
  prefersDark.value = colorSchemeQuery.matches;
  renderedTheme.value = resolveTheme(theme.value);
  colorSchemeQuery.addEventListener('change', handleColorSchemeChange);
  document.addEventListener('click', handleDocumentClick);
  window.addEventListener('keydown', handleHotkeyRecord, true);
  window.addEventListener('keydown', preventWebviewRefresh, true);
  window.addEventListener('resize', updateToolboxScrollbar);
  window.addEventListener('mousemove', handleToolboxGlobalMouseMove);
  window.addEventListener('mouseup', handleToolboxGlobalMouseUp);
  requestAnimationFrame(updateToolboxScrollbar);
  try {
    await loadConfigFromBackend();
    privacyDialogOpen.value = !privacyConsentAccepted.value;
    await runStartupUpdateCheck();
    await invoke('set_hotkey_enabled', { enabled: false });
    await refreshHomeWindows();
    scheduleMemoryCleanup();
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
  if (themeSwitchTimer !== null) window.clearTimeout(themeSwitchTimer);
  for (const timer of noticeTimers.values()) window.clearTimeout(timer);
  noticeTimers.clear();
  colorSchemeQuery?.removeEventListener('change', handleColorSchemeChange);
  document.removeEventListener('click', handleDocumentClick);
  window.removeEventListener('keydown', handleHotkeyRecord, true);
  window.removeEventListener('keydown', preventWebviewRefresh, true);
  window.removeEventListener('resize', updateToolboxScrollbar);
  window.removeEventListener('mousemove', handleToolboxGlobalMouseMove);
  window.removeEventListener('mouseup', handleToolboxGlobalMouseUp);
  clearMemoryCleanupScheduler();
  updateProgressUnlisten?.();
  updateProgressUnlisten = null;
  openSettingsUnlisten?.();
  openSettingsUnlisten = null;
});
</script>
<template>
  <div ref="appShellRef" :class="['app-shell', `theme-${activeTheme}`]" :style="{ '--font-scale': String(fontScale) }" @contextmenu.prevent>
    <div v-if="themeRipple.visible" :class="['theme-ripple-overlay', `target-${themeRipple.target}`]" :style="{ width: `${themeRipple.size}px`, height: `${themeRipple.size}px`, left: `${themeRipple.x}px`, top: `${themeRipple.y}px` }" @animationend="handleThemeRippleEnd" />
    <TransitionGroup name="save-notice" tag="div" class="save-notice-stack">
      <div v-for="notice in notices" :key="notice.id" :class="['save-notice', notice.type]">
        <div class="save-notice-body">
          <span class="save-notice-title">{{ notice.title }}</span>
          <span v-if="notice.content" class="save-notice-content">{{ notice.content }}</span>
        </div>
        <button class="save-notice-close" type="button" @click="removeSaveNotice(notice.id)">
          {{ t('common.dismiss') }}
        </button>
      </div>
    </TransitionGroup>

    <header class="dynamic-island">
      <button :class="['island-button', { active: currentPage === 'home' }]" type="button" @click="currentPage = 'home'">{{ t('nav.home') }}</button>
      <button :class="['island-button', { active: currentPage === 'toolbox' }]" type="button" @click="currentPage = 'toolbox'">{{ t('nav.toolbox') }}</button>
      <button :class="['island-button', { active: currentPage === 'settings' }]" type="button" @click="currentPage = 'settings'">{{ t('nav.settings') }}</button>
    </header>

    <main class="page-panel">
      <Transition name="page-switch" mode="out-in">
        <section v-if="currentPage === 'home'" key="home" class="page-card home-page">
            <button v-if="listenSettingsOpen" class="home-overlay" type="button" :aria-label="t('home.closeListenSettings')" @click="closeListenSettings" />
            <div class="home-actions">
              <div class="window-list-panel">
                <div class="window-list-header">{{ t('home.windowListTitle') }}</div>
                <div class="window-list-card">
                  <button v-for="item in homeWindows" :key="item.hwnd" :class="['window-list-item', { active: item.hwnd === selectedHomeWindowId }]" type="button" @click="selectHomeWindow(item.hwnd)">
                    <span class="window-list-name">{{ item.title }}</span>
                    <span class="window-list-pid">PID {{ item.hwnd }}</span>
                  </button>
                  <div v-if="!homeWindows.length && !windowsLoading" class="window-list-empty">{{ t('home.noWindows') }}</div>
                  <div v-if="windowsLoading" class="window-list-empty">{{ t('home.loadingWindows') }}</div>
                </div>
              </div>
              <div class="home-secondary-actions">
                <button :class="['listen-settings-button', { active: listenSettingsOpen }]" type="button" @click.stop="toggleListenSettings">{{ t('home.listenSettings') }}</button>
                <button class="window-refresh-button" type="button" @click="refreshHomeWindows">{{ t('home.refresh') }}</button>
              </div>
              <button :class="['listen-button', { listening: isListening }]" type="button" @click="toggleListening"><span class="listen-button-text">{{ isListening ? t('home.stopListening') : t('home.startListening') }}</span></button>
            </div>
          <Transition name="listen-settings-popup">
            <div v-if="listenSettingsOpen" class="listen-settings-popup" @click.stop>
              <div class="listen-settings-group">
                <span class="listen-settings-label">{{ t('home.hotkey') }}</span>
                <div class="listen-hotkey-actions">
                  <button :class="['listen-hotkey-trigger', { recording: recordingHotkey }]" type="button" @click="toggleHotkeyRecording">{{ hotkeyButtonLabel }}</button>
                  <button v-if="listenHotkey" class="listen-hotkey-clear" type="button" @click="clearHotkey">{{ t('common.clear') }}</button>
                </div>
              </div>
              <label class="listen-checkbox-row" @click.prevent="toggleMouseSideButton">
                <input :checked="listenMouseSideButton" class="listen-checkbox-input" type="checkbox" />
                <span class="listen-checkbox-box" />
                <span class="listen-checkbox-label">{{ t('home.mouseSideButton') }}</span>
              </label>
              <p v-if="listenSettingsError" class="listen-settings-error">{{ listenSettingsError }}</p>
            </div>
          </Transition>
        </section>

        <section v-else-if="currentPage === 'toolbox'" key="toolbox" class="page-card toolbox-page">
          <div class="toolbox-scroll-shell">
            <div ref="toolboxScrollRef" class="toolbox-actions" @scroll="handleToolboxScroll">
              <div :class="['toolbox-action-card', { active: activeToolboxCard === 'memory' }]" :aria-expanded="activeToolboxCard === 'memory'" role="button" tabindex="0" @click="toggleToolboxCard('memory')">
                <div class="toolbox-action-main">
                  <span class="toolbox-action-title">{{ t('toolbox.memoryTitle') }}</span>
                  <span class="toolbox-action-subtitle">{{ t('toolbox.memorySubtitle') }}</span>
                </div>
                <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
                  <div v-if="activeToolboxCard === 'memory'" class="toolbox-action-detail">
                    <div class="toolbox-setting-row" @click.stop>
                      <span class="toolbox-setting-label">{{ t('toolbox.memoryAutoCleanup') }}</span>
                      <button :class="['settings-switch', 'toolbox-switch', { active: memoryAutoCleanup }]" type="button" role="switch" :aria-checked="memoryAutoCleanup" @click.stop="toggleMemoryAutoCleanup"><span class="settings-switch-thumb" /></button>
                    </div>
                    <div class="toolbox-setting-block" @click.stop>
                      <span class="toolbox-setting-label">{{ t('toolbox.memoryInterval') }}</span>
                      <div class="toolbox-interval-control">
                        <input class="toolbox-interval-input" type="text" inputmode="numeric" :value="memoryCleanupInterval" :disabled="!memoryAutoCleanup" @input="handleMemoryIntervalInput" @blur="finalizeMemoryIntervalInput" @click.stop />
                        <div class="toolbox-unit-group">
                          <button :class="['toolbox-unit-button', { active: memoryCleanupUnit === 'seconds' }]" type="button" :disabled="!memoryAutoCleanup" @click.stop="selectMemoryCleanupUnit('seconds')">{{ t('toolbox.seconds') }}</button>
                          <button :class="['toolbox-unit-button', { active: memoryCleanupUnit === 'minutes' }]" type="button" :disabled="!memoryAutoCleanup" @click.stop="selectMemoryCleanupUnit('minutes')">{{ t('toolbox.minutes') }}</button>
                          <button :class="['toolbox-unit-button', { active: memoryCleanupUnit === 'hours' }]" type="button" :disabled="!memoryAutoCleanup" @click.stop="selectMemoryCleanupUnit('hours')">{{ t('toolbox.hours') }}</button>
                        </div>
                      </div>
                      <p v-if="memoryAutoCleanup && memoryCleanupIntervalInvalid" class="toolbox-field-error">{{ t('toolbox.intervalInvalid') }}</p>
                    </div>
                    <button class="toolbox-action-button" type="button" :disabled="memoryCleanupRunning" @click.stop="runMemoryCleanup()">{{ memoryCleanupRunning ? t('toolbox.cleaning') : t('toolbox.runNow') }}</button>
                  </div>
                </Transition>
              </div>

              <div :class="['toolbox-action-card', { active: activeToolboxCard === 'cache' }]" :aria-expanded="activeToolboxCard === 'cache'" role="button" tabindex="0" @click="toggleToolboxCard('cache')">
                <div class="toolbox-action-main">
                  <span class="toolbox-action-title">{{ t('toolbox.cacheTitle') }}</span>
                  <span class="toolbox-action-subtitle">{{ t('toolbox.cacheSubtitle') }}</span>
                </div>
                <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
                  <div v-if="activeToolboxCard === 'cache'" class="toolbox-action-detail">
                    <div class="toolbox-setting-block" @click.stop>
                      <span class="toolbox-setting-label">{{ t('toolbox.cacheSelectLabel') }}</span>
                      <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.systemCache" class="toolbox-check-input" type="checkbox" @change="toggleCacheSelection('systemCache')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.systemCache') }}</span></label>
                      <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.tempFiles" class="toolbox-check-input" type="checkbox" @change="toggleCacheSelection('tempFiles')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.tempFiles') }}</span></label>
                      <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.thumbnailCache" class="toolbox-check-input" type="checkbox" @change="toggleCacheSelection('thumbnailCache')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.thumbnailCache') }}</span></label>
                      <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.appCache" class="toolbox-check-input" type="checkbox" @change="toggleCacheSelection('appCache')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.appCache') }}</span></label>
                      <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.recycleBin" class="toolbox-check-input" type="checkbox" @change="toggleCacheSelection('recycleBin')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.recycleBin') }}</span></label>
                      <button class="toolbox-action-button" type="button" :disabled="cacheCleanupRunning" @click.stop="runCacheCleanup()">{{ cacheCleanupRunning ? t('toolbox.cleaning') : t('toolbox.runNow') }}</button>
                    </div>
                  </div>
                </Transition>
              </div>

              <div :class="['toolbox-action-card', { active: activeToolboxCard === 'feedback' }]" :aria-expanded="activeToolboxCard === 'feedback'" role="button" tabindex="0" @click="toggleToolboxCard('feedback')">
                <span class="toolbox-action-title">{{ t('toolbox.feedbackTitle') }}</span>
                <span class="toolbox-action-subtitle">{{ t('toolbox.feedbackSubtitle') }}</span>
                <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
                  <div v-if="activeToolboxCard === 'feedback'" class="toolbox-action-detail">
                    <div class="toolbox-feedback-note" @click.stop>{{ t('toolbox.feedbackNotice') }}</div>
                    <div class="toolbox-feedback-actions" @click.stop>
                      <button class="toolbox-feedback-button" type="button" @click.stop="openExternalUrl('https://github.com/SmailPang/SkiHide/issues')">{{ t('toolbox.feedbackIssues') }}</button>
                      <button class="toolbox-feedback-button" type="button" @click.stop="openExternalUrl('https://www.bilibili.com/video/BV1wkvaBmEZP')">{{ t('toolbox.feedbackGuide') }}</button>
                    </div>
                  </div>
                </Transition>
              </div>

              <div :class="['toolbox-action-card', { active: activeToolboxCard === 'about' }]" :aria-expanded="activeToolboxCard === 'about'" role="button" tabindex="0" @click="toggleToolboxCard('about')">
                <span class="toolbox-action-title">{{ t('toolbox.aboutTitle') }}</span>
                <span class="toolbox-action-subtitle">{{ t('toolbox.aboutSubtitle') }}</span>
                <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
                  <div v-if="activeToolboxCard === 'about'" class="toolbox-action-detail">
                    <div class="toolbox-about-panel" @click.stop><img class="toolbox-about-icon" :src="appIconUrl" alt="SkiHide icon" /></div>
                    <div class="toolbox-about-profile" @click.stop>
                      <img class="toolbox-about-avatar" :src="avatarUrl" alt="SmailPang avatar" />
                      <div class="toolbox-about-meta">
                        <span class="toolbox-about-name">SmailPang</span>
                        <span class="toolbox-about-role">{{ t('toolbox.developer') }}</span>
                      </div>
                      <button class="toolbox-about-github" type="button" aria-label="Open GitHub profile" @click.stop="openGithubProfile"><svg class="toolbox-about-github-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2C6.48 2 2 6.58 2 12.23c0 4.52 2.87 8.35 6.84 9.7.5.1.68-.22.68-.49 0-.24-.01-1.05-.01-1.9-2.78.62-3.37-1.21-3.37-1.21-.46-1.19-1.11-1.51-1.11-1.51-.91-.64.07-.63.07-.63 1 .07 1.53 1.06 1.53 1.06.9 1.57 2.36 1.12 2.94.86.09-.67.35-1.12.63-1.38-2.22-.26-4.56-1.14-4.56-5.1 0-1.13.39-2.05 1.03-2.78-.1-.26-.45-1.32.1-2.76 0 0 .84-.27 2.75 1.06A9.3 9.3 0 0 1 12 6.84c.85 0 1.71.12 2.51.35 1.91-1.33 2.75-1.06 2.75-1.06.55 1.44.2 2.5.1 2.76.64.73 1.03 1.65 1.03 2.78 0 3.97-2.34 4.83-4.57 5.09.36.32.68.95.68 1.92 0 1.39-.01 2.5-.01 2.84 0 .27.18.59.69.49A10.25 10.25 0 0 0 22 12.23C22 6.58 17.52 2 12 2Z" /></svg></button>
                    </div>
                    <div class="toolbox-about-links" @click.stop>
                      <div class="toolbox-about-links-title">{{ t('toolbox.friendLinks') }}</div>
                      <div class="toolbox-about-link-card">
                        <img class="toolbox-about-link-logo" :src="raincloudUrl" alt="RainCloud logo" />
                        <div class="toolbox-about-link-meta">
                          <span class="toolbox-about-link-name">{{ t('toolbox.raincloudName') }}</span>
                          <span class="toolbox-about-link-desc">{{ t('toolbox.raincloudDesc') }}</span>
                        </div>
                        <button class="toolbox-about-link-button" type="button" :aria-label="t('toolbox.openLink')" @click.stop="openExternalUrl('https://www.rainyun.com/Pang_')">
                          <svg class="toolbox-about-link-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4h6v6h-2V7.41l-8.29 8.3-1.42-1.42 8.3-8.29H14V4Zm4 14V11h2v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h8v2H6v12h12Z" /></svg>
                        </button>
                      </div>
                      <div class="toolbox-about-link-card">
                        <img class="toolbox-about-link-logo" :src="mirrorchyanUrl" alt="MirrorChyan logo" />
                        <div class="toolbox-about-link-meta">
                          <span class="toolbox-about-link-name">{{ t('toolbox.mirrorchyanName') }}</span>
                          <span class="toolbox-about-link-desc">{{ t('toolbox.mirrorchyanDesc') }}</span>
                        </div>
                        <button class="toolbox-about-link-button" type="button" :aria-label="t('toolbox.openLink')" @click.stop="openExternalUrl('https://mirrorchyan.com/zh/get-start?source=skihide-client')">
                          <svg class="toolbox-about-link-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4h6v6h-2V7.41l-8.29 8.3-1.42-1.42 8.3-8.29H14V4Zm4 14V11h2v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h8v2H6v12h12Z" /></svg>
                        </button>
                      </div>
                    </div>
                    <div class="toolbox-about-version" @click.stop>V2.0.0</div>
                    <button class="toolbox-action-button toolbox-about-update" type="button" @click.stop="openUpdateDialog">{{ t('toolbox.checkUpdates') }}</button>
                  </div>
                </Transition>
              </div>

              <button class="toolbox-action-card" type="button" @click="openDangerDialog"><span class="toolbox-action-title">{{ t('toolbox.dangerTitle') }}</span><span class="toolbox-action-subtitle">{{ t('toolbox.dangerSubtitle') }}</span></button>
            </div>
            <Transition name="toolbox-scrollbar-fade"><div v-if="toolboxScrollbarActive" ref="toolboxScrollbarRef" class="toolbox-scrollbar" @mousedown="handleToolboxScrollbarTrackClick"><div class="toolbox-scrollbar-thumb" :style="{ height: `${toolboxThumbHeight}px`, transform: `translateY(${toolboxThumbOffset}px)` }" @mousedown.stop="handleToolboxScrollbarDragStart" /></div></Transition>
          </div>
        </section>

        <section v-else key="settings" class="page-card settings-page">
          <div class="settings-section">
            <div class="settings-section-title">{{ t('settings.appearance') }}</div>
            <div class="settings-row"><span class="settings-label">{{ t('settings.language') }}</span><div class="custom-select"><button :class="['custom-select-trigger', { open: languageOpen }]" type="button" @click.stop="toggleLanguageMenu"><span>{{ currentLanguageLabel }}</span></button><Transition name="dropdown-fade"><div v-if="languageOpen" :class="['custom-select-menu', { upward: languageMenuUp }]"><button v-for="option in languageOptionValues" :key="option" :class="['custom-select-option', { active: option === language }]" type="button" @click.stop="selectLanguage(option)">{{ languageLabel(option) }}</button></div></Transition></div></div>
            <div class="settings-row"><span class="settings-label">{{ t('settings.theme') }}</span><div class="custom-select"><button ref="themeTriggerRef" :class="['custom-select-trigger', { open: themeOpen }]" type="button" @click.stop="toggleThemeMenu"><span>{{ currentThemeLabel }}</span></button><Transition name="dropdown-fade"><div v-if="themeOpen" :class="['custom-select-menu', { upward: themeMenuUp }]"><button v-for="option in themeOptionValues" :key="option" :class="['custom-select-option', { active: option === theme }]" type="button" @click.stop="selectTheme(option)">{{ optionLabel('optionLabels.theme', option) }}</button></div></Transition></div></div>
            <div class="settings-row"><span class="settings-label">{{ t('settings.fontSize') }}</span><div class="custom-select"><button :class="['custom-select-trigger', { open: fontSizeOpen }]" type="button" @click.stop="toggleFontSizeMenu"><span>{{ currentFontSizeLabel }}</span></button><Transition name="dropdown-fade"><div v-if="fontSizeOpen" :class="['custom-select-menu', { upward: fontSizeMenuUp }]"><button v-for="option in fontSizeOptionValues" :key="option" :class="['custom-select-option', { active: option === fontSize }]" type="button" @click.stop="selectFontSize(option)">{{ optionLabel('optionLabels.fontSize', option) }}</button></div></Transition></div></div>
            <div class="settings-section-title settings-section-title-program">{{ t('settings.program') }}</div>
            <div class="settings-row"><span class="settings-label">{{ t('settings.autoStart') }}</span><button :class="['settings-switch', { active: autoStart }]" type="button" role="switch" :aria-checked="autoStart" @click="toggleAutoStart"><span class="settings-switch-thumb" /></button></div>
            <div class="settings-row"><span class="settings-label settings-label-child">{{ t('settings.silentStart') }}</span><button :class="['settings-switch', { active: silentStart, disabled: silentStartDisabled }]" type="button" role="switch" :aria-checked="silentStart" :aria-disabled="silentStartDisabled" @click="toggleSilentStart"><span class="settings-switch-thumb" /></button></div>
            <div class="settings-row"><span class="settings-label">{{ t('settings.muteOnHide') }}</span><button :class="['settings-switch', { active: muteOnHide }]" type="button" role="switch" :aria-checked="muteOnHide" @click="muteOnHide = !muteOnHide"><span class="settings-switch-thumb" /></button></div>
            <div class="settings-section-title settings-section-title-program">{{ t('settings.updates') }}</div>
            <div class="settings-row"><span class="settings-label">{{ t('settings.autoCheckUpdates') }}</span><button :class="['settings-switch', { active: autoCheckUpdates }]" type="button" role="switch" :aria-checked="autoCheckUpdates" @click="autoCheckUpdates = !autoCheckUpdates"><span class="settings-switch-thumb" /></button></div>
            <div class="settings-row">
              <span class="settings-label settings-label-with-hint">
                {{ t('settings.updateSource') }}
                <span class="settings-hint" tabindex="0">
                  <span class="settings-hint-icon" aria-hidden="true">i</span>
                  <span class="settings-hint-tooltip">{{ t('settings.sourceHint') }}</span>
                </span>
              </span>
              <div class="custom-select">
                <button :class="['custom-select-trigger', { open: updateSourceOpen }]" type="button" @click.stop="toggleUpdateSourceMenu"><span>{{ currentUpdateSourceLabel }}</span></button>
                <Transition name="dropdown-fade">
                  <div v-if="updateSourceOpen" :class="['custom-select-menu', { upward: updateSourceMenuUp }]">
                    <button v-for="option in updateSourceOptionValues" :key="option" :class="['custom-select-option', { active: option === updateSource }]" type="button" @click.stop="selectUpdateSource(option)">{{ optionLabel('optionLabels.updateSource', option) }}</button>
                  </div>
                </Transition>
              </div>
            </div>
            <div class="settings-row">
              <span class="settings-label settings-label-with-hint">
                {{ t('settings.downloadSource') }}
                <span class="settings-hint" tabindex="0">
                  <span class="settings-hint-icon" aria-hidden="true">i</span>
                  <span class="settings-hint-tooltip">{{ t('settings.sourceHint') }}</span>
                </span>
              </span>
              <div class="custom-select">
                <button :class="['custom-select-trigger', { open: downloadSourceOpen }]" type="button" @click.stop="toggleDownloadSourceMenu"><span>{{ currentDownloadSourceLabel }}</span></button>
                <Transition name="dropdown-fade">
                  <div v-if="downloadSourceOpen" :class="['custom-select-menu', { upward: downloadSourceMenuUp }]">
                    <button v-for="option in availableDownloadSourceOptionValues" :key="option" :class="['custom-select-option', { active: option === downloadSource }]" type="button" @click.stop="selectDownloadSource(option)">{{ optionLabel('optionLabels.downloadSource', option) }}</button>
                  </div>
                </Transition>
              </div>
            </div>
            <div class="settings-row">
              <span class="settings-label">{{ t('settings.mirrorChanSdk') }}</span>
              <button class="settings-edit-button" type="button" @click="openMirrorChanSdkDialog">{{ t('common.edit') }}</button>
            </div>
          </div>
          <Transition name="bottom-actions"><div v-if="settingsDirty" class="settings-actions"><button class="settings-action-button primary" type="button" @click="saveSettings">{{ t('common.save') }}</button><button class="settings-action-button secondary" type="button" @click="discardSettings">{{ t('common.discard') }}</button></div></Transition>
        </section>
      </Transition>
    </main>

    <Transition name="dialog-fade">
      <div v-if="privacyDialogOpen" class="dialog-overlay" @click.stop>
        <div class="dialog-panel privacy-dialog-panel" @click.stop>
          <div class="dialog-title">{{ t('privacyDialog.title') }}</div>
          <div class="dialog-description">
            {{ t('privacyDialog.prefix') }}
            <button class="privacy-inline-link" type="button" @click="openPrivacyPolicy">{{ t('privacyDialog.policyAndDisclaimer') }}</button>
            {{ t('privacyDialog.suffix') }}
          </div>
          <div class="dialog-actions">
            <button class="dialog-action-button primary" type="button" @click="acceptPrivacyConsent">{{ t('privacyDialog.accept') }}</button>
            <button class="dialog-action-button secondary" type="button" @click="rejectPrivacyConsent">{{ t('privacyDialog.reject') }}</button>
          </div>
        </div>
      </div>
    </Transition>

    <Transition name="dialog-fade">
      <div v-if="mirrorChanSdkDialogOpen" class="dialog-overlay" @click="cancelMirrorChanSdkDialog">
        <div class="dialog-panel" @click.stop>
          <div class="dialog-title">{{ t('settings.mirrorChanSdkDialogTitle') }}</div>
          <div class="dialog-description">{{ t('settings.mirrorChanSdkDialogDesc') }}</div>
          <input v-model="mirrorChanSdkDraft" class="dialog-input" type="text" :placeholder="t('settings.mirrorChanSdkPlaceholder')" />
          <div v-if="mirrorChanSdkError" class="dialog-error">{{ mirrorChanSdkError }}</div>
          <button class="dialog-link-button" type="button" @click="openExternalUrl('https://mirrorchyan.com/zh/get-start?source=skihide-client')">
            {{ t('settings.getMirrorChanSdk') }}
          </button>
          <div class="dialog-actions">
            <button class="dialog-action-button primary" type="button" @click="saveMirrorChanSdk">{{ t('common.confirm') }}</button>
            <button class="dialog-action-button secondary" type="button" @click="cancelMirrorChanSdkDialog">{{ t('common.cancel') }}</button>
          </div>
        </div>
      </div>
    </Transition>

    <Transition name="dialog-fade">
      <div v-if="updateDialogOpen" class="dialog-overlay" @click="closeUpdateDialog">
        <div class="dialog-panel update-dialog-panel" @click.stop>
          <div class="dialog-title">{{ t('updateDialog.title') }}</div>
          <div class="update-dialog-version">{{ t('updateDialog.latestVersion') }} {{ latestVersion }}</div>
          <div class="update-dialog-log">
            <div class="update-dialog-log-title">{{ t('updateDialog.changelog') }}</div>
            <div class="update-dialog-markdown" v-html="renderedUpdateChangelog" />
          </div>
          <div v-if="updateInProgress || updateProgress > 0" class="update-progress-block">
            <div class="update-progress-track">
              <div class="update-progress-fill" :style="{ width: `${updateProgress}%` }" />
            </div>
            <div class="update-progress-text">{{ t('updateDialog.downloading') }} {{ updateProgress }}%</div>
          </div>
          <button v-if="!mirrorChanSdkConfigured" class="update-dialog-sdk-button" type="button" @click="openMirrorChanSdkDialogFromUpdate">
            {{ t('updateDialog.fillMirrorChanSdk') }}
          </button>
          <div class="dialog-actions">
            <button class="dialog-action-button primary" type="button" :disabled="updateInProgress || updateClosePromptOpen" @click="runImmediateUpdate">{{ updateInProgress ? t('updateDialog.updatingNow') : t('updateDialog.updateNow') }}</button>
            <button class="dialog-action-button secondary" type="button" :disabled="updateInProgress || updateClosePromptOpen" @click="closeUpdateDialog">{{ t('updateDialog.cancelUpdate') }}</button>
          </div>
        </div>
      </div>
    </Transition>

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

    <Transition name="dialog-fade">
      <div v-if="dangerDialogOpen" class="dialog-overlay" @click="closeDangerDialog">
        <div class="dialog-panel" @click.stop>
          <div class="dialog-title">{{ t('toolbox.dangerTitle') }}</div>
          <div class="dialog-description dialog-description-preline">{{ t('toolbox.dangerWarning') }}</div>
          <div class="dialog-actions dialog-actions-triple">
            <button class="dialog-action-button danger" type="button" @click="continueDangerAction">{{ t('toolbox.dangerContinue') }}</button>
            <button class="dialog-action-button secondary" type="button" @click="continueDangerAction">{{ t('toolbox.dangerContinue') }}</button>
            <button class="dialog-action-button secondary" type="button" @click="continueDangerAction">{{ t('toolbox.dangerContinue') }}</button>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

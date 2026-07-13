<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  languageOptionValues,
  themeOptionValues,
  fontSizeOptionValues,
  updateSourceOptionValues,
  updateChannelOptionValues,
  downloadSourceOptionValues,
  type LanguageValue,
} from '../composables/useAppSettings';

const BUILTIN_FONT_STACK = '"HarmonyOS Sans SC", "Microsoft YaHei UI", "Segoe UI", sans-serif';
const DEFAULT_FONT_FAMILY = '';

const languageOptionLabels: Record<LanguageValue, string> = {
  zh_CN: '简体中文',
  zh_TW: '繁體中文',
  en_US: 'English',
  ja_JP: '日本語',
};

const props = defineProps<{
  language: LanguageValue;
  theme: 'system' | 'light' | 'dark';
  fontFamily: string;
  fontSize: 'small' | 'medium' | 'large' | 'xlarge';
  autoStart: boolean;
  silentStart: boolean;
  muteOnHide: boolean;
  pauseOnHide: boolean;
  pauseHotkey: string;
  updateSource: 'mirror_chan' | 'skihide';
  updateChannel: 'stable' | 'beta';
  mirrorChanSdk: string;
  downloadSource: 'mirror_chan' | 'github' | 'cnb';
  autoCheckUpdates: boolean;
  settingsDirty: boolean;
  silentStartDisabled: boolean;
  pauseHotkeyError: string;
  recordingPauseHotkey: boolean;
  systemFonts: string[];
}>();

const emit = defineEmits<{
  'update:language': [value: LanguageValue];
  'update:theme': [value: 'system' | 'light' | 'dark'];
  'update:fontFamily': [value: string];
  'update:fontSize': [value: 'small' | 'medium' | 'large' | 'xlarge'];
  'toggle-auto-start': [];
  'toggle-silent-start': [];
  'toggle-mute-on-hide': [];
  'toggle-pause-on-hide': [];
  'toggle-auto-check-updates': [];
  'select-update-source': [value: 'mirror_chan' | 'skihide'];
  'select-update-channel': [value: 'stable' | 'beta'];
  'select-download-source': [value: 'mirror_chan' | 'github' | 'cnb'];
  'open-mirror-cdk-dialog': [];
  'toggle-pause-hotkey-recording': [];
  'clear-pause-hotkey': [];
  save: [];
  discard: [];
}>();

const { t } = useI18n();

// Expose theme trigger ref so App.vue can wire it to animateThemeChange
const themeTriggerRef = ref<HTMLElement | null>(null);
defineExpose({ themeTriggerRef });

// Internal menu open/up states
const languageOpen = ref(false);
const languageMenuUp = ref(false);
const themeOpen = ref(false);
const themeMenuUp = ref(false);
const fontFamilyOpen = ref(false);
const fontFamilyMenuUp = ref(false);
const fontSizeOpen = ref(false);
const fontSizeMenuUp = ref(false);
const updateSourceOpen = ref(false);
const updateSourceMenuUp = ref(false);
const updateChannelOpen = ref(false);
const updateChannelMenuUp = ref(false);
const downloadSourceOpen = ref(false);
const downloadSourceMenuUp = ref(false);

function closeAllMenus() {
  languageOpen.value = false;
  themeOpen.value = false;
  fontFamilyOpen.value = false;
  fontSizeOpen.value = false;
  updateSourceOpen.value = false;
  updateChannelOpen.value = false;
  downloadSourceOpen.value = false;
}

function shouldOpenMenuUp(trigger: HTMLElement | null, optionCount: number): boolean {
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

// Internal computeds (derived from props)
const currentLanguageLabel = computed(() => languageOptionLabels[props.language]);
const currentThemeLabel = computed(() => t(`optionLabels.theme.${props.theme}`));
const currentFontFamilyLabel = computed(() => props.fontFamily || t('settings.fontFamilyDefault'));
const currentFontSizeLabel = computed(() => t(`optionLabels.fontSize.${props.fontSize}`));
const currentUpdateSourceLabel = computed(() => t(`optionLabels.updateSource.${props.updateSource}`));
const currentUpdateChannelLabel = computed(() => t(`optionLabels.updateChannel.${props.updateChannel}`));
const currentDownloadSourceLabel = computed(() => t(`optionLabels.downloadSource.${props.downloadSource}`));

const fontFamilySelectOptions = computed(() => {
  const options: string[] = [DEFAULT_FONT_FAMILY];
  const seen = new Set<string>(['']);
  if (props.fontFamily && !seen.has(props.fontFamily)) {
    options.push(props.fontFamily);
    seen.add(props.fontFamily);
  }
  for (const name of props.systemFonts) {
    if (!seen.has(name)) { options.push(name); seen.add(name); }
  }
  return options;
});

const availableUpdateSourceOptions = computed(() =>
  props.updateChannel === 'beta'
    ? updateSourceOptionValues.filter((o) => o !== 'skihide')
    : [...updateSourceOptionValues],
);

const mirrorChanSdkConfigured = computed(() => props.mirrorChanSdk.trim().length > 0);
const availableDownloadSourceOptions = computed(() => {
  let options = [...downloadSourceOptionValues];
  if (!mirrorChanSdkConfigured.value) options = options.filter((o) => o !== 'mirror_chan');
  return options;
});

const pauseHotkeyButtonLabel = computed(() =>
  props.recordingPauseHotkey ? t('settings.recordingPauseHotkey') : props.pauseHotkey || t('settings.bindPauseHotkey'),
);

function fontFamilyOptionLabel(value: string) { return value ? value : t('settings.fontFamilyDefault'); }
function fontFamilyOptionStyle(value: string) {
  return value ? { fontFamily: `"${value.replace(/"/g, '\\"')}", ${BUILTIN_FONT_STACK}` } : undefined;
}

// Menu toggle helpers
function toggleLanguageMenu(event: MouseEvent) {
  const next = !languageOpen.value;
  languageMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement, languageOptionValues.length);
  closeAllMenus(); languageOpen.value = next;
}
function selectLanguage(value: LanguageValue) {
  emit('update:language', value); languageOpen.value = false;
}
function toggleThemeMenu(event: MouseEvent) {
  const next = !themeOpen.value;
  themeMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement, themeOptionValues.length);
  closeAllMenus(); themeOpen.value = next;
}
function selectTheme(value: 'system' | 'light' | 'dark') {
  emit('update:theme', value); themeOpen.value = false;
}
function toggleFontFamilyMenu(event: MouseEvent) {
  const next = !fontFamilyOpen.value;
  fontFamilyMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement, Math.min(fontFamilySelectOptions.value.length, 8));
  closeAllMenus(); fontFamilyOpen.value = next;
}
function selectFontFamily(value: string) {
  emit('update:fontFamily', value); fontFamilyOpen.value = false;
}
function toggleFontSizeMenu(event: MouseEvent) {
  const next = !fontSizeOpen.value;
  fontSizeMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement, fontSizeOptionValues.length);
  closeAllMenus(); fontSizeOpen.value = next;
}
function selectFontSize(value: 'small' | 'medium' | 'large' | 'xlarge') {
  emit('update:fontSize', value); fontSizeOpen.value = false;
}
function toggleUpdateSourceMenu(event: MouseEvent) {
  const next = !updateSourceOpen.value;
  updateSourceMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement, availableUpdateSourceOptions.value.length);
  closeAllMenus(); updateSourceOpen.value = next;
}
function selectUpdateSource(value: 'mirror_chan' | 'skihide') {
  emit('select-update-source', value); updateSourceOpen.value = false;
}
function toggleUpdateChannelMenu(event: MouseEvent) {
  const next = !updateChannelOpen.value;
  updateChannelMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement, updateChannelOptionValues.length);
  closeAllMenus(); updateChannelOpen.value = next;
}
function selectUpdateChannel(value: 'stable' | 'beta') {
  emit('select-update-channel', value); updateChannelOpen.value = false;
}
function toggleDownloadSourceMenu(event: MouseEvent) {
  const next = !downloadSourceOpen.value;
  downloadSourceMenuUp.value = shouldOpenMenuUp(event.currentTarget as HTMLElement, availableDownloadSourceOptions.value.length);
  closeAllMenus(); downloadSourceOpen.value = next;
}
function selectDownloadSource(value: 'mirror_chan' | 'github' | 'cnb') {
  if (value === 'mirror_chan' && !mirrorChanSdkConfigured.value) return;
  emit('select-download-source', value); downloadSourceOpen.value = false;
}
</script>

<template>
  <section class="page-card settings-page">
    <div class="settings-section">
      <div class="settings-section-title">{{ t('settings.appearance') }}</div>

      <!-- Language -->
      <div class="settings-row">
        <span class="settings-label">{{ t('settings.language') }}</span>
        <div class="custom-select">
          <button :class="['custom-select-trigger', { open: languageOpen }]" type="button" @click.stop="toggleLanguageMenu"><span>{{ currentLanguageLabel }}</span></button>
          <Transition name="dropdown-fade">
            <div v-if="languageOpen" :class="['custom-select-menu', { upward: languageMenuUp }]">
              <button v-for="option in languageOptionValues" :key="option" :class="['custom-select-option', { active: option === language }]" type="button" @click.stop="selectLanguage(option)">{{ languageOptionLabels[option] }}</button>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Theme -->
      <div class="settings-row">
        <span class="settings-label">{{ t('settings.theme') }}</span>
        <div class="custom-select">
          <button ref="themeTriggerRef" :class="['custom-select-trigger', { open: themeOpen }]" type="button" @click.stop="toggleThemeMenu"><span>{{ currentThemeLabel }}</span></button>
          <Transition name="dropdown-fade">
            <div v-if="themeOpen" :class="['custom-select-menu', { upward: themeMenuUp }]">
              <button v-for="option in themeOptionValues" :key="option" :class="['custom-select-option', { active: option === theme }]" type="button" @click.stop="selectTheme(option)">{{ t(`optionLabels.theme.${option}`) }}</button>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Font Family -->
      <div class="settings-row">
        <span class="settings-label">{{ t('settings.fontFamily') }}</span>
        <div class="custom-select">
          <button :class="['custom-select-trigger', { open: fontFamilyOpen }]" type="button" @click.stop="toggleFontFamilyMenu"><span>{{ currentFontFamilyLabel }}</span></button>
          <Transition name="dropdown-fade">
            <div v-if="fontFamilyOpen" :class="['custom-select-menu', 'custom-select-menu-font', { upward: fontFamilyMenuUp }]">
              <button v-for="option in fontFamilySelectOptions" :key="option || '__default__'" :class="['custom-select-option', 'custom-select-option-font-preview', { active: option === fontFamily }]" type="button" :style="fontFamilyOptionStyle(option)" @click.stop="selectFontFamily(option)">{{ fontFamilyOptionLabel(option) }}</button>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Font Size -->
      <div class="settings-row">
        <span class="settings-label">{{ t('settings.fontSize') }}</span>
        <div class="custom-select">
          <button :class="['custom-select-trigger', { open: fontSizeOpen }]" type="button" @click.stop="toggleFontSizeMenu"><span>{{ currentFontSizeLabel }}</span></button>
          <Transition name="dropdown-fade">
            <div v-if="fontSizeOpen" :class="['custom-select-menu', { upward: fontSizeMenuUp }]">
              <button v-for="option in fontSizeOptionValues" :key="option" :class="['custom-select-option', { active: option === fontSize }]" type="button" @click.stop="selectFontSize(option)">{{ t(`optionLabels.fontSize.${option}`) }}</button>
            </div>
          </Transition>
        </div>
      </div>

      <div class="settings-section-title settings-section-title-program">{{ t('settings.program') }}</div>

      <div class="settings-row"><span class="settings-label">{{ t('settings.autoStart') }}</span><button :class="['settings-switch', { active: autoStart }]" type="button" role="switch" :aria-checked="autoStart" @click="emit('toggle-auto-start')"><span class="settings-switch-thumb" /></button></div>
      <div class="settings-row"><span class="settings-label settings-label-child">{{ t('settings.silentStart') }}</span><button :class="['settings-switch', { active: silentStart, disabled: silentStartDisabled }]" type="button" role="switch" :aria-checked="silentStart" :aria-disabled="silentStartDisabled" @click="emit('toggle-silent-start')"><span class="settings-switch-thumb" /></button></div>
      <div class="settings-row"><span class="settings-label">{{ t('settings.muteOnHide') }}</span><button :class="['settings-switch', { active: muteOnHide }]" type="button" role="switch" :aria-checked="muteOnHide" @click="emit('toggle-mute-on-hide')"><span class="settings-switch-thumb" /></button></div>
      <div class="settings-row"><span class="settings-label">{{ t('settings.pauseOnHide') }}</span><button :class="['settings-switch', { active: pauseOnHide }]" type="button" role="switch" :aria-checked="pauseOnHide" @click="emit('toggle-pause-on-hide')"><span class="settings-switch-thumb" /></button></div>
      <div class="settings-row settings-row-stack">
        <span class="settings-label settings-label-child">{{ t('settings.pauseHotkey') }}</span>
        <div class="listen-hotkey-actions">
          <button :class="['listen-hotkey-trigger', { recording: recordingPauseHotkey }]" type="button" @click="emit('toggle-pause-hotkey-recording')">{{ pauseHotkeyButtonLabel }}</button>
          <button v-if="pauseHotkey" class="listen-hotkey-clear" type="button" @click="emit('clear-pause-hotkey')">{{ t('common.clear') }}</button>
        </div>
      </div>
      <div v-if="pauseHotkeyError" class="settings-inline-error">{{ pauseHotkeyError }}</div>

      <div class="settings-section-title settings-section-title-program">{{ t('settings.updates') }}</div>
      <div class="settings-row"><span class="settings-label">{{ t('settings.autoCheckUpdates') }}</span><button :class="['settings-switch', { active: autoCheckUpdates }]" type="button" role="switch" :aria-checked="autoCheckUpdates" @click="emit('toggle-auto-check-updates')"><span class="settings-switch-thumb" /></button></div>

      <!-- Update Source -->
      <div class="settings-row">
        <span class="settings-label settings-label-with-hint">
          {{ t('settings.updateSource') }}
          <span class="settings-hint" tabindex="0"><span class="settings-hint-icon" aria-hidden="true">i</span><span class="settings-hint-tooltip">{{ t('settings.sourceHint') }}</span></span>
        </span>
        <div class="custom-select">
          <button :class="['custom-select-trigger', { open: updateSourceOpen }]" type="button" @click.stop="toggleUpdateSourceMenu"><span>{{ currentUpdateSourceLabel }}</span></button>
          <Transition name="dropdown-fade">
            <div v-if="updateSourceOpen" :class="['custom-select-menu', { upward: updateSourceMenuUp }]">
              <button v-for="option in availableUpdateSourceOptions" :key="option" :class="['custom-select-option', { active: option === updateSource }]" type="button" @click.stop="selectUpdateSource(option)">{{ t(`optionLabels.updateSource.${option}`) }}</button>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Update Channel -->
      <div class="settings-row">
        <span class="settings-label">{{ t('settings.updateChannel') }}</span>
        <div class="custom-select">
          <button :class="['custom-select-trigger', { open: updateChannelOpen }]" type="button" @click.stop="toggleUpdateChannelMenu"><span>{{ currentUpdateChannelLabel }}</span></button>
          <Transition name="dropdown-fade">
            <div v-if="updateChannelOpen" :class="['custom-select-menu', { upward: updateChannelMenuUp }]">
              <button v-for="option in updateChannelOptionValues" :key="option" :class="['custom-select-option', { active: option === updateChannel }]" type="button" @click.stop="selectUpdateChannel(option)">{{ t(`optionLabels.updateChannel.${option}`) }}</button>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Download Source -->
      <div class="settings-row">
        <span class="settings-label settings-label-with-hint">
          {{ t('settings.downloadSource') }}
          <span class="settings-hint" tabindex="0"><span class="settings-hint-icon" aria-hidden="true">i</span><span class="settings-hint-tooltip">{{ t('settings.sourceHint') }}</span></span>
        </span>
        <div class="custom-select">
          <button :class="['custom-select-trigger', { open: downloadSourceOpen }]" type="button" @click.stop="toggleDownloadSourceMenu"><span>{{ currentDownloadSourceLabel }}</span></button>
          <Transition name="dropdown-fade">
            <div v-if="downloadSourceOpen" :class="['custom-select-menu', { upward: downloadSourceMenuUp }]">
              <button v-for="option in availableDownloadSourceOptions" :key="option" :class="['custom-select-option', { active: option === downloadSource }]" type="button" @click.stop="selectDownloadSource(option)">{{ t(`optionLabels.downloadSource.${option}`) }}</button>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Mirror CDK -->
      <div class="settings-row">
        <span class="settings-label settings-label-with-hint">
          {{ t('settings.mirrorChanSdk') }}
          <span class="settings-hint" tabindex="0"><span class="settings-hint-icon" aria-hidden="true">i</span><span class="settings-hint-tooltip">{{ t('settings.mirrorChanSdkHint') }}</span></span>
        </span>
        <button class="settings-edit-button" type="button" @click="emit('open-mirror-cdk-dialog')">{{ t('common.edit') }}</button>
      </div>
    </div>

    <Transition name="bottom-actions">
      <div v-if="settingsDirty" class="settings-actions">
        <button class="settings-action-button primary" type="button" @click="emit('save')">{{ t('common.save') }}</button>
        <button class="settings-action-button secondary" type="button" @click="emit('discard')">{{ t('common.discard') }}</button>
      </div>
    </Transition>
  </section>
</template>

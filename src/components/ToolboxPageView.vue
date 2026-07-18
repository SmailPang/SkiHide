<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import appIconUrl from '../assets/app-icon.svg';
import mirrorchyanUrl from '../assets/mirrorchyan.png';
import raincloudUrl from '../assets/raincloud.png';
import bilibiliIconUrl from '../assets/bilibili.ico';
import avatarUrl from '../assets/touxiang.jpg';
import type { MemoryStatusInfo } from '../types';

type ToolboxCardKey = 'memory' | 'cache' | 'feedback' | 'about';

const props = defineProps<{
  memoryAutoCleanup: boolean;
  memoryCleanupInterval: string;
  memoryCleanupUnit: 'seconds' | 'minutes' | 'hours';
  memoryCleanupRunning: boolean;
  memoryCleanupIntervalInvalid: boolean;
  cacheSelections: {
    systemCache: boolean;
    tempFiles: boolean;
    thumbnailCache: boolean;
    appCache: boolean;
    recycleBin: boolean;
  };
  cacheCleanupRunning: boolean;
  appVersion: string;
}>();

const emit = defineEmits<{
  'run-memory-cleanup': [];
  'toggle-memory-auto-cleanup': [];
  'memory-interval-input': [event: Event];
  'finalize-memory-interval': [];
  'select-memory-unit': [unit: 'seconds' | 'minutes' | 'hours'];
  'toggle-cache-selection': [key: 'systemCache' | 'tempFiles' | 'thumbnailCache' | 'appCache' | 'recycleBin'];
  'run-cache-cleanup': [];
  'open-update-dialog': [];
  'open-danger-dialog': [];
}>();

const { t } = useI18n();

// Internal state: active card
const activeToolboxCard = ref<ToolboxCardKey | null>(null);

// Internal state: custom scrollbar
const toolboxScrollRef = ref<HTMLElement | null>(null);
const toolboxScrollbarRef = ref<HTMLElement | null>(null);
const toolboxScrollbarActive = ref(false);
const toolboxThumbHeight = ref(0);
const toolboxThumbOffset = ref(0);
let toolboxDragStartY = 0;
let toolboxDragStartScrollTop = 0;
let toolboxDragging = false;

// Internal state: memory status
const memoryStatus = ref<MemoryStatusInfo | null>(null);
let memoryStatusTimer: number | null = null;

const memoryUsagePercent = computed(() => memoryStatus.value?.usage_percent ?? 0);
const memoryUsageFillStyle = computed(() => ({ width: `${memoryUsagePercent.value}%` }));
const memoryUsageState = computed(() => {
  if (memoryUsagePercent.value >= 90) return 'critical';
  if (memoryUsagePercent.value >= 70) return 'warning';
  return 'normal';
});
const memoryUsageText = computed(() => {
  if (!memoryStatus.value) return '-- / --';
  return `${formatBytes(memoryStatus.value.used_bytes)} / ${formatBytes(memoryStatus.value.total_bytes)}`;
});
const memoryUsageWarningVisible = computed(() => memoryUsagePercent.value >= 90);

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 GB';
  const gb = bytes / 1024 ** 3;
  if (gb >= 100) return `${gb.toFixed(0)} GB`;
  if (gb >= 10) return `${gb.toFixed(1)} GB`;
  return `${gb.toFixed(2)} GB`;
}

async function refreshMemoryStatus() {
  try {
    memoryStatus.value = await invoke<MemoryStatusInfo>('get_memory_status');
  } catch {
    memoryStatus.value = null;
  }
}

function clearMemoryStatusRefresh() {
  if (memoryStatusTimer !== null) {
    window.clearInterval(memoryStatusTimer);
    memoryStatusTimer = null;
  }
}

function scheduleMemoryStatusRefresh() {
  clearMemoryStatusRefresh();
  if (activeToolboxCard.value !== 'memory') return;
  void refreshMemoryStatus();
  memoryStatusTimer = window.setInterval(() => void refreshMemoryStatus(), 3000);
}

watch(activeToolboxCard, () => scheduleMemoryStatusRefresh());

// Refresh memory status immediately when cleanup finishes
watch(
  () => props.memoryCleanupRunning,
  (running, wasRunning) => {
    if (!running && wasRunning && activeToolboxCard.value === 'memory') {
      void refreshMemoryStatus();
    }
  },
);

function toggleToolboxCard(card: ToolboxCardKey) {
  activeToolboxCard.value = activeToolboxCard.value === card ? null : card;
  void nextTick(() => requestAnimationFrame(updateToolboxScrollbar));
}

// Scrollbar logic
function updateToolboxScrollbar() {
  const container = toolboxScrollRef.value;
  if (!container) { toolboxScrollbarActive.value = false; return; }
  const { scrollTop, scrollHeight, clientHeight } = container;
  const hasOverflow = scrollHeight > clientHeight + 1;
  toolboxScrollbarActive.value = hasOverflow;
  if (!hasOverflow) { toolboxThumbHeight.value = 0; toolboxThumbOffset.value = 0; return; }
  const thumbHeight = Math.max((clientHeight / scrollHeight) * clientHeight, 28);
  const maxThumbOffset = Math.max(clientHeight - thumbHeight, 0);
  const maxScrollTop = Math.max(scrollHeight - clientHeight, 1);
  toolboxThumbHeight.value = thumbHeight;
  toolboxThumbOffset.value = (scrollTop / maxScrollTop) * maxThumbOffset;
}

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

function handleGlobalMouseMove(event: MouseEvent) {
  if (!toolboxDragging || !toolboxScrollRef.value || !toolboxScrollbarRef.value) return;
  const trackHeight = toolboxScrollbarRef.value.clientHeight;
  const maxThumbOffset = Math.max(trackHeight - toolboxThumbHeight.value, 1);
  const maxScrollTop = Math.max(toolboxScrollRef.value.scrollHeight - toolboxScrollRef.value.clientHeight, 0);
  const deltaY = event.clientY - toolboxDragStartY;
  const nextScrollTop = toolboxDragStartScrollTop + (deltaY / maxThumbOffset) * maxScrollTop;
  toolboxScrollRef.value.scrollTop = Math.max(0, Math.min(nextScrollTop, maxScrollTop));
  updateToolboxScrollbar();
}

function handleGlobalMouseUp() { toolboxDragging = false; }

function openExternalUrl(url: string) { void invoke('open_external_url', { url }); }
function openGithubProfile() { openExternalUrl('https://github.com/SmailPang'); }
function openBilibiliProfile() { openExternalUrl('https://space.bilibili.com/674779529'); }

onMounted(() => {
  window.addEventListener('mousemove', handleGlobalMouseMove);
  window.addEventListener('mouseup', handleGlobalMouseUp);
  window.addEventListener('resize', updateToolboxScrollbar);
  requestAnimationFrame(updateToolboxScrollbar);
  scheduleMemoryStatusRefresh();
});

onBeforeUnmount(() => {
  window.removeEventListener('mousemove', handleGlobalMouseMove);
  window.removeEventListener('mouseup', handleGlobalMouseUp);
  window.removeEventListener('resize', updateToolboxScrollbar);
  clearMemoryStatusRefresh();
});
</script>

<template>
  <section class="page-card toolbox-page">
    <div class="toolbox-scroll-shell">
      <div ref="toolboxScrollRef" class="toolbox-actions" @scroll="handleToolboxScroll">

        <!-- Memory Card -->
        <div
          :class="['toolbox-action-card', { active: activeToolboxCard === 'memory' }]"
          :aria-expanded="activeToolboxCard === 'memory'"
          role="button"
          tabindex="0"
          @click="toggleToolboxCard('memory')"
        >
          <div class="toolbox-action-main">
            <span class="toolbox-action-title">{{ t('toolbox.memoryTitle') }}</span>
            <span class="toolbox-action-subtitle">{{ t('toolbox.memorySubtitle') }}</span>
          </div>
          <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
            <div v-if="activeToolboxCard === 'memory'" class="toolbox-action-detail">
              <div class="toolbox-memory-status" @click.stop>
                <div class="toolbox-memory-status-header">
                  <span class="toolbox-setting-label">{{ t('toolbox.memoryUsage') }}</span>
                  <span class="toolbox-memory-status-value">{{ memoryUsagePercent }}%</span>
                </div>
                <div :class="['toolbox-memory-bar', `is-${memoryUsageState}`]">
                  <div class="toolbox-memory-bar-fill" :style="memoryUsageFillStyle" />
                </div>
                <div class="toolbox-memory-status-meta">{{ memoryUsageText }}</div>
                <p v-if="memoryUsageWarningVisible" class="toolbox-memory-warning">{{ t('toolbox.memoryUsageHigh') }}</p>
              </div>
              <div class="toolbox-setting-row" @click.stop>
                <span class="toolbox-setting-label">{{ t('toolbox.memoryAutoCleanup') }}</span>
                <button
                  :class="['settings-switch', 'toolbox-switch', { active: memoryAutoCleanup }]"
                  type="button"
                  role="switch"
                  :aria-checked="memoryAutoCleanup"
                  @click.stop="emit('toggle-memory-auto-cleanup')"
                ><span class="settings-switch-thumb" /></button>
              </div>
              <div class="toolbox-setting-block" @click.stop>
                <span class="toolbox-setting-label">{{ t('toolbox.memoryInterval') }}</span>
                <div class="toolbox-interval-control">
                  <input
                    class="toolbox-interval-input"
                    type="text"
                    inputmode="numeric"
                    :value="memoryCleanupInterval"
                    :disabled="!memoryAutoCleanup"
                    @input="emit('memory-interval-input', $event)"
                    @blur="emit('finalize-memory-interval')"
                    @click.stop
                  />
                  <div class="toolbox-unit-group">
                    <button :class="['toolbox-unit-button', { active: memoryCleanupUnit === 'seconds' }]" type="button" :disabled="!memoryAutoCleanup" @click.stop="emit('select-memory-unit', 'seconds')">{{ t('toolbox.seconds') }}</button>
                    <button :class="['toolbox-unit-button', { active: memoryCleanupUnit === 'minutes' }]" type="button" :disabled="!memoryAutoCleanup" @click.stop="emit('select-memory-unit', 'minutes')">{{ t('toolbox.minutes') }}</button>
                    <button :class="['toolbox-unit-button', { active: memoryCleanupUnit === 'hours' }]" type="button" :disabled="!memoryAutoCleanup" @click.stop="emit('select-memory-unit', 'hours')">{{ t('toolbox.hours') }}</button>
                  </div>
                </div>
                <p v-if="memoryAutoCleanup && memoryCleanupIntervalInvalid" class="toolbox-field-error">{{ t('toolbox.intervalInvalid') }}</p>
              </div>
              <button class="toolbox-action-button" type="button" :disabled="memoryCleanupRunning" @click.stop="emit('run-memory-cleanup')">
                {{ memoryCleanupRunning ? t('toolbox.cleaning') : t('toolbox.runNow') }}
              </button>
            </div>
          </Transition>
        </div>

        <!-- Cache Card -->
        <div
          :class="['toolbox-action-card', { active: activeToolboxCard === 'cache' }]"
          :aria-expanded="activeToolboxCard === 'cache'"
          role="button"
          tabindex="0"
          @click="toggleToolboxCard('cache')"
        >
          <div class="toolbox-action-main">
            <span class="toolbox-action-title">{{ t('toolbox.cacheTitle') }}</span>
            <span class="toolbox-action-subtitle">{{ t('toolbox.cacheSubtitle') }}</span>
          </div>
          <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
            <div v-if="activeToolboxCard === 'cache'" class="toolbox-action-detail">
              <div class="toolbox-setting-block" @click.stop>
                <span class="toolbox-setting-label">{{ t('toolbox.cacheSelectLabel') }}</span>
                <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.systemCache" class="toolbox-check-input" type="checkbox" @change="emit('toggle-cache-selection', 'systemCache')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.systemCache') }}</span></label>
                <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.tempFiles" class="toolbox-check-input" type="checkbox" @change="emit('toggle-cache-selection', 'tempFiles')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.tempFiles') }}</span></label>
                <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.thumbnailCache" class="toolbox-check-input" type="checkbox" @change="emit('toggle-cache-selection', 'thumbnailCache')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.thumbnailCache') }}</span></label>
                <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.appCache" class="toolbox-check-input" type="checkbox" @change="emit('toggle-cache-selection', 'appCache')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.appCache') }}</span></label>
                <label class="toolbox-check-row" @click.stop><input :checked="cacheSelections.recycleBin" class="toolbox-check-input" type="checkbox" @change="emit('toggle-cache-selection', 'recycleBin')" /><span class="toolbox-check-box" /><span class="toolbox-check-label">{{ t('toolbox.recycleBin') }}</span></label>
                <button class="toolbox-action-button" type="button" :disabled="cacheCleanupRunning" @click.stop="emit('run-cache-cleanup')">
                  {{ cacheCleanupRunning ? t('toolbox.cleaning') : t('toolbox.runNow') }}
                </button>
              </div>
            </div>
          </Transition>
        </div>

        <!-- Feedback Card -->
        <div
          :class="['toolbox-action-card', { active: activeToolboxCard === 'feedback' }]"
          :aria-expanded="activeToolboxCard === 'feedback'"
          role="button"
          tabindex="0"
          @click="toggleToolboxCard('feedback')"
        >
          <span class="toolbox-action-title">{{ t('toolbox.feedbackTitle') }}</span>
          <span class="toolbox-action-subtitle">{{ t('toolbox.feedbackSubtitle') }}</span>
          <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
            <div v-if="activeToolboxCard === 'feedback'" class="toolbox-action-detail">
              <div class="toolbox-feedback-note" @click.stop>{{ t('toolbox.feedbackNotice') }}</div>
              <div class="toolbox-feedback-actions" @click.stop>
                <button class="toolbox-feedback-button" type="button" @click.stop="openExternalUrl('https://github.com/SmailPang/SkiHide/issues')">{{ t('toolbox.feedbackIssues') }}</button>
              </div>
            </div>
          </Transition>
        </div>

        <!-- About Card -->
        <div
          :class="['toolbox-action-card', { active: activeToolboxCard === 'about' }]"
          :aria-expanded="activeToolboxCard === 'about'"
          role="button"
          tabindex="0"
          @click="toggleToolboxCard('about')"
        >
          <span class="toolbox-action-title">{{ t('toolbox.aboutTitle') }}</span>
          <span class="toolbox-action-subtitle">{{ t('toolbox.aboutSubtitle') }}</span>
          <Transition name="toolbox-expand" @after-enter="handleToolboxExpandTransitionEnd" @after-leave="handleToolboxExpandTransitionEnd">
            <div v-if="activeToolboxCard === 'about'" class="toolbox-action-detail">
              <div class="toolbox-about-panel" @click.stop>
                <img class="toolbox-about-icon" :src="appIconUrl" alt="SkiHide icon" />
              </div>
              <div class="toolbox-about-profile" @click.stop>
                <img class="toolbox-about-avatar" :src="avatarUrl" alt="SmailPang avatar" />
                <div class="toolbox-about-meta">
                  <span class="toolbox-about-name">SmailPang</span>
                  <span class="toolbox-about-role">{{ t('toolbox.developer') }}</span>
                </div>
                <button class="toolbox-about-github" type="button" aria-label="Open Bilibili profile" @click.stop="openBilibiliProfile">
                  <img class="toolbox-about-bilibili-icon" :src="bilibiliIconUrl" alt="Bilibili" />
                </button>
                <button class="toolbox-about-github" type="button" aria-label="Open GitHub profile" @click.stop="openGithubProfile">
                  <svg class="toolbox-about-github-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2C6.48 2 2 6.58 2 12.23c0 4.52 2.87 8.35 6.84 9.7.5.1.68-.22.68-.49 0-.24-.01-1.05-.01-1.9-2.78.62-3.37-1.21-3.37-1.21-.46-1.19-1.11-1.51-1.11-1.51-.91-.64.07-.63.07-.63 1 .07 1.53 1.06 1.53 1.06.9 1.57 2.36 1.12 2.94.86.09-.67.35-1.12.63-1.38-2.22-.26-4.56-1.14-4.56-5.1 0-1.13.39-2.05 1.03-2.78-.1-.26-.45-1.32.1-2.76 0 0 .84-.27 2.75 1.06A9.3 9.3 0 0 1 12 6.84c.85 0 1.71.12 2.51.35 1.91-1.33 2.75-1.06 2.75-1.06.55 1.44.2 2.5.1 2.76.64.73 1.03 1.65 1.03 2.78 0 3.97-2.34 4.83-4.57 5.09.36.32.68.95.68 1.92 0 1.39-.01 2.5-.01 2.84 0 .27.18.59.69.49A10.25 10.25 0 0 0 22 12.23C22 6.58 17.52 2 12 2Z" /></svg>
                </button>
              </div>
              <div class="toolbox-about-links" @click.stop>
                <div class="toolbox-about-links-title">{{ t('toolbox.friendLinks') }}</div>
                <div class="toolbox-about-link-card">
                  <img class="toolbox-about-link-logo" :src="raincloudUrl" alt="RainCloud logo" />
                  <div class="toolbox-about-link-meta">
                    <span class="toolbox-about-link-name">{{ t('toolbox.raincloudName') }}</span>
                    <span class="toolbox-about-link-desc">{{ t('toolbox.raincloudDesc') }}</span>
                  </div>
                  <button class="toolbox-about-link-button" type="button" :aria-label="t('toolbox.openLink')" @click.stop="openExternalUrl('https://www.rainyun.com/Pang_?s=skihide-client')">
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
              <div class="toolbox-about-version" @click.stop>V{{ appVersion || '2.0.1' }}</div>
              <button class="toolbox-action-button toolbox-about-update" type="button" @click.stop="emit('open-update-dialog')">
                {{ t('toolbox.checkUpdates') }}
              </button>
            </div>
          </Transition>
        </div>

        <!-- Danger Button -->
        <button class="toolbox-action-card" type="button" @click="emit('open-danger-dialog')">
          <span class="toolbox-action-title">{{ t('toolbox.dangerTitle') }}</span>
          <span class="toolbox-action-subtitle">{{ t('toolbox.dangerSubtitle') }}</span>
        </button>
      </div>

      <Transition name="toolbox-scrollbar-fade">
        <div
          v-if="toolboxScrollbarActive"
          ref="toolboxScrollbarRef"
          class="toolbox-scrollbar"
          @mousedown="handleToolboxScrollbarTrackClick"
        >
          <div
            class="toolbox-scrollbar-thumb"
            :style="{ height: `${toolboxThumbHeight}px`, transform: `translateY(${toolboxThumbOffset}px)` }"
            @mousedown.stop="handleToolboxScrollbarDragStart"
          />
        </div>
      </Transition>
    </div>
  </section>
</template>

import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import type { MemoryAutoCleanupScheduleLog, MemoryCleanupReport, MemoryCleanupRequest } from '../types';
import type { NoticeType } from './useNotify';

type NotifyFn = (options: { title: string; content?: string; type: NoticeType }) => void;

export function useMemoryCleanup(notify: NotifyFn) {
  const { t } = useI18n();

  const memoryAutoCleanup = ref(false);
  const memoryCleanupInterval = ref('5');
  const memoryCleanupUnit = ref<'seconds' | 'minutes' | 'hours'>('minutes');
  const memoryCleanupRunning = ref(false);

  let memoryCleanupTimer: number | null = null;

  const memoryCleanupIntervalInvalid = computed(() => {
    if (!memoryAutoCleanup.value) return false;
    const parsed = Number(memoryCleanupInterval.value);
    return !Number.isFinite(parsed) || parsed <= 0;
  });

  function memoryCleanupIntervalMs(): number | null {
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

  function buildRequest(isAutoTrigger: boolean): MemoryCleanupRequest | null {
    if (!isAutoTrigger) return { auto_trigger: false };
    const parsed = Number(memoryCleanupInterval.value);
    if (!Number.isFinite(parsed) || parsed <= 0) return null;
    return { auto_trigger: true, interval_value: parsed, interval_unit: memoryCleanupUnit.value };
  }

  function logAutoCleanupSchedule() {
    const settings: MemoryAutoCleanupScheduleLog = {
      enabled: memoryAutoCleanup.value,
      interval_value: Number(memoryCleanupInterval.value) || 0,
      interval_unit: memoryCleanupUnit.value,
      scheduler_active: memoryCleanupIntervalMs() !== null,
    };
    void invoke('log_memory_auto_cleanup_schedule', { settings });
  }

  async function runMemoryCleanup(isAutoTrigger = false) {
    if (memoryCleanupRunning.value) return;
    if (isAutoTrigger && memoryCleanupIntervalInvalid.value) return;
    const request = buildRequest(isAutoTrigger);
    if (isAutoTrigger && request === null) return;
    memoryCleanupRunning.value = true;
    try {
      const report = await invoke<MemoryCleanupReport>('cleanup_memory', { request });
      if (!isAutoTrigger) {
        const reclaimedMb = (report.reclaimed_bytes / 1024 / 1024).toFixed(2);
        notify({
          title: t('toolbox.memoryTitle'),
          content: t('toolbox.memoryResult', { cleaned: report.cleaned, scanned: report.scanned, reclaimedMb }),
          type: 'true',
        });
      }
    } catch (error) {
      notify({ title: t('toolbox.memoryTitle'), content: String(error), type: 'false' });
    } finally {
      memoryCleanupRunning.value = false;
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

  function toggleMemoryAutoCleanup() {
    memoryAutoCleanup.value = !memoryAutoCleanup.value;
    if (memoryAutoCleanup.value && (memoryCleanupInterval.value === '' || Number(memoryCleanupInterval.value) <= 0)) {
      memoryCleanupInterval.value = '1';
    }
    scheduleMemoryCleanup();
    logAutoCleanupSchedule();
  }

  function handleMemoryIntervalInput(event: Event) {
    const target = event.target as HTMLInputElement;
    memoryCleanupInterval.value = target.value.replace(/[^\d]/g, '');
    scheduleMemoryCleanup();
  }

  function finalizeMemoryIntervalInput() {
    if (!memoryAutoCleanup.value) return;
    if (memoryCleanupInterval.value === '' || Number(memoryCleanupInterval.value) <= 0) {
      memoryCleanupInterval.value = '1';
    }
    scheduleMemoryCleanup();
    logAutoCleanupSchedule();
  }

  function selectMemoryCleanupUnit(unit: 'seconds' | 'minutes' | 'hours') {
    if (!memoryAutoCleanup.value) return;
    memoryCleanupUnit.value = unit;
    scheduleMemoryCleanup();
    logAutoCleanupSchedule();
  }

  return {
    memoryAutoCleanup,
    memoryCleanupInterval,
    memoryCleanupUnit,
    memoryCleanupRunning,
    memoryCleanupIntervalInvalid,
    runMemoryCleanup,
    scheduleMemoryCleanup,
    clearMemoryCleanupScheduler,
    toggleMemoryAutoCleanup,
    handleMemoryIntervalInput,
    finalizeMemoryIntervalInput,
    selectMemoryCleanupUnit,
  };
}

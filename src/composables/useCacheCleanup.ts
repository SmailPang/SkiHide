import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import type { CacheCleanupOptions, CacheCleanupReport } from '../types';
import type { NoticeType } from './useNotify';

type NotifyFn = (options: { title: string; content?: string; type: NoticeType }) => void;

export function useCacheCleanup(notify: NotifyFn) {
  const { t } = useI18n();

  const cacheSelections = ref({
    systemCache: false,
    tempFiles: false,
    thumbnailCache: false,
    appCache: false,
    recycleBin: false,
  });
  const cacheCleanupRunning = ref(false);

  function toggleCacheSelection(key: keyof typeof cacheSelections.value) {
    cacheSelections.value[key] = !cacheSelections.value[key];
  }

  async function runCacheCleanup() {
    if (cacheCleanupRunning.value) return;
    const options: CacheCleanupOptions = {
      system_cache: cacheSelections.value.systemCache,
      temp_files: cacheSelections.value.tempFiles,
      thumbnail_cache: cacheSelections.value.thumbnailCache,
      app_cache: cacheSelections.value.appCache,
      recycle_bin: cacheSelections.value.recycleBin,
    };
    if (!Object.values(options).some(Boolean)) {
      notify({ title: t('toolbox.cacheTitle'), content: t('toolbox.cacheSelectLabel'), type: 'warn' });
      return;
    }
    cacheCleanupRunning.value = true;
    try {
      const report = await invoke<CacheCleanupReport>('cleanup_cache', { options });
      const reclaimedMb = (report.reclaimed_bytes / 1024 / 1024).toFixed(2);
      notify({
        title: t('toolbox.cacheTitle'),
        content: t('toolbox.cacheResult', { cleaned: report.cleaned, reclaimedMb }),
        type: 'true',
      });
    } catch (error) {
      notify({ title: t('toolbox.cacheTitle'), content: String(error), type: 'false' });
    } finally {
      cacheCleanupRunning.value = false;
    }
  }

  return { cacheSelections, cacheCleanupRunning, toggleCacheSelection, runCacheCleanup };
}

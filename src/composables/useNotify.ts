import { ref } from 'vue';

export type NoticeType = 'true' | 'false' | 'warn' | 'info';
export type AppNotice = { id: number; type: NoticeType; title: string; content?: string };

export function useNotify() {
  const notices = ref<AppNotice[]>([]);
  let noticeIdSeed = 0;
  const noticeTimers = new Map<number, number>();

  function removeSaveNotice(id: number) {
    const timer = noticeTimers.get(id);
    if (timer !== undefined) {
      window.clearTimeout(timer);
      noticeTimers.delete(id);
    }
    notices.value = notices.value.filter((n) => n.id !== id);
  }

  function notify(options: { title: string; content?: string; type: NoticeType }) {
    const id = ++noticeIdSeed;
    notices.value = [...notices.value, { id, ...options }];
    const timer = window.setTimeout(() => removeSaveNotice(id), 3000);
    noticeTimers.set(id, timer);
  }

  function clearAll() {
    for (const timer of noticeTimers.values()) window.clearTimeout(timer);
    noticeTimers.clear();
  }

  return { notices, notify, removeSaveNotice, clearAll };
}

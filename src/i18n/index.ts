import { createI18n } from 'vue-i18n';
import { messages } from './messages';

export const i18n = createI18n({
  legacy: false,
  locale: 'zh_CN',
  fallbackLocale: 'en_US',
  messages,
});

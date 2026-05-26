import { createI18n } from 'vue-i18n';

// 自动扫描 locales/ 目录下所有 *.json 文件。
// 新增语言只需在该目录放入对应的 <locale>.json 文件，无需修改此文件。
const localeModules = import.meta.glob('./locales/*.json', { eager: true });

const messages: Record<string, Record<string, unknown>> = {};
for (const path in localeModules) {
  const matched = path.match(/\.\/locales\/(.+)\.json$/);
  if (matched) {
    const locale = matched[1];
    messages[locale] = (localeModules[path] as { default: Record<string, unknown> }).default;
  }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const i18n = (createI18n as any)({
  legacy: false,
  locale: 'zh_CN',
  fallbackLocale: 'en_US',
  messages,
});

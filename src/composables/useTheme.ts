import { computed, ref } from 'vue';

export function useTheme(getThemeTrigger?: () => HTMLElement | null) {
  const appShellRef = ref<HTMLElement | null>(null);
  const prefersDark = ref(false);
  const renderedTheme = ref<'light' | 'dark'>('light');
  const themeRipple = ref({ visible: false, target: 'light' as 'light' | 'dark', x: 0, y: 0, size: 0 });

  let colorSchemeQuery: MediaQueryList | null = null;
  let themeSwitchTimer: number | null = null;

  const activeTheme = computed(() => renderedTheme.value);

  function resolveTheme(mode: string): 'light' | 'dark' {
    if (mode === 'system') return prefersDark.value ? 'dark' : 'light';
    return mode === 'dark' ? 'dark' : 'light';
  }

  function animateThemeChange(nextTheme: 'light' | 'dark') {
    if (renderedTheme.value === nextTheme || !appShellRef.value) {
      renderedTheme.value = nextTheme;
      return;
    }
    const shellRect = appShellRef.value.getBoundingClientRect();
    const triggerEl = getThemeTrigger?.();
    const triggerRect = triggerEl?.getBoundingClientRect();
    const centerX = triggerRect ? triggerRect.left - shellRect.left + triggerRect.width / 2 : shellRect.width / 2;
    const centerY = triggerRect ? triggerRect.top - shellRect.top + triggerRect.height / 2 : shellRect.height / 2;
    const radius = Math.max(
      Math.hypot(centerX, centerY),
      Math.hypot(shellRect.width - centerX, centerY),
      Math.hypot(centerX, shellRect.height - centerY),
      Math.hypot(shellRect.width - centerX, shellRect.height - centerY),
    );
    if (themeSwitchTimer !== null) window.clearTimeout(themeSwitchTimer);
    themeRipple.value = { visible: false, target: nextTheme, x: centerX - radius, y: centerY - radius, size: radius * 2 };
    requestAnimationFrame(() => {
      themeRipple.value = { ...themeRipple.value, visible: true };
    });
    themeSwitchTimer = window.setTimeout(() => {
      renderedTheme.value = nextTheme;
      themeSwitchTimer = null;
    }, 170);
  }

  function handleThemeRippleEnd() {
    themeRipple.value = { ...themeRipple.value, visible: false };
  }

  function init(onSystemChange: () => void) {
    colorSchemeQuery = window.matchMedia('(prefers-color-scheme: dark)');
    prefersDark.value = colorSchemeQuery.matches;
    colorSchemeQuery.addEventListener('change', (event) => {
      prefersDark.value = event.matches;
      onSystemChange();
    });
  }

  function cleanup() {
    if (themeSwitchTimer !== null) window.clearTimeout(themeSwitchTimer);
    colorSchemeQuery?.removeEventListener('change', () => {});
  }

  return {
    appShellRef,
    prefersDark,
    renderedTheme,
    themeRipple,
    activeTheme,
    resolveTheme,
    animateThemeChange,
    handleThemeRippleEnd,
    init,
    cleanup,
  };
}

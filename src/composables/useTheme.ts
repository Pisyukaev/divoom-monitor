import { ref } from 'vue';

type Theme = 'light' | 'dark';

const THEME_STORAGE_KEY = 'divoom-monitor-theme';

const isDark = ref<boolean>(false);

let mediaQueryCleanup: (() => void) | null = null;

export function useTheme() {
  const setTheme = (theme: Theme) => {
    isDark.value = theme === 'dark';
    const html = document.documentElement;

    if (theme === 'dark') {
      html.classList.add('dark');
    } else {
      html.classList.remove('dark');
    }

    localStorage.setItem(THEME_STORAGE_KEY, theme);
  };

  const toggleTheme = () => {
    setTheme(isDark.value ? 'light' : 'dark');
  };

  const initTheme = () => {
    if (mediaQueryCleanup) {
      mediaQueryCleanup();
    }

    const savedTheme = localStorage.getItem(THEME_STORAGE_KEY) as Theme | null;

    if (savedTheme) {
      setTheme(savedTheme);
    } else {
      const prefersDark = window.matchMedia(
        '(prefers-color-scheme: dark)'
      ).matches;
      setTheme(prefersDark ? 'dark' : 'light');
    }

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem(THEME_STORAGE_KEY)) {
        setTheme(e.matches ? 'dark' : 'light');
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    mediaQueryCleanup = () => mediaQuery.removeEventListener('change', handleChange);
  };

  return {
    isDark,
    setTheme,
    toggleTheme,
    initTheme,
  };
}

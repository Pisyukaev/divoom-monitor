import { createI18n } from 'vue-i18n';
import ru from './locales/ru';
import en from './locales/en';

export type Locale = 'ru' | 'en';

const STORAGE_KEY = 'app_locale';

function getSavedLocale(): Locale {
  const saved = localStorage.getItem(STORAGE_KEY);
  if (saved === 'en' || saved === 'ru') return saved;
  return 'ru';
}

export function saveLocale(locale: Locale) {
  localStorage.setItem(STORAGE_KEY, locale);
}

const i18n = createI18n({
  legacy: false,
  locale: getSavedLocale(),
  fallbackLocale: 'ru',
  messages: { ru, en },
});

export default i18n;

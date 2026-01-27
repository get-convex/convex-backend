export default async function loadTranslations(locale: string) {
  try {
    const translations = await import(`../i18n/locales/${locale}.json`);
    return translations.default;
  } catch (error) {
    console.warn(`Missing gt-react translations for locale: ${locale}`, error);
    return {};
  }
}

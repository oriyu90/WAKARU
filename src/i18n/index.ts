import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./en.json";
import ja from "./ja.json";
import zhHans from "./zh-Hans.json";

export const SUPPORTED_LANGUAGES = ["en", "ja", "zh-Hans"] as const;
export type UiLanguage = (typeof SUPPORTED_LANGUAGES)[number];

const KEY = "wakaru.lang";

function detect(): UiLanguage {
  try {
    const stored = localStorage.getItem(KEY);
    if (stored && (SUPPORTED_LANGUAGES as readonly string[]).includes(stored)) {
      return stored as UiLanguage;
    }
  } catch {
    /* ignore */
  }
  const nav = navigator.language.toLowerCase();
  if (nav.startsWith("ja")) return "ja";
  if (nav.startsWith("zh")) return "zh-Hans";
  return "en";
}

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    ja: { translation: ja },
    "zh-Hans": { translation: zhHans },
  },
  lng: detect(),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export function setUiLanguage(lang: UiLanguage) {
  void i18n.changeLanguage(lang);
  document.documentElement.lang = lang === "zh-Hans" ? "zh-Hans" : lang;
  try {
    localStorage.setItem(KEY, lang);
  } catch {
    /* ignore */
  }
}

// Keep <html lang> in sync on first load (CJK glyph selection, docs/07 §2.4).
document.documentElement.lang =
  i18n.language === "zh-Hans" ? "zh-Hans" : i18n.language;

export default i18n;

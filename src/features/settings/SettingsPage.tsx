import { useTranslation } from "react-i18next";
import { useUiStore } from "../../stores/ui";
import type { Scale, ThemePref, ReadingFont } from "../../stores/ui";
import { Field } from "../../components/Field";
import { Select } from "../../components/Select";
import { Switch } from "../../components/Switch";
import { SUPPORTED_LANGUAGES, setUiLanguage } from "../../i18n";
import type { UiLanguage } from "../../i18n";
import i18n from "../../i18n";
import styles from "./SettingsPage.module.css";

const SCALES: Scale[] = [80, 90, 100, 110, 125, 150];
const LANG_LABEL: Record<UiLanguage, string> = {
  en: "English",
  ja: "日本語",
  "zh-Hans": "简体中文",
};

export function SettingsPage() {
  const { t } = useTranslation();
  const ui = useUiStore();

  return (
    <section className={styles.page}>
      <aside className={styles.rail} aria-label={t("settings.title")}>
        <span className={styles.railItemActive}>{t("settings.title")}</span>
      </aside>

      <div className={styles.panel}>
        <h1 className={styles.title}>{t("settings.title")}</h1>

        <div className={styles.group}>
          <Field label="Theme">
            {({ id }) => (
              <Select
                id={id}
                value={ui.theme}
                onChange={(e) => ui.setTheme(e.target.value as ThemePref)}
              >
                <option value="light">Light</option>
                <option value="dark">Dark</option>
                <option value="system">Follow system</option>
              </Select>
            )}
          </Field>

          <Field label="Display size">
            {({ id }) => (
              <Select
                id={id}
                value={String(ui.scale)}
                onChange={(e) => ui.setScale(Number(e.target.value) as Scale)}
              >
                {SCALES.map((s) => (
                  <option key={s} value={s}>
                    {s}%
                  </option>
                ))}
              </Select>
            )}
          </Field>

          <Field label="Reading font">
            {({ id }) => (
              <Select
                id={id}
                value={ui.readingFont}
                onChange={(e) =>
                  ui.setReadingFont(e.target.value as ReadingFont)
                }
              >
                <option value="serif">Serif (Spectral)</option>
                <option value="sans">Sans (Geist)</option>
              </Select>
            )}
          </Field>

          <div className={styles.switchRow}>
            <span className={styles.switchLabel}>Monochrome</span>
            <Switch
              label="Monochrome"
              checked={ui.monochrome}
              onChange={(e) => ui.setMonochrome(e.target.checked)}
            />
          </div>

          <div className={styles.switchRow}>
            <span className={styles.switchLabel}>{t("settings.illustrator")}</span>
            <Switch
              label={t("settings.illustrator")}
              checked={ui.illustratorEnabled}
              onChange={(e) => ui.setIllustratorEnabled(e.target.checked)}
            />
          </div>

          <Field label="Interface language">
            {({ id }) => (
              <Select
                id={id}
                value={i18n.language}
                onChange={(e) => setUiLanguage(e.target.value as UiLanguage)}
              >
                {SUPPORTED_LANGUAGES.map((l) => (
                  <option key={l} value={l}>
                    {LANG_LABEL[l]}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        </div>

        <p className={styles.note}>
          The full settings surface (AI, transcription, MCP, project management,
          about) arrives in Phase 9.
        </p>
      </div>
    </section>
  );
}

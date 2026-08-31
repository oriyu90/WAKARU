import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useUiStore } from "../../stores/ui";
import type { Scale, ThemePref, ReadingFont } from "../../stores/ui";
import { Field } from "../../components/Field";
import { Select } from "../../components/Select";
import { Switch } from "../../components/Switch";
import { SUPPORTED_LANGUAGES, setUiLanguage } from "../../i18n";
import type { UiLanguage } from "../../i18n";
import i18n from "../../i18n";
import { AiSettings } from "./AiSettings";
import { About } from "./About";
import { Shortcuts } from "./Shortcuts";
import { ProjectManagement } from "./ProjectManagement";
import { useAppSettings } from "./useAppSettings";
import styles from "./SettingsPage.module.css";

const SCALES: Scale[] = [80, 90, 100, 110, 125, 150];
const LANG_LABEL: Record<UiLanguage, string> = {
  en: "English",
  ja: "日本語",
  "zh-Hans": "简体中文",
};
const SECTIONS = [
  "general",
  "appearance",
  "ai",
  "transcription",
  "mcp",
  "language",
  "illustrator",
  "projects",
  "shortcuts",
  "about",
] as const;
type Section = (typeof SECTIONS)[number];

export function SettingsPage() {
  const { t } = useTranslation();
  const [section, setSection] = useState<Section>("general");
  const ui = useUiStore();
  const { settings, patch } = useAppSettings();

  return (
    <section className={styles.page}>
      <aside className={styles.rail} aria-label={t("settings.title")}>
        {SECTIONS.map((s) => (
          <button
            key={s}
            type="button"
            className={s === section ? styles.railItemActive : styles.railItem}
            aria-current={s === section}
            onClick={() => setSection(s)}
          >
            {t(`settings.section.${s}`)}
          </button>
        ))}
      </aside>

      <div className={styles.panel}>
        <h1 className={styles.title}>{t(`settings.section.${section}`)}</h1>

        {section === "general" && (
          <div className={styles.group}>
            <Field label={t("settings.startupView")}>
              {({ id }) => (
                <Select
                  id={id}
                  value={settings?.general.startupView ?? "home"}
                  onChange={(e) => patch({ general: { startupView: e.target.value } })}
                >
                  <option value="home">{t("settings.startupHome")}</option>
                  <option value="last_project">{t("settings.startupLast")}</option>
                </Select>
              )}
            </Field>
            <div className={styles.switchRow}>
              <span className={styles.switchLabel}>{t("settings.checkUpdates")}</span>
              <Switch
                label={t("settings.checkUpdates")}
                checked={settings?.general.checkUpdatesOnStart ?? true}
                onChange={(e) => patch({ general: { checkUpdatesOnStart: e.target.checked } })}
              />
            </div>
          </div>
        )}

        {section === "appearance" && (
          <div className={styles.group}>
            <Field label={t("settings.themeLabel")}>
              {({ id }) => (
                <Select id={id} value={ui.theme} onChange={(e) => ui.setTheme(e.target.value as ThemePref)}>
                  <option value="light">{t("settings.light")}</option>
                  <option value="dark">{t("settings.dark")}</option>
                  <option value="system">{t("settings.system")}</option>
                </Select>
              )}
            </Field>
            <Field label={t("settings.displaySize")}>
              {({ id }) => (
                <Select id={id} value={String(ui.scale)} onChange={(e) => ui.setScale(Number(e.target.value) as Scale)}>
                  {SCALES.map((s) => (
                    <option key={s} value={s}>
                      {s}%
                    </option>
                  ))}
                </Select>
              )}
            </Field>
            <Field label={t("settings.readingFont")}>
              {({ id }) => (
                <Select id={id} value={ui.readingFont} onChange={(e) => ui.setReadingFont(e.target.value as ReadingFont)}>
                  <option value="serif">Serif (Spectral)</option>
                  <option value="sans">Sans (Geist)</option>
                </Select>
              )}
            </Field>
            <div className={styles.switchRow}>
              <span className={styles.switchLabel}>{t("settings.monochrome")}</span>
              <Switch label={t("settings.monochrome")} checked={ui.monochrome} onChange={(e) => ui.setMonochrome(e.target.checked)} />
            </div>
          </div>
        )}

        {section === "ai" && <AiSettings />}

        {section === "transcription" && (
          <p className={styles.placeholder}>{t("settings.comingIn", { phase: 5 })}</p>
        )}
        {section === "mcp" && <p className={styles.placeholder}>{t("settings.comingIn", { phase: 7 })}</p>}
        {section === "projects" && <ProjectManagement />}

        {section === "language" && (
          <div className={styles.group}>
            <Field label={t("settings.uiLanguage")}>
              {({ id }) => (
                <Select id={id} value={i18n.language} onChange={(e) => setUiLanguage(e.target.value as UiLanguage)}>
                  {SUPPORTED_LANGUAGES.map((l) => (
                    <option key={l} value={l}>
                      {LANG_LABEL[l]}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
            <Field label={t("settings.aiResponseLang")} hint={t("settings.aiResponseHint")}>
              {({ id }) => (
                <Select
                  id={id}
                  value={settings?.language.aiResponse ?? "follow_ui"}
                  onChange={(e) => patch({ language: { aiResponse: e.target.value } })}
                >
                  <option value="follow_ui">{t("settings.followUi")}</option>
                  <option value="match_source">{t("settings.matchSource")}</option>
                  {SUPPORTED_LANGUAGES.map((l) => (
                    <option key={l} value={l}>
                      {LANG_LABEL[l]}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          </div>
        )}

        {section === "illustrator" && (
          <div className={styles.group}>
            <div className={styles.switchRow}>
              <span className={styles.switchLabel}>{t("settings.illustrator")}</span>
              <Switch
                label={t("settings.illustrator")}
                checked={ui.illustratorEnabled}
                onChange={(e) => {
                  ui.setIllustratorEnabled(e.target.checked);
                  patch({ illustrator: { enabled: e.target.checked } });
                }}
              />
            </div>
            <Field label={t("illustrator.detail")}>
              {({ id }) => (
                <Select
                  id={id}
                  value={settings?.illustrator.defaultLevel ?? "standard"}
                  onChange={(e) => patch({ illustrator: { defaultLevel: e.target.value } })}
                >
                  <option value="simple">{t("illustrator.simple")}</option>
                  <option value="standard">{t("illustrator.standard")}</option>
                  <option value="detailed">{t("illustrator.detailed")}</option>
                </Select>
              )}
            </Field>
            <Field label={t("illustrator.scope")}>
              {({ id }) => (
                <Select
                  id={id}
                  value={settings?.illustrator.defaultScope ?? "source"}
                  onChange={(e) => patch({ illustrator: { defaultScope: e.target.value } })}
                >
                  <option value="page">{t("illustrator.scopePage")}</option>
                  <option value="source">{t("illustrator.scopeSource")}</option>
                  <option value="project">{t("illustrator.scopeProject")}</option>
                </Select>
              )}
            </Field>
            <div className={styles.switchRow}>
              <span className={styles.switchLabel}>{t("settings.prefetch")}</span>
              <Switch
                label={t("settings.prefetch")}
                checked={settings?.illustrator.prefetchNext ?? true}
                onChange={(e) => patch({ illustrator: { prefetchNext: e.target.checked } })}
              />
            </div>
            <div className={styles.switchRow}>
              <span className={styles.switchLabel}>{t("settings.carryOver")}</span>
              <Switch
                label={t("settings.carryOver")}
                checked={settings?.illustrator.carryOverPreviousPage ?? true}
                onChange={(e) => patch({ illustrator: { carryOverPreviousPage: e.target.checked } })}
              />
            </div>
          </div>
        )}

        {section === "shortcuts" && <Shortcuts />}
        {section === "about" && <About />}
      </div>
    </section>
  );
}

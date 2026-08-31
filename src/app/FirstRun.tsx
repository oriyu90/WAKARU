import { useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { Dialog } from "../components/Dialog";
import { Button } from "../components/Button";
import { Field } from "../components/Field";
import { Select } from "../components/Select";
import { useUiStore } from "../stores/ui";
import type { ThemePref } from "../stores/ui";
import { SUPPORTED_LANGUAGES, setUiLanguage } from "../i18n";
import type { UiLanguage } from "../i18n";
import i18n from "../i18n";

const KEY = "wakaru.wizard.done";
const LANG_LABEL: Record<UiLanguage, string> = { en: "English", ja: "日本語", "zh-Hans": "简体中文" };

function seen() {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return true;
  }
}

/** One-screen welcome, shown once (FR / docs/07 §1). Everything here is also
 * reachable from Settings — skipping is fine. */
export function FirstRun() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const ui = useUiStore();
  const [open, setOpen] = useState(!seen());

  function done() {
    try {
      localStorage.setItem(KEY, "1");
    } catch {
      /* ignore */
    }
    setOpen(false);
  }

  return (
    <Dialog
      open={open}
      onClose={done}
      title={t("wizard.title")}
      footer={
        <>
          <Button
            variant="quiet"
            onClick={() => {
              done();
              navigate("/settings");
            }}
          >
            {t("wizard.setUpAi")}
          </Button>
          <Button variant="primary" onClick={done}>
            {t("wizard.start")}
          </Button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
        <p style={{ fontSize: "var(--fs-sm)", color: "var(--color-muted)" }}>{t("wizard.body")}</p>
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
        <Field label={t("settings.themeLabel")}>
          {({ id }) => (
            <Select id={id} value={ui.theme} onChange={(e) => ui.setTheme(e.target.value as ThemePref)}>
              <option value="light">{t("settings.light")}</option>
              <option value="dark">{t("settings.dark")}</option>
              <option value="system">{t("settings.system")}</option>
            </Select>
          )}
        </Field>
      </div>
    </Dialog>
  );
}

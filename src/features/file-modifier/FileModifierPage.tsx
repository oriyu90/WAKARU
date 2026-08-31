import { useTranslation } from "react-i18next";
import styles from "../_stub.module.css";

export function FileModifierPage() {
  const { t } = useTranslation();
  return (
    <section className={styles.page}>
      <h1 className={styles.title}>{t("fileModifier.title")}</h1>
      <p className={styles.subtitle}>{t("fileModifier.subtitle")}</p>
      <p className={styles.note}>Image → PDF and Text → MD/TXT arrive in Phase 8.</p>
    </section>
  );
}

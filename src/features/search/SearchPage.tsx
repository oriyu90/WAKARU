import { useTranslation } from "react-i18next";
import styles from "../_stub.module.css";

export function SearchPage() {
  const { t } = useTranslation();
  return (
    <section className={styles.page}>
      <h1 className={styles.title}>{t("search.title")}</h1>
      <p className={styles.subtitle}>{t("search.placeholder")}</p>
      <p className={styles.note}>Cross-project search arrives in Phase 3.</p>
    </section>
  );
}

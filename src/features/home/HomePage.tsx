import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button";
import { EmptyState } from "../../components/EmptyState";
import { PlusIcon } from "../../app/Icons";
import styles from "./HomePage.module.css";

export function HomePage() {
  const { t } = useTranslation();
  return (
    <section className={styles.page}>
      <header className={styles.head}>
        <h1 className={styles.title}>{t("home.title")}</h1>
        <Button variant="primary" icon={<PlusIcon size={16} />} disabled disabledReason="Phase 1">
          {t("home.newProject")}
        </Button>
      </header>

      <EmptyState
        title={t("home.empty.title")}
        body={t("home.empty.body")}
        actions={
          <>
            <Button variant="primary" disabled disabledReason="Phase 1">
              {t("home.empty.cta")}
            </Button>
            <Button variant="quiet" disabled disabledReason="Phase 10">
              {t("home.empty.import")}
            </Button>
          </>
        }
      />
    </section>
  );
}

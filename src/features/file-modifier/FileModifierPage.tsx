import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Tabs, TabPanel } from "../../components/Tabs";
import { ImagesToPdf } from "./ImagesToPdf";
import { TextToDoc } from "./TextToDoc";
import styles from "./FileModifier.module.css";

export function FileModifierPage() {
  const { t } = useTranslation();
  const [tab, setTab] = useState("pdf");

  return (
    <section className={styles.page}>
      <h1 className={styles.title}>{t("fileModifier.title")}</h1>
      <p className={styles.subtitle}>{t("fileModifier.subtitle")}</p>

      <Tabs
        label={t("fileModifier.title")}
        variant="segmented"
        value={tab}
        onChange={setTab}
        items={[
          { id: "pdf", label: t("fileModifier.imagesToPdf") },
          { id: "text", label: t("fileModifier.textToDoc") },
        ]}
      />

      <TabPanel id="pdf" active={tab === "pdf"}>
        <ImagesToPdf />
      </TabPanel>
      <TabPanel id="text" active={tab === "text"}>
        <TextToDoc />
      </TabPanel>
    </section>
  );
}

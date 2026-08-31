import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { appApi } from "../../ipc/app";
import { inTauri } from "../../ipc/client";
import styles from "./About.module.css";

const OFFICIAL_X = "https://x.com/InovateofRIZI";
const COMMUNITY_DISCORD = "https://discord.gg/x7KXhNTD8M";
const SITE = "https://studio-rizi.pages.dev/projects/wakaru/";

const THIRD_PARTY: [string, string][] = [
  ["Tauri", "MIT / Apache-2.0"],
  ["React · Vite", "MIT"],
  ["SQLite · sqlite-vec", "Public Domain · Apache-2.0"],
  ["pdf-extract · image · calamine", "MIT / Apache-2.0"],
  ["reqwest (rustls) · scraper", "MIT / Apache-2.0"],
  ["Geist · Spectral · JetBrains Mono", "SIL OFL 1.1"],
  ["KaTeX", "MIT"],
];

function openExternal(url: string) {
  if (!inTauri) {
    window.open(url, "_blank", "noreferrer");
    return;
  }
  void import("@tauri-apps/plugin-opener").then((m) => m.openUrl(url));
}

export function About() {
  const { t } = useTranslation();
  const info = useQuery({ queryKey: ["app-info"], queryFn: appApi.getInfo, enabled: inTauri });

  return (
    <div className={styles.wrap}>
      <dl className={styles.facts}>
        <dt>{t("about.version")}</dt>
        <dd className="u-mono-nums">
          {info.data?.version ?? "0.2.0"}
          {info.data?.buildDate && info.data.buildDate !== "dev" ? ` · ${info.data.buildDate}` : ""}
        </dd>
        <dt>{t("about.license")}</dt>
        <dd>MIT · © 2026 Yuki Orita</dd>
        <dt>{t("about.dataDir")}</dt>
        <dd className={styles.path}>{info.data?.dataDir ?? "—"}</dd>
      </dl>

      <div className={styles.links}>
        <Button size="sm" variant="quiet" onClick={() => openExternal(OFFICIAL_X)}>
          {t("about.official")}
        </Button>
        <Button size="sm" variant="quiet" onClick={() => openExternal(COMMUNITY_DISCORD)}>
          {t("about.community")}
        </Button>
        <Button size="sm" variant="quiet" onClick={() => openExternal(SITE)}>
          {t("about.site")}
        </Button>
      </div>

      <section>
        <h3 className={styles.h3}>{t("about.network")}</h3>
        <ul className={styles.list}>
          <li>{t("about.net1")}</li>
          <li>{t("about.net2")}</li>
          <li>{t("about.net3")}</li>
          <li>{t("about.net4")}</li>
        </ul>
      </section>

      <section>
        <h3 className={styles.h3}>{t("about.thirdParty")}</h3>
        <ul className={styles.list}>
          {THIRD_PARTY.map(([name, lic]) => (
            <li key={name}>
              {name} <span className={styles.lic}>— {lic}</span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

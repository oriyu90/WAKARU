import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { Skeleton } from "../../components/Skeleton";
import { ErrorState } from "../../components/ErrorState";
import { documentApi } from "../../ipc/viewer";
import type { SourceDetail } from "../../ipc/types.gen";
import styles from "./WebsitePreview.module.css";

const HTML_RE = /\.html?($|[?#])/i;

/** Preview for an imported / Studio-authored static site (issues 4 & 5).
 * Files are served through the project-scoped `wakaru-asset://` scheme into a
 * sandboxed <iframe>. The frame deliberately has an opaque origin: combining
 * `allow-scripts` and `allow-same-origin` would let authored HTML escape the
 * effective sandbox and is never necessary for a local preview. */
export function WebsitePreview({
  projectId,
  detail,
}: {
  projectId: string;
  detail: SourceDetail;
}) {
  const { t } = useTranslation();
  const manifest = useQuery({
    queryKey: ["website-manifest", projectId, detail.id],
    queryFn: () => documentApi.websiteManifest(projectId, detail.id),
  });

  const [current, setCurrent] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);

  const entry = manifest.data?.entry ?? null;
  const active = current ?? entry;

  const src = useQuery({
    queryKey: ["website-asset", projectId, detail.id, active],
    enabled: !!active,
    queryFn: () =>
      documentApi.assetUrl(projectId, detail.id, `sources/${detail.id}/${active}`),
  });

  const files = useMemo(() => manifest.data?.files ?? [], [manifest.data]);

  useEffect(() => {
    setCurrent(null);
    setReloadKey(0);
  }, [detail.id]);

  if (manifest.isLoading) {
    return (
      <div className={styles.wrap}>
        <div className={styles.loading}>
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} height="1rem" width={`${80 - i * 8}%`} />
          ))}
        </div>
      </div>
    );
  }
  if (manifest.isError) {
    return <ErrorState error={manifest.error} onRetry={() => manifest.refetch()} />;
  }

  return (
    <div className={styles.wrap}>
      <aside className={styles.tree} aria-label={t("viewer.websiteFiles")}>
        <p className={styles.treeHead}>{t("viewer.websiteFiles")}</p>
        <ul className={styles.list} ref={listRef}>
          {files.map((f) => {
            const isHtml = HTML_RE.test(f.path);
            return (
              <li key={f.path}>
                <button
                  type="button"
                  className={styles.fileBtn}
                  data-active={f.path === active}
                  data-html={isHtml}
                  aria-current={f.path === active}
                  onClick={() => isHtml && setCurrent(f.path)}
                  disabled={!isHtml}
                  title={f.path}
                >
                  <span className={styles.fileName}>{f.path}</span>
                  {f.path === entry ? (
                    <span className={styles.entryTag}>{t("viewer.websiteEntry")}</span>
                  ) : null}
                </button>
              </li>
            );
          })}
        </ul>
      </aside>

      <div className={styles.stage}>
        <div className={styles.bar}>
          <span className={styles.crumb} title={active ?? ""}>{active}</span>
          <span className={styles.spacer} />
          <Button
            size="sm"
            variant="quiet"
            onClick={() => setReloadKey((k) => k + 1)}
            disabled={!src.data}
          >
            {t("viewer.websiteReload")}
          </Button>
        </div>
        <div className={styles.frameHost}>
          {src.isLoading ? (
            <div className={styles.centered}>
              <Skeleton height="100%" width="100%" />
            </div>
          ) : src.isError ? (
            <ErrorState error={src.error} onRetry={() => src.refetch()} />
          ) : src.data ? (
            <iframe
              key={`${active}#${reloadKey}`}
              className={styles.frame}
              src={src.data}
              title={t("viewer.websitePreview")}
              sandbox="allow-scripts"
              referrerPolicy="no-referrer"
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}

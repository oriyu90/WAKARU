import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { EmptyState } from "../../components/EmptyState";
import { ErrorState } from "../../components/ErrorState";
import { Skeleton } from "../../components/Skeleton";
import { sourcesApi } from "../../ipc/sources";
import { inTauri } from "../../ipc/client";
import type { SourceStatusEvent } from "../../ipc/types.gen";
import { SourceRow } from "./SourceRow";
import styles from "./SourceList.module.css";

/** The "home" view of the Viewer: the project's sources. The add / link
 * actions live in the Viewer rail (they open documents into the same pane). */
export function SourceListPanel({
  projectId,
  onOpen,
}: {
  projectId: string;
  onOpen?: (sourceId: string) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [dragging, setDragging] = useState(false);

  const key = ["sources", projectId];
  const sources = useQuery({
    queryKey: key,
    queryFn: () => sourcesApi.list(projectId),
    enabled: inTauri,
  });

  // Refetch when the backend reports a status change for this project.
  useEffect(() => {
    if (!inTauri) return;
    let unlisten: (() => void) | undefined;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<SourceStatusEvent>("source://status", (e) => {
        if (e.payload.projectId === projectId) {
          void qc.invalidateQueries({ queryKey: key });
        }
      });
    })();
    return () => unlisten?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  const reanalyze = useMutation({
    mutationFn: (id: string) => sourcesApi.reanalyze(projectId, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: key }),
  });
  const del = useMutation({
    mutationFn: (id: string) => sourcesApi.delete(projectId, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: key }),
  });

  const list = sources.data ?? [];

  return (
    <div
      className={styles.panel}
      data-dragging={dragging}
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        // Tauri delivers real paths via its own drag-drop event; browser File
        // objects have no path we can pass to Rust.
      }}
    >
      <header className={styles.toolbar}>
        <span className={styles.count}>
          {t("project.sources.count", { count: list.length })}
        </span>
      </header>

      <div className={styles.body}>
        {sources.isLoading ? (
          <div className={styles.skel}>
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} height="2rem" />
            ))}
          </div>
        ) : sources.isError ? (
          <ErrorState error={sources.error} onRetry={() => sources.refetch()} />
        ) : list.length === 0 ? (
          <div className={styles.emptyWrap}>
            <EmptyState title={t("project.sources.title")} body={t("project.sources.empty")} />
          </div>
        ) : (
          <div className={styles.list} role="list">
            {list.map((s) => (
              <SourceRow
                key={s.id}
                source={s}
                onOpen={onOpen ? () => onOpen(s.id) : undefined}
                onReanalyze={() => reanalyze.mutate(s.id)}
                onDelete={() => del.mutate(s.id)}
              />
            ))}
          </div>
        )}
      </div>

      <p className={styles.dropHint} data-dragging={dragging}>
        {t("project.sources.drop")}
      </p>
    </div>
  );
}

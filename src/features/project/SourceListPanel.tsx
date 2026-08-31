import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { EmptyState } from "../../components/EmptyState";
import { ErrorState } from "../../components/ErrorState";
import { Skeleton } from "../../components/Skeleton";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { PlusIcon } from "../../app/Icons";
import { sourcesApi, pickSourceFiles } from "../../ipc/sources";
import { useToast } from "../../components/useToast";
import { IpcError, inTauri } from "../../ipc/client";
import type { SourceStatusEvent } from "../../ipc/types.gen";
import { SourceRow } from "./SourceRow";
import styles from "./SourceList.module.css";

export function SourceListPanel({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const [dragging, setDragging] = useState(false);
  const [urlDialog, setUrlDialog] = useState(false);
  const [url, setUrl] = useState("");

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

  const add = useMutation({
    mutationFn: (paths: string[]) => sourcesApi.addFiles(projectId, paths),
    onSuccess: () => qc.invalidateQueries({ queryKey: key }),
    onError: (err) => {
      if (err instanceof IpcError) {
        toast.push({ tone: "error", message: t([`errors.${err.code}`, "errors.internal"]) });
      } else {
        toast.push({ tone: "error", message: t("errors.internal") });
      }
    },
  });

  const addUrl = useMutation({
    mutationFn: (u: string) => sourcesApi.addUrl(projectId, u),
    onSuccess: () => {
      setUrl("");
      setUrlDialog(false);
      qc.invalidateQueries({ queryKey: key });
    },
    onError: (err) => {
      toast.push({
        tone: "error",
        message:
          err instanceof IpcError
            ? t([`errors.${err.code}`, "errors.internal"])
            : t("errors.internal"),
      });
    },
  });

  const reanalyze = useMutation({
    mutationFn: (id: string) => sourcesApi.reanalyze(projectId, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: key }),
  });
  const del = useMutation({
    mutationFn: (id: string) => sourcesApi.delete(projectId, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: key }),
  });

  async function onAddClick() {
    const paths = await pickSourceFiles();
    if (paths.length) add.mutate(paths);
  }

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
        // Tauri delivers real paths via its own drag-drop event (wired in the
        // follow-up); browser File objects have no path we can pass to Rust.
      }}
    >
      <header className={styles.toolbar}>
        <span className={styles.count}>
          {t("project.sources.count", { count: list.length })}
        </span>
        <span className={styles.toolbarActions}>
          <Button size="sm" variant="quiet" onClick={() => setUrlDialog(true)}>
            {t("project.sources.addLink")}
          </Button>
          <Button
            size="sm"
            variant="primary"
            icon={<PlusIcon size={14} />}
            loading={add.isPending}
            onClick={onAddClick}
          >
            {t("project.sources.add")}
          </Button>
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
          <EmptyState title={t("project.sources.title")} body={t("project.sources.empty")} />
        ) : (
          <div className={styles.list} role="list">
            {list.map((s) => (
              <SourceRow
                key={s.id}
                source={s}
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

      <Dialog
        open={urlDialog}
        onClose={() => setUrlDialog(false)}
        title={t("project.sources.addLink")}
        footer={
          <>
            <Button variant="quiet" onClick={() => setUrlDialog(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              loading={addUrl.isPending}
              disabled={!/^https?:\/\//i.test(url.trim())}
              onClick={() => addUrl.mutate(url.trim())}
            >
              {t("project.sources.add")}
            </Button>
          </>
        }
      >
        <Field label="URL" hint={t("project.sources.linkHint")}>
          {({ id }) => (
            <Input
              id={id}
              type="url"
              value={url}
              autoFocus
              placeholder="https://example.com/article"
              onChange={(e) => setUrl(e.target.value)}
            />
          )}
        </Field>
      </Dialog>
    </div>
  );
}

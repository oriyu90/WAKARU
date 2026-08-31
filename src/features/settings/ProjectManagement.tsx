import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { Checkbox } from "../../components/Checkbox";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { EmptyState } from "../../components/EmptyState";
import { projectsApi } from "../../ipc/projects";
import { exportApi, pickZip } from "../../ipc/exportImport";
import { pickSaveDir } from "../../ipc/fileModifier";
import { useToast } from "../../components/useToast";
import { inTauri } from "../../ipc/client";
import type { ProjectSummary } from "../../ipc/types.gen";
import styles from "./ProjectManagement.module.css";

export function ProjectManagement() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [confirmText, setConfirmText] = useState("");

  const list = useQuery({
    queryKey: ["projects", true],
    queryFn: () => projectsApi.list(true),
    enabled: inTauri,
  });
  const rows = list.data ?? [];
  const selectedRows = rows.filter((r) => selected.has(r.id));

  const invalidate = () => {
    void qc.invalidateQueries({ queryKey: ["projects"] });
    setSelected(new Set());
  };

  const doExport = useMutation({
    mutationFn: async () => {
      const dir = await pickSaveDir();
      if (!dir) throw new Error("cancelled");
      return exportApi.export({ ids: [...selected], destDir: dir, includeEmbeddings: false });
    },
    onSuccess: (r) => toast.push({ tone: "success", message: t("pm.exported", { count: r.files.length }) }),
    onError: (e) => {
      if (e instanceof Error && e.message === "cancelled") return;
      toast.push({ tone: "error", message: t("errors.internal") });
    },
  });

  const doImport = useMutation({
    mutationFn: async () => {
      const zip = await pickZip();
      if (!zip) throw new Error("cancelled");
      return exportApi.import(zip);
    },
    onSuccess: () => {
      invalidate();
      toast.push({ tone: "success", message: t("pm.imported") });
    },
    onError: (e) => {
      if (e instanceof Error && e.message === "cancelled") return;
      toast.push({ tone: "error", message: t("pm.importFailed") });
    },
  });

  const archive = useMutation({
    mutationFn: (archived: boolean) =>
      Promise.all([...selected].map((id) => projectsApi.setArchived(id, archived))),
    onSuccess: invalidate,
  });

  const del = useMutation({
    mutationFn: () =>
      Promise.all(
        selectedRows.map((p) => projectsApi.delete(p.id, selectedRows.length === 1 ? p.name : confirmText)),
      ),
    onSuccess: () => {
      setConfirmDelete(false);
      setConfirmText("");
      invalidate();
    },
    onError: () => toast.push({ tone: "error", message: t("errors.PROJECT_DELETE_NAME_MISMATCH") }),
  });

  const toggle = (id: string) =>
    setSelected((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });

  if (rows.length === 0) {
    return (
      <div className={styles.wrap}>
        <EmptyState title={t("home.empty.title")} body={t("home.empty.body")} />
        <Button variant="secondary" size="sm" loading={doImport.isPending} onClick={() => doImport.mutate()}>
          {t("pm.import")}
        </Button>
      </div>
    );
  }

  const allChecked = selected.size === rows.length;
  const anyArchived = selectedRows.some((r) => r.archived);

  return (
    <div className={styles.wrap}>
      <div className={styles.bar}>
        <Checkbox
          label={t("pm.selectAll")}
          checked={allChecked}
          onChange={(e) => setSelected(e.target.checked ? new Set(rows.map((r) => r.id)) : new Set())}
        />
        <Button size="sm" variant="secondary" loading={doImport.isPending} onClick={() => doImport.mutate()}>
          {t("pm.import")}
        </Button>
      </div>

      <ul className={styles.list}>
        {rows.map((p: ProjectSummary) => (
          <li key={p.id} className={styles.row}>
            <Checkbox label="" checked={selected.has(p.id)} onChange={() => toggle(p.id)} />
            <span className={styles.name}>{p.name}</span>
            <span className={`${styles.meta} u-mono-nums`}>
              {t("project.sources.count", { count: p.sourceCount })}
            </span>
            {p.archived ? <span className={styles.badge}>📦</span> : <span />}
          </li>
        ))}
      </ul>

      {selected.size > 0 ? (
        <div className={styles.actions}>
          <span className={styles.selCount}>{t("pm.selectedN", { count: selected.size })}</span>
          <Button size="sm" variant="secondary" loading={doExport.isPending} onClick={() => doExport.mutate()}>
            {t("pm.exportZip")}
          </Button>
          <Button size="sm" variant="quiet" loading={archive.isPending} onClick={() => archive.mutate(!anyArchived)}>
            {anyArchived ? t("pm.unarchive") : t("pm.archive")}
          </Button>
          <Button size="sm" variant="danger" onClick={() => setConfirmDelete(true)}>
            {t("pm.delete")}
          </Button>
        </div>
      ) : null}

      <p className={styles.estimateNote}>{t("pm.embeddingsNote")}</p>

      <Dialog
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        title={t("pm.deleteTitle")}
        footer={
          <>
            <Button variant="quiet" onClick={() => setConfirmDelete(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              loading={del.isPending}
              disabled={selectedRows.length > 1 && confirmText !== String(selectedRows.length)}
              onClick={() => del.mutate()}
            >
              {t("pm.delete")}
            </Button>
          </>
        }
      >
        {selectedRows.length === 1 ? (
          <p>{t("pm.deleteOne", { name: selectedRows[0]?.name })}</p>
        ) : (
          <Field label={t("pm.deleteManyLabel", { count: selectedRows.length })}>
            {({ id }) => (
              <Input id={id} value={confirmText} onChange={(e) => setConfirmText(e.target.value)} placeholder={String(selectedRows.length)} />
            )}
          </Field>
        )}
      </Dialog>
    </div>
  );
}

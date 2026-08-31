import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { useToast } from "../../components/useToast";
import { inTauri } from "../../ipc/client";
import { whisperApi } from "../../ipc/whisper";
import type { WhisperModel } from "../../ipc/types.gen";
import styles from "./WhisperSettings.module.css";

function mb(bytes: number): string {
  return `${Math.round(bytes / 1_048_576)} MB`;
}

export function WhisperSettings() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const [progress, setProgress] = useState<Record<string, { done: number; total: number }>>({});

  const models = useQuery({
    queryKey: ["whisper-models"],
    queryFn: whisperApi.list,
    enabled: inTauri,
  });
  const refresh = () => qc.invalidateQueries({ queryKey: ["whisper-models"] });

  useEffect(() => {
    if (!inTauri) return;
    let un: (() => void) | undefined;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      un = await listen<{ name: string; done: number; total: number }>(
        "whisper://download",
        (e) => setProgress((p) => ({ ...p, [e.payload.name]: { done: e.payload.done, total: e.payload.total } })),
      );
    })();
    return () => un?.();
  }, []);

  const download = useMutation({
    mutationFn: async (name: string) => {
      const check = await whisperApi.diskCheck(name);
      if (!check.ok) {
        throw new Error(
          t("whisper.diskShort", { needed: mb(check.neededBytes), free: mb(check.freeBytes) }),
        );
      }
      return whisperApi.download(name);
    },
    onSuccess: (_r, name) => {
      setProgress((p) => {
        const n = { ...p };
        delete n[name];
        return n;
      });
      void refresh();
    },
    onError: (e) => toast.push({ tone: "error", message: (e as Error).message }),
  });
  const cancel = useMutation({ mutationFn: (name: string) => whisperApi.cancelDownload(name) });
  const remove = useMutation({
    mutationFn: (name: string) => whisperApi.remove(name),
    onSuccess: refresh,
  });
  const select = useMutation({
    mutationFn: (name: string) => whisperApi.select(name),
    onSuccess: refresh,
  });

  const rows = models.data ?? [];

  return (
    <div className={styles.wrap}>
      <p className={styles.note}>{t("whisper.intro")}</p>
      <ul className={styles.list}>
        {rows.map((m: WhisperModel) => {
          const p = progress[m.name];
          const pct = p && p.total ? Math.round((p.done / p.total) * 100) : 0;
          return (
            <li key={m.name} className={styles.row} data-selected={m.selected}>
              <div className={styles.main}>
                <span className={styles.name}>{m.name}</span>
                <span className={styles.size}>~{mb(m.approxBytes)}</span>
                {m.selected ? <span className={styles.badge}>{t("whisper.selected")}</span> : null}
              </div>

              {download.isPending && download.variables === m.name ? (
                <div className={styles.dl}>
                  <span className={`${styles.pct} u-mono-nums`}>{pct}%</span>
                  <Button size="sm" variant="quiet" onClick={() => cancel.mutate(m.name)}>
                    {t("common.cancel")}
                  </Button>
                </div>
              ) : m.downloaded ? (
                <div className={styles.actions}>
                  {!m.selected ? (
                    <Button size="sm" variant="secondary" loading={select.isPending} onClick={() => select.mutate(m.name)}>
                      {t("whisper.use")}
                    </Button>
                  ) : null}
                  <Button size="sm" variant="danger" loading={remove.isPending} onClick={() => remove.mutate(m.name)}>
                    {t("common.delete")}
                  </Button>
                </div>
              ) : (
                <Button size="sm" variant="secondary" onClick={() => download.mutate(m.name)}>
                  {t("whisper.download")}
                </Button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

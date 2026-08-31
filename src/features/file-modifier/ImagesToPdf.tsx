import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { IconButton } from "../../components/IconButton";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { Select } from "../../components/Select";
import { EmptyState } from "../../components/EmptyState";
import { CloseIcon, PlusIcon, ChevronRightIcon } from "../../app/Icons";
import { fmApi, pickImages, pickSaveDir } from "../../ipc/fileModifier";
import { useToast } from "../../components/useToast";
import { IpcError, inTauri } from "../../ipc/client";
import styles from "./FileModifier.module.css";

export function ImagesToPdf() {
  const { t } = useTranslation();
  const toast = useToast();
  const [paths, setPaths] = useState<string[]>([]);
  const [pageSize, setPageSize] = useState("a4");
  const [orientation, setOrientation] = useState("auto");
  const [margin, setMargin] = useState("md");
  const [fit, setFit] = useState("contain");
  const [dir, setDir] = useState("");
  const [name, setName] = useState("scan.pdf");

  const previews = useQuery({
    queryKey: ["fm-previews", paths],
    enabled: inTauri && paths.length > 0,
    queryFn: async () => {
      const out: Record<string, string> = {};
      for (const p of paths) {
        try {
          out[p] = await fmApi.imagePreview(p);
        } catch {
          /* skip unreadable */
        }
      }
      return out;
    },
  });

  const make = useMutation({
    mutationFn: async () => {
      const destPath = `${dir}/${name.endsWith(".pdf") ? name : name + ".pdf"}`;
      if (await fmApi.pathExists(destPath)) {
        if (!window.confirm(t("fileModifier.overwrite"))) throw new Error("cancelled");
      }
      return fmApi.imagesToPdf({
        images: paths,
        pageSize,
        orientation,
        margin,
        fit,
        destPath,
      });
    },
    onSuccess: (r) => toast.push({ tone: "success", message: t("fileModifier.wrote", { path: r.path }) }),
    onError: (e) => {
      if (e instanceof Error && e.message === "cancelled") return;
      toast.push({
        tone: "error",
        message: e instanceof IpcError ? t([`errors.${e.code}`, "errors.internal"]) : t("errors.internal"),
      });
    },
  });

  const move = (i: number, d: -1 | 1) => {
    setPaths((p) => {
      const j = i + d;
      if (j < 0 || j >= p.length) return p;
      const next = [...p];
      const a = next[i]!;
      next[i] = next[j]!;
      next[j] = a;
      return next;
    });
  };

  return (
    <div className={styles.tool}>
      <div className={styles.toolBar}>
        <Button
          size="sm"
          variant="secondary"
          icon={<PlusIcon size={14} />}
          onClick={async () => {
            const picked = await pickImages();
            if (picked.length) setPaths((p) => [...p, ...picked]);
          }}
        >
          {t("fileModifier.addImages")}
        </Button>
      </div>

      {paths.length === 0 ? (
        <EmptyState title={t("fileModifier.imagesToPdf")} body={t("fileModifier.imagesEmpty")} />
      ) : (
        <ol className={styles.thumbs}>
          {paths.map((p, i) => (
            <li key={p + i} className={styles.thumb}>
              {previews.data?.[p] ? (
                <img src={previews.data[p]} alt="" />
              ) : (
                <span className={styles.thumbPlaceholder} />
              )}
              <span className={styles.thumbIdx}>{i + 1}</span>
              <div className={styles.thumbActions}>
                <IconButton label="↑" size="sm" onClick={() => move(i, -1)}>
                  <ChevronRightIcon size={12} style={{ transform: "rotate(-90deg)" }} />
                </IconButton>
                <IconButton label="↓" size="sm" onClick={() => move(i, 1)}>
                  <ChevronRightIcon size={12} style={{ transform: "rotate(90deg)" }} />
                </IconButton>
                <IconButton
                  label={t("common.close")}
                  size="sm"
                  onClick={() => setPaths((x) => x.filter((_, k) => k !== i))}
                >
                  <CloseIcon size={12} />
                </IconButton>
              </div>
            </li>
          ))}
        </ol>
      )}

      <div className={styles.opts}>
        <Field label={t("fileModifier.pageSize")}>
          {({ id }) => (
            <Select id={id} value={pageSize} onChange={(e) => setPageSize(e.target.value)}>
              <option value="a4">A4</option>
              <option value="a3">A3</option>
              <option value="letter">Letter</option>
              <option value="fit">{t("fileModifier.fitImage")}</option>
            </Select>
          )}
        </Field>
        <Field label={t("fileModifier.orientation")}>
          {({ id }) => (
            <Select id={id} value={orientation} onChange={(e) => setOrientation(e.target.value)}>
              <option value="auto">{t("fileModifier.auto")}</option>
              <option value="portrait">{t("fileModifier.portrait")}</option>
              <option value="landscape">{t("fileModifier.landscape")}</option>
            </Select>
          )}
        </Field>
        <Field label={t("fileModifier.margin")}>
          {({ id }) => (
            <Select id={id} value={margin} onChange={(e) => setMargin(e.target.value)}>
              <option value="none">{t("fileModifier.marginNone")}</option>
              <option value="sm">5&nbsp;mm</option>
              <option value="md">10&nbsp;mm</option>
              <option value="lg">20&nbsp;mm</option>
            </Select>
          )}
        </Field>
        <Field label={t("fileModifier.fit")}>
          {({ id }) => (
            <Select id={id} value={fit} onChange={(e) => setFit(e.target.value)}>
              <option value="contain">{t("fileModifier.contain")}</option>
              <option value="cover">{t("fileModifier.cover")}</option>
            </Select>
          )}
        </Field>
      </div>

      <div className={styles.output}>
        <Field label={t("fileModifier.outputDir")}>
          {({ id }) => (
            <div className={styles.dirRow}>
              <Input id={id} value={dir} readOnly placeholder="…" />
              <Button
                size="sm"
                variant="secondary"
                onClick={async () => {
                  const d = await pickSaveDir();
                  if (d) setDir(d);
                }}
              >
                {t("fileModifier.choose")}
              </Button>
            </div>
          )}
        </Field>
        <Field label={t("fileModifier.fileName")}>
          {({ id }) => <Input id={id} value={name} onChange={(e) => setName(e.target.value)} />}
        </Field>
      </div>

      <div className={styles.actions}>
        <Button
          variant="primary"
          loading={make.isPending}
          disabled={paths.length === 0 || !dir || !name.trim()}
          onClick={() => make.mutate()}
        >
          {t("fileModifier.createPdf")}
        </Button>
      </div>
    </div>
  );
}

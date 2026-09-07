import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { Textarea } from "../../components/Textarea";
import { Markdown } from "../../components/Markdown";
import { fmApi, pickSaveDir } from "../../ipc/fileModifier";
import { useToast } from "../../components/useToast";
import { useStream } from "../viewer/useStream";
import { inTauri, IpcError } from "../../ipc/client";
import styles from "./FileModifier.module.css";

export function TextToDoc() {
  const { t } = useTranslation();
  const toast = useToast();
  const [input, setInput] = useState("");
  const [format, setFormat] = useState<"md" | "txt">("md");
  const [streamId, setStreamId] = useState<string | null>(null);
  const [showDiff, setShowDiff] = useState(false);
  const [dir, setDir] = useState("");
  const [name, setName] = useState("notes");

  const stream = useStream(streamId);

  useEffect(() => {
    if (!stream.error) return;
    toast.push({
      tone: "error",
      message: t([`errors.${stream.errorCode ?? "internal"}`, "errors.internal"]),
    });
  }, [stream.error, stream.errorCode, t, toast]);

  const run = useMutation({
    mutationFn: () => fmApi.textToMarkdown(input),
    onSuccess: (id) => setStreamId(id),
    onError: (e) =>
      toast.push({
        tone: "error",
        message:
          e instanceof IpcError && e.code === "AI_NOT_CONFIGURED"
            ? t("errors.AI_NOT_CONFIGURED")
            : t("errors.internal"),
      }),
  });

  const save = useMutation({
    mutationFn: () => {
      const content = format === "md" && output ? output : input;
      return fmApi.saveText({ content, destPath: `${dir}/${name}`, format });
    },
    onSuccess: (r) => toast.push({ tone: "success", message: t("fileModifier.wrote", { path: r.path }) }),
    onError: () => toast.push({ tone: "error", message: t("errors.internal") }),
  });

  const output = stream.text;
  const suspiciousOutput = isSuspiciouslyShort(input, output);

  const diff = useMemo(() => (showDiff ? lineDiff(input, output) : []), [showDiff, input, output]);

  return (
    <div className={styles.tool}>
      <div className={styles.textGrid}>
        <div className={styles.textCol}>
          <span className={styles.colLabel}>{t("fileModifier.pasted")}</span>
          <Textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={t("fileModifier.pastePlaceholder")}
            rows={16}
          />
          <span className={styles.count}>{input.length.toLocaleString()}</span>
        </div>
        <div className={styles.textCol}>
          <span className={styles.colLabel}>
            {format === "md" ? t("fileModifier.organised") : "TXT"}
          </span>
          <div className={styles.outputBox}>
            {showDiff ? (
              <pre className={styles.diff}>
                {diff.map((d, i) => (
                  <span key={i} data-op={d.op}>
                    {d.op === "+" ? "+ " : d.op === "-" ? "- " : "  "}
                    {d.text}
                    {"\n"}
                  </span>
                ))}
              </pre>
            ) : format === "md" ? (
              output ? (
                <div className="reading">
                  <Markdown>{output}</Markdown>
                </div>
              ) : (
                <p className={styles.muted}>{stream.streaming ? `${t("common.loading")}…` : t("fileModifier.willOrganise")}</p>
              )
            ) : (
              <pre className={styles.plain}>{input}</pre>
            )}
          </div>
          <div className={styles.outputBar}>
            {format === "md" ? (
              <>
                <Button
                  size="sm"
                  variant="quiet"
                  loading={run.isPending || stream.streaming}
                  disabled={!input.trim() || !inTauri || !stream.ready}
                  onClick={() => {
                    setShowDiff(false);
                    run.mutate();
                  }}
                >
                  ↻ {t("fileModifier.organise")}
                </Button>
                <Button size="sm" variant="quiet" disabled={!output} onClick={() => setShowDiff((v) => !v)}>
                  {t("fileModifier.showDiff")}
                </Button>
              </>
            ) : null}
            {suspiciousOutput ? <span className={styles.warn}>{t("fileModifier.shrunkWarn")}</span> : null}
          </div>
        </div>
      </div>

      <div className={styles.formatRow} role="radiogroup" aria-label={t("fileModifier.saveAs")}>
        <label>
          <input type="radio" checked={format === "md"} onChange={() => setFormat("md")} disabled={!inTauri} />
          {t("fileModifier.organisedMd")}
        </label>
        <label>
          <input type="radio" checked={format === "txt"} onChange={() => setFormat("txt")} />
          {t("fileModifier.plainTxt")}
        </label>
        {!inTauri ? <span className={styles.muted}>{t("errors.offline")}</span> : null}
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
          loading={save.isPending}
          disabled={!dir || !name.trim() || (format === "md" ? !output || suspiciousOutput : !input)}
          onClick={() => save.mutate()}
        >
          {t("common.save")}
        </Button>
      </div>
    </div>
  );
}

export function isSuspiciouslyShort(input: string, output: string): boolean {
  return Boolean(input && output && output.length < input.length * 0.7);
}

type DiffLine = { op: " " | "+" | "-"; text: string };
function lineDiff(a: string, b: string): DiffLine[] {
  const A = a.split("\n");
  const B = b.split("\n");
  const setB = new Set(B.map((l) => l.trim()));
  const setA = new Set(A.map((l) => l.trim()));
  const out: DiffLine[] = [];
  for (const l of A) if (!setB.has(l.trim())) out.push({ op: "-", text: l });
  for (const l of B) out.push({ op: setA.has(l.trim()) ? " " : "+", text: l });
  return out;
}

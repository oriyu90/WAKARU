import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { ErrorState } from "../../components/ErrorState";
import { IconButton } from "../../components/IconButton";
import { ChevronRightIcon } from "../../app/Icons";
import { documentApi } from "../../ipc/viewer";
import type { DocumentPayload, SourceDetail } from "../../ipc/types.gen";
import styles from "./previews.module.css";

const MAX_INTERACTIVE_BYTES = 200 * 1024 * 1024;

function useAssetBuffer(detail: SourceDetail) {
  return useQuery({
    queryKey: ["asset-buffer", detail.id, detail.primaryAssetUrl, detail.bytes],
    enabled: !!detail.primaryAssetUrl && detail.bytes <= MAX_INTERACTIVE_BYTES,
    queryFn: async ({ signal }) => {
      const response = await fetch(detail.primaryAssetUrl!, { signal });
      if (!response.ok) throw new Error(`asset ${response.status}`);
      const declared = Number(response.headers.get("content-length") ?? 0);
      if (declared > MAX_INTERACTIVE_BYTES) throw new Error("asset-too-large");
      const bytes = await response.arrayBuffer();
      if (bytes.byteLength > MAX_INTERACTIVE_BYTES) throw new Error("asset-too-large");
      return bytes;
    },
  });
}

function sanitizeRenderedOffice(root: HTMLElement) {
  root.querySelectorAll("script,iframe,object,embed,form,meta,base").forEach((node) => node.remove());
  root.querySelectorAll<HTMLElement>("*").forEach((node) => {
    for (const attr of [...node.attributes]) {
      if (attr.name.toLowerCase().startsWith("on")) node.removeAttribute(attr.name);
    }
  });
  root.querySelectorAll<HTMLAnchorElement>("a[href]").forEach((anchor) => {
    const value = anchor.getAttribute("href")?.trim() ?? "";
    if (!/^(https?:|mailto:)/i.test(value)) anchor.removeAttribute("href");
    anchor.rel = "noreferrer noopener";
    anchor.target = "_blank";
  });
}

function FileLimit({ detail, fallback }: { detail: SourceDetail; fallback: React.ReactNode }) {
  const { t } = useTranslation();
  if (detail.bytes <= MAX_INTERACTIVE_BYTES) return fallback;
  return (
    <div className={styles.previewFallback} role="status">
      <p>{t("viewer.fileTooLarge")}</p>
      {fallback}
    </div>
  );
}

function PageControls({ page, total, onPage }: { page: number; total: number; onPage: (page: number) => void }) {
  const { t } = useTranslation();
  return (
    <div className={styles.fileControls}>
      <IconButton label={t("viewer.prevPage")} size="sm" disabled={page <= 1} onClick={() => onPage(page - 1)}>
        <ChevronRightIcon size={16} style={{ transform: "rotate(180deg)" }} />
      </IconButton>
      <span className={styles.pageLabel}>{page} / {Math.max(total, 1)}</span>
      <IconButton label={t("viewer.nextPage")} size="sm" disabled={page >= total} onClick={() => onPage(page + 1)}>
        <ChevronRightIcon size={16} />
      </IconButton>
    </div>
  );
}

export function PdfFilePreview({
  detail,
  initialPage,
  onPage,
  fallback,
}: {
  detail: SourceDetail;
  initialPage: number;
  onPage: (page: number, total: number) => void;
  fallback: React.ReactNode;
}) {
  const { t } = useTranslation();
  const asset = useAssetBuffer(detail);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const renderTask = useRef<{ cancel: () => void } | null>(null);
  const [pdf, setPdf] = useState<Awaited<ReturnType<typeof import("pdfjs-dist")["getDocument"]>["promise"]> | null>(null);
  const [page, setPage] = useState(Math.max(1, initialPage));
  const [zoom, setZoom] = useState(1);
  const [error, setError] = useState<unknown>(null);
  const onPageRef = useRef(onPage);
  onPageRef.current = onPage;

  useEffect(() => {
    if (!asset.data) return;
    let cancelled = false;
    let task: ReturnType<typeof import("pdfjs-dist")["getDocument"]> | undefined;
    void (async () => {
      const pdfjs = await import("pdfjs-dist");
      const worker = await import("pdfjs-dist/build/pdf.worker.mjs?url");
      pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
      task = pdfjs.getDocument({ data: asset.data.slice(0) });
      const loaded = await task.promise;
      if (!cancelled) {
        setPdf(loaded);
        setPage((current) => Math.min(current, loaded.numPages));
      }
    })().catch((reason) => !cancelled && setError(reason));
    return () => {
      cancelled = true;
      void task?.destroy();
    };
  }, [asset.data]);

  useEffect(() => {
    if (!pdf || !canvasRef.current) return;
    let cancelled = false;
    void (async () => {
      const pdfPage = await pdf.getPage(page);
      if (cancelled || !canvasRef.current) return;
      const viewport = pdfPage.getViewport({ scale: Math.min(window.devicePixelRatio || 1, 2) * zoom });
      const canvas = canvasRef.current;
      const context = canvas.getContext("2d", { alpha: false });
      if (!context) throw new Error("canvas-context-unavailable");
      canvas.width = Math.floor(viewport.width);
      canvas.height = Math.floor(viewport.height);
      canvas.style.width = `${viewport.width / Math.min(window.devicePixelRatio || 1, 2)}px`;
      canvas.style.height = `${viewport.height / Math.min(window.devicePixelRatio || 1, 2)}px`;
      renderTask.current?.cancel();
      const task = pdfPage.render({ canvas, canvasContext: context, viewport });
      renderTask.current = task;
      await task.promise;
      onPageRef.current(page, pdf.numPages);
    })().catch((reason) => {
      if (!cancelled && !(reason instanceof Error && reason.name === "RenderingCancelledException")) setError(reason);
    });
    return () => {
      cancelled = true;
      renderTask.current?.cancel();
    };
  }, [page, pdf, zoom]);

  if (detail.bytes > MAX_INTERACTIVE_BYTES) return <FileLimit detail={detail} fallback={fallback} />;
  if (asset.isError || error) return <div className={styles.previewFallback}><ErrorState error={asset.error ?? error} onRetry={() => { setError(null); void asset.refetch(); }} />{fallback}</div>;

  return (
    <div className={styles.filePreview}>
      <div className={styles.fileToolbar}>
        <PageControls page={page} total={pdf?.numPages ?? detail.pageCount ?? 1} onPage={setPage} />
        <span className={styles.toolbarSpacer} />
        <Button size="sm" variant="quiet" onClick={() => setZoom((value) => Math.max(.5, value - .25))}>−</Button>
        <span className={styles.pageLabel}>{Math.round(zoom * 100)}%</span>
        <Button size="sm" variant="quiet" onClick={() => setZoom((value) => Math.min(3, value + .25))}>+</Button>
      </div>
      <div className={styles.canvasViewport} aria-label={t("viewer.pdfPreview")}>
        {!pdf && !error ? <div className={styles.centered}>{t("states.analyzing")}…</div> : null}
        <canvas ref={canvasRef} className={styles.pdfCanvas} />
      </div>
    </div>
  );
}

export function DocxFilePreview({ detail, fallback }: { detail: SourceDetail; fallback: React.ReactNode }) {
  const { t } = useTranslation();
  const asset = useAssetBuffer(detail);
  const host = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<unknown>(null);

  useEffect(() => {
    if (!asset.data || !host.current) return;
    let cancelled = false;
    const container = host.current;
    container.replaceChildren();
    void import("docx-preview")
      .then(async ({ renderAsync }) => {
        if (cancelled) return;
        await renderAsync(asset.data.slice(0), container, undefined, {
          inWrapper: true,
          breakPages: true,
          renderAltChunks: false,
          ignoreLastRenderedPageBreak: false,
          useBase64URL: true,
        });
        if (!cancelled) sanitizeRenderedOffice(container);
      })
      .catch((reason) => !cancelled && setError(reason));
    return () => {
      cancelled = true;
      container.replaceChildren();
    };
  }, [asset.data]);

  if (detail.bytes > MAX_INTERACTIVE_BYTES) return <FileLimit detail={detail} fallback={fallback} />;
  if (asset.isError || error) return <div className={styles.previewFallback}><ErrorState error={asset.error ?? error} onRetry={() => { setError(null); void asset.refetch(); }} />{fallback}</div>;
  return <div className={styles.officeViewport} aria-label={t("viewer.docxPreview")} ref={host} />;
}

export function PptxFilePreview({
  detail,
  initialPage,
  onPage,
  fallback,
}: {
  detail: SourceDetail;
  initialPage: number;
  onPage: (page: number, total: number) => void;
  fallback: React.ReactNode;
}) {
  const { t } = useTranslation();
  const asset = useAssetBuffer(detail);
  const host = useRef<HTMLDivElement>(null);
  const presentation = useRef<Awaited<ReturnType<typeof import("pptx-viewer")["loadPresentation"]>> | null>(null);
  const [page, setPage] = useState(Math.max(1, initialPage));
  const [total, setTotal] = useState(detail.pageCount ?? 1);
  const [error, setError] = useState<unknown>(null);
  const onPageRef = useRef(onPage);
  onPageRef.current = onPage;

  useEffect(() => {
    if (!asset.data) return;
    let cancelled = false;
    void import("pptx-viewer").then(async ({ loadPresentation }) => {
      const loaded = await loadPresentation(asset.data.slice(0));
      if (cancelled) return loaded.cleanup();
      presentation.current = loaded;
      setTotal(loaded.slides.length);
      setPage((current) => Math.min(current, loaded.slides.length));
    }).catch((reason) => !cancelled && setError(reason));
    return () => {
      cancelled = true;
      presentation.current?.cleanup();
      presentation.current = null;
    };
  }, [asset.data]);

  useEffect(() => {
    if (!presentation.current || !host.current) return;
    host.current.replaceChildren();
    void import("pptx-viewer").then(({ renderSlideToElement }) => {
      if (!presentation.current || !host.current) return;
      renderSlideToElement(presentation.current, page - 1, host.current, { width: 1200 });
      sanitizeRenderedOffice(host.current);
      onPageRef.current(page, total);
    }).catch(setError);
  }, [page, total]);

  if (detail.bytes > MAX_INTERACTIVE_BYTES) return <FileLimit detail={detail} fallback={fallback} />;
  if (asset.isError || error) return <div className={styles.previewFallback}><ErrorState error={asset.error ?? error} onRetry={() => { setError(null); void asset.refetch(); }} />{fallback}</div>;
  return (
    <div className={styles.filePreview}>
      <div className={styles.fileToolbar}><PageControls page={page} total={total} onPage={setPage} /></div>
      <div className={styles.slideViewport} aria-label={t("viewer.pptxPreview")}><div ref={host} className={styles.slideHost} /></div>
    </div>
  );
}

export function WorkbookPreview({ projectId, detail }: { projectId: string; detail: SourceDetail }) {
  const { t } = useTranslation();
  const total = detail.pageCount ?? 1;
  const docs = useQuery({
    queryKey: ["workbook-documents", projectId, detail.id, total],
    queryFn: () => Promise.all(Array.from({ length: total }, (_, index) => documentApi.get(projectId, detail.id, index + 1))),
  });
  const [selected, setSelected] = useState(0);
  const tables = (docs.data ?? []).filter((doc: DocumentPayload) => doc.text.trimStart().startsWith("|"));
  if (docs.isError) return <ErrorState error={docs.error} onRetry={() => docs.refetch()} />;
  return (
    <div className={styles.filePreview}>
      <div className={styles.fileToolbar}>
        <label className={styles.sheetPicker}>{t("viewer.sheetRange")}
          <select value={selected} onChange={(event) => setSelected(Number(event.target.value))}>
            {tables.map((doc, index) => <option key={doc.ordinal} value={index}>{doc.title ?? `${index + 1}`}</option>)}
          </select>
        </label>
      </div>
      <div className={styles.body}>{docs.isLoading ? <div className={styles.centered}>{t("states.analyzing")}…</div> : <MarkdownTable text={tables[selected]?.text ?? ""} />}</div>
    </div>
  );
}

function MarkdownTable({ text }: { text: string }) {
  const rows = text.split("\n").filter(Boolean).filter((_, index) => index !== 1).map((line) => line.slice(1, -1).split("|").map((cell) => cell.trim().replace(/\\\|/g, "|")));
  return <table className={styles.sheet}><thead><tr>{(rows[0] ?? []).map((cell, index) => <th key={index}>{cell}</th>)}</tr></thead><tbody>{rows.slice(1).map((row, ri) => <tr key={ri}>{row.map((cell, ci) => <td key={ci}>{cell}</td>)}</tr>)}</tbody></table>;
}

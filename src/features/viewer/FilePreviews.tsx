import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { ErrorState } from "../../components/ErrorState";
import { IconButton } from "../../components/IconButton";
import { ChevronRightIcon } from "../../app/Icons";
import { documentApi } from "../../ipc/viewer";
import { ocrApi } from "../../ipc/ocr";
import { useToast } from "../../components/useToast";
import type { DocumentPayload, SourceDetail } from "../../ipc/types.gen";
import styles from "./previews.module.css";

// Office/PDF renderers duplicate the ArrayBuffer and then allocate decoded
// canvases/DOM. A 200 MiB source could therefore push a WebView past a safe
// working set. Keep ingestion and extracted-text reading available, but bound
// the interactive renderer to a release-safe ceiling.
const MAX_INTERACTIVE_BYTES = 96 * 1024 * 1024;
const MAX_SHARPEN_PIXELS = 4_000_000;

export function documentContrast(clarity: number) {
  return 1 + Math.min(Math.max(clarity, 0), 100) * 0.006;
}

/** Bounded unsharp-style convolution for raster PDF pages. */
export function sharpenRgba(data: Uint8ClampedArray, width: number, height: number, amount: number) {
  const strength = Math.min(Math.max(amount, 0), 100) * 0.005;
  if (!strength || width < 3 || height < 3 || width * height > MAX_SHARPEN_PIXELS) return false;
  const source = data.slice();
  for (let y = 1; y < height - 1; y += 1) {
    for (let x = 1; x < width - 1; x += 1) {
      const i = (y * width + x) * 4;
      for (let channel = 0; channel < 3; channel += 1) {
        const value = source[i + channel]! * (1 + 4 * strength)
          - strength * (source[i - 4 + channel]! + source[i + 4 + channel]!
            + source[i - width * 4 + channel]! + source[i + width * 4 + channel]!);
        data[i + channel] = Math.min(255, Math.max(0, Math.round(value)));
      }
    }
  }
  return true;
}

function DocumentAdjustments({
  inverted,
  clarity,
  onInverted,
  onClarity,
}: {
  inverted: boolean;
  clarity: number;
  onInverted: (value: boolean) => void;
  onClarity: (value: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className={styles.documentAdjustments}>
      <Button
        size="sm"
        variant={inverted ? "secondary" : "quiet"}
        aria-pressed={inverted}
        onClick={() => onInverted(!inverted)}
      >
        {t("viewer.invertDocument")}
      </Button>
      <label className={styles.clarityControl}>
        <span>{t("viewer.clarity")}</span>
        <input
          type="range"
          min="0"
          max="100"
          step="5"
          value={clarity}
          aria-label={t("viewer.clarity")}
          onChange={(event) => onClarity(Number(event.target.value))}
        />
        <output className={`${styles.pageLabel} u-mono-nums`}>{clarity}</output>
      </label>
    </div>
  );
}

function useAssetBuffer(detail: SourceDetail) {
  return useQuery({
    // ts-rs maps Rust u64 to bigint. TanStack's default key hash uses
    // JSON.stringify, which throws on bigint before any PDF/DOCX/PPTX fetch can
    // start, so keep the exact value as a serializable decimal string.
    queryKey: ["asset-buffer", detail.id, detail.primaryAssetUrl, detail.bytes.toString()],
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

/** True while the reader is typing somewhere — page/scroll keys must not steal
 * those keystrokes (e.g. the Illustrator textarea sits next to the viewer). */
function isTypingTarget() {
  const el = document.activeElement as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
}

/** Arrow / PageUp / PageDown page turning, shared by the PDF and PPTX renderers
 * (issue 2). Ignored while typing. */
function usePageKeys(onDelta: (delta: number) => void) {
  const cb = useRef(onDelta);
  cb.current = onDelta;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTypingTarget()) return;
      if (e.key === "ArrowRight" || e.key === "PageDown") {
        e.preventDefault();
        cb.current(1);
      } else if (e.key === "ArrowLeft" || e.key === "PageUp") {
        e.preventDefault();
        cb.current(-1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

/** Large translucent ‹ › affordances pinned to the sides of a paged viewport. */
function EdgeNav({ page, total, onPage }: { page: number; total: number; onPage: (page: number) => void }) {
  const { t } = useTranslation();
  return (
    <>
      <button
        type="button"
        className={styles.edgeNav}
        data-side="prev"
        aria-label={t("viewer.prevPage")}
        disabled={page <= 1}
        onClick={() => onPage(page - 1)}
      >
        <ChevronRightIcon size={22} style={{ transform: "rotate(180deg)" }} />
      </button>
      <button
        type="button"
        className={styles.edgeNav}
        data-side="next"
        aria-label={t("viewer.nextPage")}
        disabled={page >= Math.max(total, 1)}
        onClick={() => onPage(page + 1)}
      >
        <ChevronRightIcon size={22} />
      </button>
    </>
  );
}

/** Up / down scroll affordances for a long, non-paged document (issue 2). */
export function ScrollNav({ targetRef }: { targetRef: React.RefObject<HTMLElement | null> }) {
  const { t } = useTranslation();
  const by = (dir: number) => {
    const el = targetRef.current;
    if (el) el.scrollBy({ top: dir * el.clientHeight * 0.9, behavior: "smooth" });
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTypingTarget() || !targetRef.current) return;
      if (e.key === "PageDown") { e.preventDefault(); by(1); }
      else if (e.key === "PageUp") { e.preventDefault(); by(-1); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return (
    <div className={styles.scrollNav}>
      <button type="button" aria-label={t("viewer.scrollUp")} onClick={() => by(-1)}>
        <ChevronRightIcon size={18} style={{ transform: "rotate(-90deg)" }} />
      </button>
      <button type="button" aria-label={t("viewer.scrollDown")} onClick={() => by(1)}>
        <ChevronRightIcon size={18} style={{ transform: "rotate(90deg)" }} />
      </button>
    </div>
  );
}

export function PdfFilePreview({
  detail,
  projectId,
  initialPage,
  onPage,
  fallback,
}: {
  detail: SourceDetail;
  projectId: string;
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
  const [inverted, setInverted] = useState(false);
  const [clarity, setClarity] = useState(0);
  const [error, setError] = useState<unknown>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  usePageKeys((d) =>
    setPage((p) => Math.min(Math.max(p + d, 1), pdf?.numPages ?? detail.pageCount ?? p)),
  );
  const [availWidth, setAvailWidth] = useState(0);
  const onPageRef = useRef(onPage);
  onPageRef.current = onPage;

  // Track the usable width so the page can be drawn fit-to-width and re-fit as
  // the window resizes (P1). `zoom` stays a manual multiplier on top of the fit.
  useEffect(() => {
    const el = viewportRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const measure = () => {
      const cs = getComputedStyle(el);
      const pad = parseFloat(cs.paddingLeft) + parseFloat(cs.paddingRight);
      setAvailWidth(Math.max(0, el.clientWidth - pad));
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

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
      const raster = Math.min(window.devicePixelRatio || 1, 2);
      // Fit the page to the available width; `zoom` (the − / + control) multiplies
      // on top. Clamp so a small page doesn't balloon and a huge one still fits.
      const intrinsic = pdfPage.getViewport({ scale: 1 }).width;
      const fit = availWidth > 0 ? availWidth / intrinsic : 1;
      const cssScale = Math.min(Math.max(fit * zoom, 0.1), 4);
      const viewport = pdfPage.getViewport({ scale: raster * cssScale });
      const canvas = canvasRef.current;
      const context = canvas.getContext("2d", { alpha: false });
      if (!context) throw new Error("canvas-context-unavailable");
      canvas.width = Math.floor(viewport.width);
      canvas.height = Math.floor(viewport.height);
      canvas.style.width = `${viewport.width / raster}px`;
      canvas.style.height = `${viewport.height / raster}px`;
      renderTask.current?.cancel();
      const task = pdfPage.render({ canvas, canvasContext: context, viewport });
      renderTask.current = task;
      await task.promise;
      if (clarity > 0 && canvas.width * canvas.height <= MAX_SHARPEN_PIXELS) {
        const pixels = context.getImageData(0, 0, canvas.width, canvas.height);
        if (sharpenRgba(pixels.data, canvas.width, canvas.height, clarity)) {
          context.putImageData(pixels, 0, 0);
        }
      }
      onPageRef.current(page, pdf.numPages);
    })().catch((reason) => {
      if (!cancelled && !(reason instanceof Error && reason.name === "RenderingCancelledException")) setError(reason);
    });
    return () => {
      cancelled = true;
      renderTask.current?.cancel();
    };
  }, [page, pdf, zoom, availWidth, clarity]);

  const qc = useQueryClient();
  const toast = useToast();
  const [ocr, setOcr] = useState<{ done: number; total: number } | null>(null);

  async function runOcr() {
    if (!pdf || ocr) return;
    const total = pdf.numPages;
    setOcr({ done: 0, total });
    let allOk = true;
    try {
      const off = document.createElement("canvas");
      const ctx = off.getContext("2d", { alpha: false });
      for (let p = 1; p <= total; p += 1) {
        const pdfPage = await pdf.getPage(p);
        // Skip pages that already carry a real text layer.
        const tc = await pdfPage.getTextContent();
        const chars = tc.items.reduce(
          (n, it) => n + ("str" in it ? it.str.length : 0),
          0,
        );
        if (chars > 100) {
          setOcr({ done: p, total });
          continue;
        }
        // Cap the raster so a large page can't spike hundreds of MB, and yield
        // to the event loop between pages so the UI stays responsive.
        await new Promise((r) => setTimeout(r, 0));
        const base = pdfPage.getViewport({ scale: 1 });
        const ocrScale = Math.min(2, 2000 / Math.max(base.width, base.height));
        const vp = pdfPage.getViewport({ scale: Math.max(ocrScale, 1) });
        off.width = Math.floor(vp.width);
        off.height = Math.floor(vp.height);
        if (!ctx) throw new Error("canvas-context-unavailable");
        await pdfPage.render({ canvas: off, canvasContext: ctx, viewport: vp }).promise;
        const b64 = off.toDataURL("image/png").split(",")[1] ?? "";
        try {
          await ocrApi.page({ projectId, sourceId: detail.id, page: p, total, pngBase64: b64 });
        } catch {
          allOk = false;
        }
        setOcr({ done: p, total });
      }
      await ocrApi.finalize({ projectId, sourceId: detail.id, allOk });
      await qc.invalidateQueries({ queryKey: ["source-detail", projectId, detail.id] });
      toast.push({ tone: allOk ? "success" : "error", message: t("viewer.ocrDone") });
    } catch {
      toast.push({ tone: "error", message: t("errors.ocr.unavailable") });
    } finally {
      setOcr(null);
    }
  }

  if (detail.bytes > MAX_INTERACTIVE_BYTES) return <FileLimit detail={detail} fallback={fallback} />;
  if (asset.isError || error) return <div className={styles.previewFallback}><ErrorState error={asset.error ?? error} onRetry={() => { setError(null); void asset.refetch(); }} />{fallback}</div>;

  return (
    <div className={styles.filePreview}>
      <div className={styles.fileToolbar}>
        <PageControls page={page} total={pdf?.numPages ?? detail.pageCount ?? 1} onPage={setPage} />
        <DocumentAdjustments
          inverted={inverted}
          clarity={clarity}
          onInverted={setInverted}
          onClarity={setClarity}
        />
        <span className={styles.toolbarSpacer} />
        {ocr ? (
          <span className={styles.pageLabel}>{t("viewer.ocrProgress", { done: ocr.done, total: ocr.total })}</span>
        ) : detail.ocrStatus === "pending" || detail.ocrStatus === "partial" ? (
          <Button size="sm" variant="quiet" onClick={() => void runOcr()} disabled={!pdf}>
            {t("viewer.ocrRun")}
          </Button>
        ) : null}
        <Button size="sm" variant="quiet" onClick={() => setZoom((value) => Math.max(.5, value - .25))}>−</Button>
        <span className={styles.pageLabel}>{Math.round(zoom * 100)}%</span>
        <Button size="sm" variant="quiet" onClick={() => setZoom((value) => Math.min(3, value + .25))}>+</Button>
      </div>
      <div ref={viewportRef} className={styles.canvasViewport} aria-label={t("viewer.pdfPreview")}>
        {!pdf && !error ? <div className={styles.centered}>{t("states.analyzing")}…</div> : null}
        <canvas
          ref={canvasRef}
          className={styles.pdfCanvas}
          data-inverted={inverted}
          style={{ filter: `${inverted ? "invert(1) " : ""}contrast(${documentContrast(clarity)})` }}
        />
        {pdf && pdf.numPages > 1 ? (
          <EdgeNav page={page} total={pdf.numPages} onPage={setPage} />
        ) : null}
      </div>
    </div>
  );
}

export function DocxFilePreview({ detail, fallback }: { detail: SourceDetail; fallback: React.ReactNode }) {
  const { t } = useTranslation();
  const asset = useAssetBuffer(detail);
  const host = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<unknown>(null);
  const [inverted, setInverted] = useState(false);
  const [clarity, setClarity] = useState(0);

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
  return (
    <div className={styles.filePreview}>
      <div className={styles.fileToolbar}>
        <DocumentAdjustments inverted={inverted} clarity={clarity} onInverted={setInverted} onClarity={setClarity} />
      </div>
      <div
        className={styles.officeViewport}
        aria-label={t("viewer.docxPreview")}
        ref={host}
        data-inverted={inverted}
        style={{ "--document-contrast": documentContrast(clarity) } as React.CSSProperties}
      />
      <ScrollNav targetRef={host} />
    </div>
  );
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
  const [presentation, setPresentation] = useState<Awaited<ReturnType<typeof import("pptx-viewer")["loadPresentation"]>> | null>(null);
  const [page, setPage] = useState(Math.max(1, initialPage));
  const [total, setTotal] = useState(detail.pageCount ?? 1);
  const [error, setError] = useState<unknown>(null);
  const [inverted, setInverted] = useState(false);
  const [clarity, setClarity] = useState(0);
  const onPageRef = useRef(onPage);
  onPageRef.current = onPage;
  usePageKeys((d) => setPage((p) => Math.min(Math.max(p + d, 1), total)));

  useEffect(() => {
    if (!asset.data) return;
    let cancelled = false;
    let loadedPresentation: Awaited<ReturnType<typeof import("pptx-viewer")["loadPresentation"]>> | null = null;
    setError(null);
    setPresentation(null);
    void import("pptx-viewer").then(async ({ loadPresentation }) => {
      const loaded = await loadPresentation(asset.data.slice(0));
      loadedPresentation = loaded;
      if (cancelled) return loaded.cleanup();
      if (loaded.slides.length === 0) throw new Error("presentation-has-no-slides");
      setPresentation(loaded);
      setTotal(loaded.slides.length);
      setPage((current) => Math.max(1, Math.min(current, loaded.slides.length)));
    }).catch((reason) => !cancelled && setError(reason));
    return () => {
      cancelled = true;
      loadedPresentation?.cleanup();
    };
  }, [asset.data]);

  useEffect(() => {
    if (!presentation || !host.current) return;
    host.current.replaceChildren();
    void import("pptx-viewer").then(({ renderSlideToElement }) => {
      if (!host.current) return;
      renderSlideToElement(presentation, page - 1, host.current, { width: 1200 });
      sanitizeRenderedOffice(host.current);
      onPageRef.current(page, total);
    }).catch(setError);
  }, [page, presentation, total]);

  if (detail.bytes > MAX_INTERACTIVE_BYTES) return <FileLimit detail={detail} fallback={fallback} />;
  if (asset.isError || error) return <div className={styles.previewFallback}><ErrorState error={asset.error ?? error} onRetry={() => { setError(null); void asset.refetch(); }} />{fallback}</div>;
  return (
    <div className={styles.filePreview}>
      <div className={styles.fileToolbar}>
        <PageControls page={page} total={total} onPage={setPage} />
        <DocumentAdjustments inverted={inverted} clarity={clarity} onInverted={setInverted} onClarity={setClarity} />
        <span className={styles.toolbarSpacer} />
      </div>
      <div className={styles.slideViewport} aria-label={t("viewer.pptxPreview")}>
        <div
          ref={host}
          className={styles.slideHost}
          data-inverted={inverted}
          style={{ filter: `${inverted ? "invert(1) " : ""}contrast(${documentContrast(clarity)})` }}
        />
        {total > 1 ? <EdgeNav page={page} total={total} onPage={setPage} /> : null}
      </div>
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

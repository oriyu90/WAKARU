import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { ErrorState } from "../../components/ErrorState";
import { IconButton } from "../../components/IconButton";
import { ChevronRightIcon } from "../../app/Icons";
import { ZoomControls } from "./ZoomControls";
import { documentApi } from "../../ipc/viewer";
import { ocrApi } from "../../ipc/ocr";
import { useToast } from "../../components/useToast";
import { useUiStore } from "../../stores/ui";
import type { DocumentPayload, SourceDetail } from "../../ipc/types.gen";
import styles from "./previews.module.css";

// Office/PDF renderers duplicate the ArrayBuffer and then allocate decoded
// canvases/DOM. A 200 MiB source could therefore push a WebView past a safe
// working set. Keep ingestion and extracted-text reading available, but bound
// the interactive renderer to a release-safe ceiling.
const MAX_INTERACTIVE_BYTES = 96 * 1024 * 1024;
const MAX_SHARPEN_PIXELS = 4_000_000;
const MAX_PDF_CANVAS_PIXELS = 16_000_000;
const MAX_PDF_CANVAS_SIDE = 8_192;
const MAX_PDF_DISPLAY_SIDE = 16_384;

export function boundedPdfRaster(width: number, height: number, preferred: number) {
  if (![width, height, preferred].every(Number.isFinite) || width <= 0 || height <= 0) return 1;
  return Math.min(
    preferred,
    Math.sqrt(MAX_PDF_CANVAS_PIXELS / Math.max(1, width * height)),
    MAX_PDF_CANVAS_SIDE / Math.max(1, width, height),
  );
}

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

/** Document rendering adjustments live in the project pane bar now; the
 * previews below only consume the shared `ui` store values. */
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
  // Large Office files keep a staged text view; complex layout, embedded
  // media, macros and change history are not reproduced — stated, never
  // silently blank (plan §3.2).
  const office = detail.kind === "doc" || detail.kind === "slides" || detail.kind === "sheet";
  return (
    <div className={styles.previewFallback} role="status">
      <p>{t("viewer.fileTooLarge")}</p>
      {office ? <p>{t("viewer.largeOfficeNotice")}</p> : null}
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

/** Pinch (trackpad, `ctrlKey`) and Shift+wheel zoom for document viewports.
 * Plain wheel scrolling is untouched. The listener is non-passive so the
 * gesture never scrolls while zooming. */
export function usePinchZoom(
  targetRef: React.RefObject<HTMLElement | null>,
  zoom: number,
  onZoom: (zoom: number) => void,
  min: number,
  max: number,
) {
  const live = useRef({ zoom, onZoom, min, max });
  live.current = { zoom, onZoom, min, max };
  useEffect(() => {
    const el = targetRef.current;
    if (!el) return;
    function onWheel(e: WheelEvent) {
      if (!e.ctrlKey && !e.shiftKey) return;
      e.preventDefault();
      const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
      if (!Number.isFinite(delta) || delta === 0) return;
      const { zoom: z, onZoom: set, min: mn, max: mx } = live.current;
      const next = Math.min(mx, Math.max(mn, +(z * Math.exp(-delta * 0.01)).toFixed(3)));
      if (next !== z) set(next);
    }
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [targetRef]);
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
  // Fit shows the whole page (portrait pages are never cut at the bottom);
  // fill covers the viewport. The manual zoom multiplies on top of either.
  const [mode, setMode] = useState<"fit" | "fill">("fit");
  const inverted = useUiStore((s) => s.docInverted);
  const clarity = useUiStore((s) => s.docClarity);
  const [error, setError] = useState<unknown>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  // External jumps (citation, sticky-note page list) arrive as a new
  // initialPage; internal paging leaves the prop value unchanged, so this
  // only fires for real external moves.
  const initialRef = useRef(initialPage);
  useEffect(() => {
    if (initialPage !== initialRef.current) {
      initialRef.current = initialPage;
      setPage(Math.max(1, initialPage));
    }
  }, [initialPage]);
  usePageKeys((d) =>
    setPage((p) => Math.min(Math.max(p + d, 1), pdf?.numPages ?? detail.pageCount ?? p)),
  );
  const [availWidth, setAvailWidth] = useState(0);
  const [availHeight, setAvailHeight] = useState(0);
  const onPageRef = useRef(onPage);
  onPageRef.current = onPage;
  usePinchZoom(viewportRef, zoom, setZoom, 0.25, 4);

  // Track the usable area so the page re-fits as the window resizes (P1).
  // `zoom` stays a manual multiplier on top of the fit.
  useEffect(() => {
    const el = viewportRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const measure = () => {
      const cs = getComputedStyle(el);
      const padX = parseFloat(cs.paddingLeft) + parseFloat(cs.paddingRight);
      const padY = parseFloat(cs.paddingTop) + parseFloat(cs.paddingBottom);
      setAvailWidth(Math.max(0, el.clientWidth - padX));
      setAvailHeight(Math.max(0, el.clientHeight - padY));
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    let cancelled = false;
    let task: ReturnType<typeof import("pdfjs-dist")["getDocument"]> | undefined;
    void (async () => {
      const pdfjs = await import("pdfjs-dist");
      const worker = await import("pdfjs-dist/build/pdf.worker.mjs?url");
      pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
      // Ranged URL loading first (plan §3.2): pages stream in 256 KiB
      // chunks instead of one arrayBuffer copy. The custom protocol may not
      // forward Range on every WebView, so any failure falls back to the
      // bounded buffer path below — and beyond 96 MiB to extracted text.
      // Past 96 MiB a blind URL handoff could pull the whole file when the
      // server answers 200 instead of 206, so preflight one byte first.
      const url = detail.primaryAssetUrl ?? undefined;
      if (url && detail.bytes > 8 * 1024 * 1024) {
        let ranged = detail.bytes <= MAX_INTERACTIVE_BYTES;
        if (!ranged) {
          try {
            const probe = await fetch(url, { headers: { Range: "bytes=0-0" } });
            ranged = probe.status === 206;
            await probe.arrayBuffer().catch(() => undefined);
          } catch {
            ranged = false;
          }
          if (cancelled) return;
        }
        if (ranged) {
          try {
            task = pdfjs.getDocument({
              url,
              rangeChunkSize: 256 * 1024,
              disableStream: true,
              disableAutoFetch: true,
            });
            const loaded = await task.promise;
            if (!cancelled) {
              setPdf(loaded);
              setPage((current) => Math.min(current, loaded.numPages));
            }
            return;
          } catch {
            task = undefined;
            if (cancelled) return;
            // Fall through to the bounded buffer path.
          }
        } else if (detail.bytes > MAX_INTERACTIVE_BYTES) {
          throw new Error("pdf-range-unsupported");
        }
      }
      if (!asset.data) return;
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [asset.data, detail.primaryAssetUrl, detail.bytes]);

  useEffect(() => {
    if (!pdf || !canvasRef.current) return;
    let cancelled = false;
    void (async () => {
      const pdfPage = await pdf.getPage(page);
      if (cancelled || !canvasRef.current) return;
      const preferredRaster = Math.min(window.devicePixelRatio || 1, 2);
      // Fit the whole page into the viewport, or cover it edge to edge.
      // `zoom` (buttons, pinch, Shift+wheel) multiplies on top. Clamp so a
      // small page doesn't balloon and a huge one still fits.
      const base = pdfPage.getViewport({ scale: 1 });
      if (![base.width, base.height].every(Number.isFinite) || base.width <= 0 || base.height <= 0) {
        throw new Error("invalid-pdf-page-dimensions");
      }
      const fitW = availWidth > 0 ? availWidth / base.width : 1;
      const fitH = availHeight > 0 ? availHeight / base.height : 1;
      const baseScale = mode === "fit" ? Math.min(fitW, fitH) : Math.max(fitW, fitH);
      const cssScale = Math.min(Math.max(baseScale * zoom, 0.1), 4, MAX_PDF_DISPLAY_SIDE / Math.max(base.width, base.height));
      const display = pdfPage.getViewport({ scale: cssScale });
      // Large windows and high display scale can otherwise allocate a canvas
      // of hundreds of megabytes. Keep the displayed zoom while bounding the
      // raster allocation; very large pages become slightly softer instead.
      const raster = boundedPdfRaster(display.width, display.height, preferredRaster);
      const viewport = pdfPage.getViewport({ scale: raster * cssScale });
      const canvas = canvasRef.current;
      const context = canvas.getContext("2d", { alpha: false });
      if (!context) throw new Error("canvas-context-unavailable");
      canvas.width = Math.max(1, Math.floor(viewport.width));
      canvas.height = Math.max(1, Math.floor(viewport.height));
      canvas.style.width = `${display.width}px`;
      canvas.style.height = `${display.height}px`;
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
  }, [page, pdf, zoom, mode, availWidth, availHeight, clarity]);

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

  // Large PDFs page in over Range (plan §3.2) instead of one arrayBuffer;
  // every other format keeps the 96 MiB interactive ceiling and falls back
  // to extracted text. Unbounded single GETs are refused above 512 MiB.
  const PDF_RANGE_MAX = 512 * 1024 * 1024;
  if (detail.bytes > MAX_INTERACTIVE_BYTES && detail.bytes > PDF_RANGE_MAX) {
    return <FileLimit detail={detail} fallback={fallback} />;
  }
  if (detail.bytes > MAX_INTERACTIVE_BYTES && asset.isError) {
    return <div className={styles.previewFallback}><ErrorState error={asset.error} onRetry={() => { void asset.refetch(); }} />{fallback}</div>;
  }
  if (error) return <div className={styles.previewFallback}><ErrorState error={error} onRetry={() => { setError(null); void asset.refetch(); }} />{fallback}</div>;

  return (
    <div className={styles.filePreview}>
      <div className={styles.fileToolbar}>
        <PageControls page={page} total={pdf?.numPages ?? detail.pageCount ?? 1} onPage={setPage} />
        <span className={styles.toolbarSpacer} />
        {ocr ? (
          <span className={styles.pageLabel}>{t("viewer.ocrProgress", { done: ocr.done, total: ocr.total })}</span>
        ) : detail.ocrStatus === "pending" || detail.ocrStatus === "partial" ? (
          <Button size="sm" variant="quiet" onClick={() => void runOcr()} disabled={!pdf}>
            {t("viewer.ocrRun")}
          </Button>
        ) : null}
        <div className={styles.zoomControls} role="group" aria-label={t("viewer.zoomMode")}>
          <Button
            size="sm"
            variant={mode === "fit" ? "primary" : "quiet"}
            aria-pressed={mode === "fit"}
            onClick={() => { setMode("fit"); setZoom(1); }}
          >
            {t("viewer.zoomFit")}
          </Button>
          <Button
            size="sm"
            variant={mode === "fill" ? "primary" : "quiet"}
            aria-pressed={mode === "fill"}
            onClick={() => { setMode("fill"); setZoom(1); }}
          >
            {t("viewer.zoomFill")}
          </Button>
        </div>
        <ZoomControls zoom={zoom} onZoom={setZoom} min={0.25} max={4} />
      </div>
      <div ref={viewportRef} className={styles.canvasViewport} data-note-scroll aria-label={t("viewer.pdfPreview")}>
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
  const [zoom, setZoom] = useState(1);
  const inverted = useUiStore((s) => s.docInverted);
  const clarity = useUiStore((s) => s.docClarity);

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
        <span className={styles.toolbarSpacer} />
        <ZoomControls zoom={zoom} onZoom={setZoom} />
      </div>
      <div
        className={styles.officeViewport} data-note-scroll
        aria-label={t("viewer.docxPreview")}
        ref={host}
        data-inverted={inverted}
        style={{ "--document-contrast": documentContrast(clarity), "--document-zoom": zoom } as React.CSSProperties}
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
  const [zoom, setZoom] = useState(1);
  const inverted = useUiStore((s) => s.docInverted);
  const clarity = useUiStore((s) => s.docClarity);
  const onPageRef = useRef(onPage);
  onPageRef.current = onPage;
  // Same external-jump contract as the PDF renderer (citation, notes).
  const initialRef = useRef(initialPage);
  useEffect(() => {
    if (initialPage !== initialRef.current) {
      initialRef.current = initialPage;
      setPage(Math.max(1, initialPage));
    }
  }, [initialPage]);
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
        <span className={styles.toolbarSpacer} />
        <ZoomControls zoom={zoom} onZoom={setZoom} />
      </div>
      <div className={styles.slideViewport} data-note-scroll aria-label={t("viewer.pptxPreview")}>
        <div
          ref={host}
          className={styles.slideHost}
          data-inverted={inverted}
          style={{ filter: `${inverted ? "invert(1) " : ""}contrast(${documentContrast(clarity)})`, "--document-zoom": zoom } as React.CSSProperties}
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
  const [zoom, setZoom] = useState(1);
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
        <span className={styles.toolbarSpacer} />
        <ZoomControls zoom={zoom} onZoom={setZoom} />
      </div>
      <div className={styles.body} data-note-scroll style={{ "--document-zoom": zoom } as React.CSSProperties}>{docs.isLoading ? <div className={styles.centered}>{t("states.analyzing")}…</div> : <MarkdownTable text={tables[selected]?.text ?? ""} />}</div>
    </div>
  );
}

function MarkdownTable({ text }: { text: string }) {
  const rows = text.split("\n").filter(Boolean).filter((_, index) => index !== 1).map((line) => line.slice(1, -1).split("|").map((cell) => cell.trim().replace(/\\\|/g, "|")));
  return <table className={styles.sheet} style={{ zoom: "var(--document-zoom, 1)" }}><thead><tr>{(rows[0] ?? []).map((cell, index) => <th key={index}>{cell}</th>)}</tr></thead><tbody>{rows.slice(1).map((row, ri) => <tr key={ri}>{row.map((cell, ci) => <td key={ci}>{cell}</td>)}</tr>)}</tbody></table>;
}

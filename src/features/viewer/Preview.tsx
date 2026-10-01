import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { keepPreviousData, useQuery, useQueryClient } from "@tanstack/react-query";
import type { QueryClient } from "@tanstack/react-query";
import { Markdown } from "../../components/Markdown";
import { Skeleton } from "../../components/Skeleton";
import { ErrorState } from "../../components/ErrorState";
import { IconButton } from "../../components/IconButton";
import { Button } from "../../components/Button";
import { ChevronRightIcon, CloseIcon } from "../../app/Icons";
import { documentApi, viewerApi } from "../../ipc/viewer";
import type { SourceDetail, SourceKind, ViewerTab } from "../../ipc/types.gen";
import { DocxFilePreview, PdfFilePreview, PptxFilePreview, ScrollNav, WorkbookPreview } from "./FilePreviews";
import { WebsitePreview } from "./WebsitePreview";
import { ZoomControls } from "./ZoomControls";
import styles from "./previews.module.css";

/** Keep the in-memory rail state consistent with the durable locator. Without
 * this, switching tabs remounts a paged preview from the stale query result
 * even though SQLite already contains the newer page. */
export function rememberTabLocator(
  qc: QueryClient,
  projectId: string,
  tabId: string,
  locator: unknown,
) {
  qc.setQueryData<ViewerTab[]>(["viewer-tabs", projectId], (tabs) =>
    tabs?.map((tab) => (tab.id === tabId ? { ...tab, locator } : tab)),
  );
}

/* ───────────────────────── shared bits ───────────────────────── */

function useAssetText(projectId: string, sourceId: string, rel: string | null) {
  return useQuery({
    queryKey: ["asset-text", projectId, sourceId, rel],
    enabled: !!rel,
    queryFn: async () => {
      const url = await documentApi.assetUrl(projectId, sourceId, rel!);
      const res = await fetch(url);
      if (!res.ok) throw new Error(`asset ${res.status}`);
      return res.text();
    },
  });
}

function Toolbar({ children }: { children?: React.ReactNode }) {
  const { t } = useTranslation();
  return (
    <div className={styles.toolbar}>
      {children}
      <span className={styles.toolbarSpacer} />
      <span className={styles.readonly}>{t("viewer.readonly")}</span>
    </div>
  );
}

/* ───────────────────────── text / code ───────────────────────── */

/** Above this size the whole file is never handed to the WebView (plan §3.2):
 * the UI pages bounded UTF-8 windows instead. */
const TEXT_WINDOW_THRESHOLD = 256 * 1024;
const TEXT_WINDOW_LIMIT = 128 * 1024;

function TextPreview({
  projectId,
  tab,
  markdown,
  sizeBytes,
}: {
  projectId: string;
  tab: ViewerTab;
  markdown: boolean;
  sizeBytes: number;
}) {
  const { t } = useTranslation();
  const rel = `sources/${tab.sourceId}/${tab.name}`;
  const windowed = sizeBytes > TEXT_WINDOW_THRESHOLD;
  const [offset, setOffset] = useState(0);
  // A new source always starts at its head, even when the preview instance
  // is reused across tab switches.
  useEffect(() => {
    setOffset(0);
  }, [projectId, tab.sourceId]);
  const win = useQuery({
    queryKey: ["text-window", projectId, tab.sourceId, offset],
    enabled: windowed,
    placeholderData: keepPreviousData,
    queryFn: () => documentApi.readWindow(projectId, tab.sourceId, offset, TEXT_WINDOW_LIMIT),
  });
  const q = useAssetText(projectId, tab.sourceId, windowed ? null : rel);
  const text = windowed ? (win.data?.text ?? "") : (q.data ?? "");
  const isLoading = windowed ? win.isLoading : q.isLoading;
  const isError = windowed ? win.isError : q.isError;
  const refetch = windowed ? win.refetch : q.refetch;
  const [wrap, setWrap] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [rendered, setRendered] = useState(markdown);
  const [find, setFind] = useState<string>("");
  const [findOpen, setFindOpen] = useState(false);
  const [activeHit, setActiveHit] = useState(0);
  const bodyRef = useRef<HTMLDivElement>(null);

  const lines = useMemo(() => text.split("\n"), [text]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setFindOpen(true);
      } else if (e.key === "Escape" && findOpen) {
        setFindOpen(false);
        setFind("");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [findOpen]);

  const hits = useMemo(() => {
    if (!find) return 0;
    const re = new RegExp(escapeRe(find), "gi");
    return text.match(re)?.length ?? 0;
  }, [find, text]);

  useEffect(() => {
    setActiveHit(0);
  }, [find]);

  useEffect(() => {
    if (!find) return;
    const el = bodyRef.current?.querySelectorAll("mark.hit")[activeHit];
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [activeHit, find, lines]);

  if (isLoading) return <LoadingRows />;
  if (isError) return <ErrorState error={(windowed ? win.error : q.error) as Error} onRetry={() => { void refetch(); }} />;

  const pager = windowed && win.data ? (
    <span className={styles.pageLabel} role="status">
      {t("viewer.windowPosition", {
        offset: win.data.offset,
        total: win.data.totalBytes,
        startLine: win.data.startLine,
      })}
    </span>
  ) : null;
  const windowNav = windowed && win.data ? (
    <>
      <Button
        size="sm"
        variant="quiet"
        disabled={offset <= 0 || win.isFetching}
        onClick={() => setOffset(0)}
      >
        {t("viewer.windowFirst")}
      </Button>
      <Button
        size="sm"
        variant="quiet"
        disabled={offset <= 0 || win.isFetching}
        onClick={() => setOffset((o) => Math.max(0, o - TEXT_WINDOW_LIMIT))}
      >
        {t("viewer.windowPrev")}
      </Button>
      <Button
        size="sm"
        variant="quiet"
        disabled={win.data.nextOffset == null || win.isFetching}
        onClick={() => win.data?.nextOffset != null && setOffset(win.data.nextOffset)}
      >
        {t("viewer.windowNext")}
      </Button>
    </>
  ) : null;

  if (markdown && rendered) {
    return (
      <div className={styles.wrap}>
        <Toolbar>
          <Button size="sm" variant="quiet" onClick={() => setRendered(false)}>
            {t("viewer.showSource")}
          </Button>
          {windowNav}
          {pager}
          <ZoomControls zoom={zoom} onZoom={setZoom} />
        </Toolbar>
        {windowed ? <p className={styles.note}>{t("viewer.windowedNotice")}</p> : null}
        <div className={styles.body} ref={bodyRef}>
          <div className={styles.reading} style={{ zoom }}>
            <Markdown>{text}</Markdown>
          </div>
        </div>
      </div>
    );
  }

  let hitCounter = 0;
  return (
    <div className={styles.wrap}>
      <Toolbar>
        <Button size="sm" variant="quiet" onClick={() => setWrap((w) => !w)}>
          {wrap ? t("viewer.nowrap") : t("viewer.wrap")}
        </Button>
        {markdown ? (
          <Button size="sm" variant="quiet" onClick={() => setRendered(true)}>
            {t("viewer.showRendered")}
          </Button>
        ) : null}
        {windowNav}
        {pager}
        <ZoomControls zoom={zoom} onZoom={setZoom} />
      </Toolbar>
      {windowed ? <p className={styles.note}>{t("viewer.windowedNotice")}</p> : null}
      <div className={styles.body} ref={bodyRef}>
        {findOpen ? (
          <div className={styles.findBar} role="search">
            <input
              autoFocus
              value={find}
              placeholder={t("viewer.find")}
              onChange={(e) => setFind(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") setActiveHit((h) => (hits ? (h + 1) % hits : 0));
              }}
            />
            <span className={styles.findCount}>
              {find ? `${hits ? activeHit + 1 : 0}/${hits}` : ""}
            </span>
            <IconButton label={t("common.close")} size="sm" onClick={() => { setFindOpen(false); setFind(""); }}>
              <CloseIcon size={13} />
            </IconButton>
          </div>
        ) : null}
        <div className={styles.code} data-wrap={wrap} style={{ zoom }}>
          <div className={styles.gutter} aria-hidden="true">
            {lines.map((_, i) => (
              <span key={i}>{(windowed ? (win.data?.startLine ?? 1) - 1 : 0) + i + 1}</span>
            ))}
          </div>
          <div className={styles.lines}>
            {lines.map((ln, i) => (
              <div key={i} className={styles.line}>
                {find
                  ? highlight(ln, find, () => hitCounter++, activeHit)
                  : ln || " "}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

/* ───────────────────────── paged (pdf / slides) ───────────────────────── */

function PagedPreview({
  projectId,
  tab,
  total,
  onContext,
}: {
  projectId: string;
  tab: ViewerTab;
  total: number;
  onContext?: (c: { sourceId: string; locator: unknown; position?: string }) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [page, setPage] = useState(
    typeof (tab.locator as { page?: number })?.page === "number"
      ? (tab.locator as { page: number }).page
      : 1,
  );
  const clamped = Math.min(Math.max(page, 1), Math.max(total, 1));

  const doc = useQuery({
    queryKey: ["document", projectId, tab.sourceId, clamped],
    queryFn: () => documentApi.get(projectId, tab.sourceId, clamped),
  });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement as HTMLElement | null;
      if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) return;
      if (e.key === "ArrowRight" || e.key === "PageDown") setPage((p) => Math.min(p + 1, total));
      else if (e.key === "ArrowLeft" || e.key === "PageUp") setPage((p) => Math.max(p - 1, 1));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [total]);

  useEffect(() => {
    const locator = { t: "page", page: clamped };
    rememberTabLocator(qc, projectId, tab.id, locator);
    void viewerApi.updateLocator(projectId, tab.id, locator).catch(() => {});
    onContext?.({
      sourceId: tab.sourceId,
      locator: { t: "page", page: clamped },
      position: `${clamped} / ${total}`,
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [clamped, projectId, tab.id, total]);

  return (
    <div className={styles.wrap}>
      <Toolbar>
        <IconButton
          label={t("viewer.prevPage")}
          size="sm"
          disabled={clamped <= 1}
          onClick={() => setPage((p) => Math.max(p - 1, 1))}
        >
          <ChevronRightIcon size={16} style={{ transform: "rotate(180deg)" }} />
        </IconButton>
        <span className={styles.pageLabel}>
          {clamped} / {total}
        </span>
        <IconButton
          label={t("viewer.nextPage")}
          size="sm"
          disabled={clamped >= total}
          onClick={() => setPage((p) => Math.min(p + 1, total))}
        >
          <ChevronRightIcon size={16} />
        </IconButton>
      </Toolbar>
      <div className={styles.body}>
        <p className={styles.note}>{t("viewer.extractedTextFallback")}</p>
        {doc.isLoading ? (
          <LoadingRows />
        ) : doc.isError ? (
          <ErrorState error={doc.error} onRetry={() => doc.refetch()} />
        ) : (
          <div className={styles.reading}>
            {doc.data?.title ? <h2>{doc.data.title}</h2> : null}
            <pre style={{ whiteSpace: "pre-wrap", fontFamily: "var(--font-read)" }}>
              {doc.data?.text}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

/* ───────────────────────── image ───────────────────────── */

function ImagePreview({ url }: { url: string }) {
  const [zoom, setZoom] = useState(1);
  return (
    <div className={styles.wrap}>
      <Toolbar>
        <ZoomControls zoom={zoom} onZoom={setZoom} min={0.25} max={4} />
      </Toolbar>
      <div className={styles.body}>
        <div className={styles.imageBody}>
          <img src={url} alt="" style={{ zoom }} />
        </div>
      </div>
    </div>
  );
}

/* ───────────────────────── sheet (csv/tsv) ───────────────────────── */

function SheetPreview({ projectId, tab, sizeBytes }: { projectId: string; tab: ViewerTab; sizeBytes: number }) {
  const { t } = useTranslation();
  const rel = `sources/${tab.sourceId}/${tab.name}`;
  const windowed = sizeBytes > TEXT_WINDOW_THRESHOLD;
  const [offset, setOffset] = useState(0);
  useEffect(() => {
    setOffset(0);
  }, [projectId, tab.sourceId]);
  const full = useAssetText(projectId, tab.sourceId, windowed ? null : rel);
  const win = useQuery({
    queryKey: ["text-window", projectId, tab.sourceId, offset],
    enabled: windowed,
    placeholderData: keepPreviousData,
    queryFn: () => documentApi.readWindow(projectId, tab.sourceId, offset, TEXT_WINDOW_LIMIT),
  });
  const headerQ = useQuery({
    queryKey: ["text-window", projectId, tab.sourceId, "header"],
    enabled: windowed,
    placeholderData: keepPreviousData,
    queryFn: () => documentApi.readWindow(projectId, tab.sourceId, 0, 8192),
  });
  const delim = tab.name.toLowerCase().endsWith(".tsv") ? "\t" : ",";

  if (!windowed) {
    const rows = parseDelimited(full.data ?? "", delim);
    if (full.isLoading) return <LoadingRows />;
    if (full.isError) return <ErrorState error={full.error} onRetry={() => full.refetch()} />;
    return (
      <div className={styles.wrap}>
        <Toolbar />
        <div className={styles.body}>
          <SheetTable head={rows[0] ?? []} rows={rows.slice(1, 2000)} startRow={2} />
        </div>
      </div>
    );
  }

  const headerRows = parseDelimited((headerQ.data?.text ?? "").split("\n")[0] ?? "", delim);
  const head = headerRows[0] ?? [];
  // Drop a possibly-partial first line when paging mid-file (the backend
  // extends windows to line ends, so only offset 0 starts cleanly).
  const bodyText = offset === 0
    ? (win.data?.text ?? "").split("\n").slice(1).join("\n")
    : (win.data?.text ?? "");
  const rows = parseDelimited(bodyText, delim);
  // Absolute row numbers: header is row 1, body starts at startLine (+1 when
  // the header line was stripped from a continuation window).
  const startRow = (win.data?.startLine ?? 1) + 1;

  if (win.isLoading) return <LoadingRows />;
  if (win.isError) return <ErrorState error={win.error} onRetry={() => win.refetch()} />;

  return (
    <div className={styles.wrap}>
      <Toolbar>
        <Button
          size="sm"
          variant="quiet"
          disabled={offset <= 0 || win.isFetching}
          onClick={() => setOffset(0)}
        >
          {t("viewer.windowFirst")}
        </Button>
        <Button
          size="sm"
          variant="quiet"
          disabled={offset <= 0 || win.isFetching}
          onClick={() => setOffset((o) => Math.max(0, o - TEXT_WINDOW_LIMIT))}
        >
          {t("viewer.windowPrev")}
        </Button>
        <Button
          size="sm"
          variant="quiet"
          disabled={win.data?.nextOffset == null || win.isFetching}
          onClick={() => win.data?.nextOffset != null && setOffset(win.data.nextOffset)}
        >
          {t("viewer.windowNext")}
        </Button>
        {win.data ? (
          <span className={styles.pageLabel} role="status">
            {t("viewer.sheetPosition", {
              first: startRow,
              last: startRow + Math.max(rows.length - 1, 0),
            })}
          </span>
        ) : null}
      </Toolbar>
      <p className={styles.note}>{t("viewer.windowedNotice")}</p>
      <div className={styles.body}>
        <SheetTable head={head} rows={rows.slice(0, 500)} startRow={startRow} />
      </div>
    </div>
  );
}

function SheetTable({ head, rows, startRow }: { head: string[]; rows: string[][]; startRow: number }) {
  return (
    <table className={styles.sheet}>
      <thead>
        <tr>
          <th aria-label="#">#</th>
          {head.map((c, i) => (
            <th key={i}>{c}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((r, ri) => (
          <tr key={ri}>
            <td className="u-mono-nums">{startRow + ri}</td>
            {r.map((c, ci) => (
              <td key={ci}>{c}</td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/* ───────────────────────── reading (md canonical / web) ───────────────────────── */

function ReadingPreview({
  projectId,
  detail,
  originalUrl,
}: {
  projectId: string;
  detail: SourceDetail;
  originalUrl?: string | null;
}) {
  const { t } = useTranslation();
  // Opt-in JS rendering for weblinks (docs/05 §4 lineage): the extracted
  // reader view stays the default so RAG grounding never changes, and remote
  // scripts run only after an explicit tap, inside the same opaque-origin
  // sandbox (`allow-scripts` only, no popups, no referrer) as site previews.
  const liveUrl =
    detail.kind === "weblink" && originalUrl && /^https?:\/\//i.test(originalUrl)
      ? originalUrl
      : null;
  const [live, setLive] = useState(false);
  // A new document always starts in reader view: never inherit a live
  // session (and its remote scripts) from the previous document.
  useEffect(() => {
    setLive(false);
  }, [detail.id]);
  // While the live frame is up, the reader text is off screen — don't fetch
  // it. Returning to reader view refetches through the usual caches.
  const showLive = live && !!liveUrl;
  const rel =
    showLive || detail.primaryAssetUrl ? null : `derived/${detail.id}/document.md`;
  const direct = useQuery({
    queryKey: ["reader", detail.id, detail.primaryAssetUrl],
    enabled: !!detail.primaryAssetUrl && !showLive,
    queryFn: async () => {
      const res = await fetch(detail.primaryAssetUrl!);
      return res.text();
    },
  });
  const viaDerived = useAssetText(projectId, detail.id, rel);
  const text = detail.primaryAssetUrl ? direct.data : viaDerived.data;
  const loading = detail.primaryAssetUrl ? direct.isLoading : viaDerived.isLoading;
  const bodyRef = useRef<HTMLDivElement>(null);

  return (
    <div className={styles.wrap}>
      <Toolbar>
        {liveUrl ? (
          <>
            <Button
              size="sm"
              variant={live ? "quiet" : "secondary"}
              aria-pressed={!live}
              onClick={() => setLive(false)}
            >
              {t("viewer.webReader")}
            </Button>
            <Button
              size="sm"
              variant={live ? "secondary" : "quiet"}
              aria-pressed={live}
              onClick={() => setLive(true)}
            >
              {t("viewer.webLive")}
            </Button>
            <Button
              size="sm"
              variant="quiet"
              onClick={() =>
                void import("@tauri-apps/plugin-opener").then((m) =>
                  m.openUrl(liveUrl),
                )
              }
            >
              {t("viewer.openOriginal")}
            </Button>
          </>
        ) : null}
      </Toolbar>
      <div className={styles.body} ref={bodyRef}>
        {showLive ? (
          <div className={styles.liveWrap}>
            <iframe
              className={styles.liveFrame}
              src={liveUrl}
              title={liveUrl}
              sandbox="allow-scripts"
              referrerPolicy="no-referrer"
            />
          </div>
        ) : loading ? (
          <LoadingRows />
        ) : (
          <>
            <div className={styles.reading}>
              <Markdown>{text ?? ""}</Markdown>
            </div>
            <ScrollNav targetRef={bodyRef} />
          </>
        )}
      </div>
    </div>
  );
}

/* ───────────────────────── dispatcher ───────────────────────── */

const TEXTY: SourceKind[] = ["text", "code", "json", "jsonl"];

export function Preview({
  projectId,
  tab,
  onContext,
}: {
  projectId: string;
  tab: ViewerTab;
  onContext?: (c: { sourceId: string; locator: unknown; position?: string }) => void;
}) {
  const qc = useQueryClient();
  const detail = useQuery({
    queryKey: ["source-detail", projectId, tab.sourceId],
    queryFn: () => documentApi.detail(projectId, tab.sourceId),
  });

  const paged = detail.data?.kind === "pdf" || detail.data?.kind === "slides";
  useEffect(() => {
    if (detail.data && !paged) {
      onContext?.({ sourceId: tab.sourceId, locator: { t: "whole" } });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [detail.data?.id, paged]);

  if (detail.isLoading) return <LoadingRows />;
  if (detail.isError)
    return <ErrorState error={detail.error} onRetry={() => detail.refetch()} />;

  const d = detail.data!;
  if (d.status === "queued" || d.status === "analyzing") {
    return <AnalyzingState />;
  }

  if (d.kind === "image" && d.primaryAssetUrl) {
    return <ImagePreview url={d.primaryAssetUrl} />;
  }
  if (d.kind === "weblink") {
    return <ReadingPreview projectId={projectId} detail={d} originalUrl={d.url} />;
  }
  if (d.kind === "website") {
    return <WebsitePreview projectId={projectId} detail={d} />;
  }
  if (d.kind === "pdf" && d.primaryAssetUrl) {
    const initialPage = typeof (tab.locator as { page?: number })?.page === "number"
      ? (tab.locator as { page: number }).page
      : 1;
    return <PdfFilePreview detail={d} projectId={projectId} initialPage={initialPage} onPage={(page, total) => {
      const locator = { t: "page", page };
      rememberTabLocator(qc, projectId, tab.id, locator);
      void viewerApi.updateLocator(projectId, tab.id, locator).catch(() => {});
      onContext?.({ sourceId: tab.sourceId, locator, position: `${page} / ${total}` });
    }} fallback={<PagedPreview projectId={projectId} tab={tab} total={d.pageCount ?? 1} onContext={onContext} />} />;
  }
  if (d.kind === "slides") {
    const fallback = <PagedPreview projectId={projectId} tab={tab} total={d.pageCount ?? 1} onContext={onContext} />;
    if (!d.primaryAssetUrl || !d.name.toLowerCase().endsWith(".pptx")) return fallback;
    const initialPage = typeof (tab.locator as { page?: number })?.page === "number"
      ? (tab.locator as { page: number }).page
      : 1;
    return <PptxFilePreview detail={d} initialPage={initialPage} onPage={(page, total) => {
      const locator = { t: "page", page };
      rememberTabLocator(qc, projectId, tab.id, locator);
      void viewerApi.updateLocator(projectId, tab.id, locator).catch(() => {});
      onContext?.({ sourceId: tab.sourceId, locator, position: `${page} / ${total}` });
    }} fallback={fallback} />;
  }
  if (d.kind === "markdown") {
    return <TextPreview projectId={projectId} tab={tab} markdown sizeBytes={Number(d.bytes)} />;
  }
  if (TEXTY.includes(d.kind)) {
    return <TextPreview projectId={projectId} tab={tab} markdown={false} sizeBytes={Number(d.bytes)} />;
  }
  if (d.kind === "sheet") {
    return d.name.toLowerCase().match(/\.(csv|tsv)$/)
      ? <SheetPreview projectId={projectId} tab={tab} sizeBytes={Number(d.bytes)} />
      : <WorkbookPreview projectId={projectId} detail={d} />;
  }
  if ((d.kind === "audio" || d.kind === "video") && d.primaryAssetUrl) {
    return <AvPreview projectId={projectId} detail={d} />;
  }
  if (d.kind === "doc" && d.primaryAssetUrl && d.name.toLowerCase().endsWith(".docx")) {
    return <DocxFilePreview detail={d} fallback={<ReadingPreview projectId={projectId} detail={{ ...d, primaryAssetUrl: null }} />} />;
  }
  // Legacy DOC and other non-browser formats fall back to canonical Markdown.
  return <ReadingPreview projectId={projectId} detail={d} />;
}

/* ───────────────────────── audio / video ───────────────────────── */

function parseTs(title: string | null): number {
  const m = (title ?? "").match(/(\d+):(\d\d)/);
  return m ? Number(m[1]) * 60 + Number(m[2]) : 0;
}

function AvPreview({ projectId, detail }: { projectId: string; detail: SourceDetail }) {
  const { t } = useTranslation();
  const mediaRef = useRef<HTMLMediaElement>(null);
  const [now, setNow] = useState(0);
  const [keyframes, setKeyframes] = useState<Array<{ time: number; url: string }>>([]);
  const total = detail.pageCount ?? 1;

  const segs = useQuery({
    queryKey: ["av-transcript", projectId, detail.id, total],
    queryFn: () =>
      Promise.all(
        Array.from({ length: total }, (_, i) =>
          documentApi.get(projectId, detail.id, i),
        ),
      ),
  });

  const seek = (sec: number) => {
    const el = mediaRef.current;
    if (!el) return;
    el.currentTime = sec;
    void el.play();
  };

  useEffect(() => {
    if (detail.kind !== "video" || !detail.primaryAssetUrl) return;
    let cancelled = false;
    const video = document.createElement("video");
    video.muted = true;
    video.preload = "auto";
    video.src = detail.primaryAssetUrl;
    const wait = (event: "loadedmetadata" | "seeked") =>
      new Promise<void>((resolve, reject) => {
      const timeout = window.setTimeout(() => reject(new Error("video-frame-timeout")), 10_000);
      video.addEventListener(event, () => { window.clearTimeout(timeout); resolve(); }, { once: true });
      video.addEventListener("error", () => { window.clearTimeout(timeout); reject(new Error("video-frame-error")); }, { once: true });
      });
    void (async () => {
      await wait("loadedmetadata");
      if (!Number.isFinite(video.duration) || video.duration <= 0) return;
      const count = Math.min(8, Math.max(2, Math.ceil(video.duration / 120)));
      const canvas = document.createElement("canvas");
      const scale = Math.min(1, 320 / Math.max(video.videoWidth, 1));
      canvas.width = Math.max(1, Math.round(video.videoWidth * scale));
      canvas.height = Math.max(1, Math.round(video.videoHeight * scale));
      const context = canvas.getContext("2d");
      if (!context) return;
      const frames: Array<{ time: number; url: string }> = [];
      for (let index = 0; index < count && !cancelled; index++) {
        const time = Math.min(video.duration - .05, ((index + .5) / count) * video.duration);
        video.currentTime = Math.max(0, time);
        await wait("seeked");
        context.drawImage(video, 0, 0, canvas.width, canvas.height);
        frames.push({ time, url: canvas.toDataURL("image/jpeg", .72) });
      }
      if (!cancelled) setKeyframes(frames);
    })().catch(() => {});
    return () => {
      cancelled = true;
      video.removeAttribute("src");
      video.load();
    };
  }, [detail.id, detail.kind, detail.primaryAssetUrl]);

  return (
    <div className={styles.av}>
      {detail.kind === "video" ? (
        <video
          ref={mediaRef as React.RefObject<HTMLVideoElement>}
          className={styles.avMedia}
          src={detail.primaryAssetUrl ?? undefined}
          controls
          onTimeUpdate={(e) => setNow(e.currentTarget.currentTime)}
        />
      ) : (
        <audio
          ref={mediaRef as React.RefObject<HTMLAudioElement>}
          className={styles.avAudio}
          src={detail.primaryAssetUrl ?? undefined}
          controls
          onTimeUpdate={(e) => setNow(e.currentTarget.currentTime)}
        />
      )}

      {keyframes.length ? (
        <div className={styles.keyframes} aria-label={t("viewer.keyframes")}>
          {keyframes.map((frame) => (
            <button key={frame.time} type="button" onClick={() => seek(frame.time)}>
              <img src={frame.url} alt="" />
              <span>{parseTime(frame.time)}</span>
            </button>
          ))}
        </div>
      ) : null}

      {segs.isLoading ? (
        <LoadingRows />
      ) : (
        <ol className={styles.transcript}>
          {(segs.data ?? []).map((doc) => {
            const start = parseTs(doc.title);
            const active = now >= start && now < start + 30;
            return (
              <li key={doc.ordinal}>
                <button
                  type="button"
                  className={active ? styles.segActive : styles.seg}
                  onClick={() => seek(start)}
                >
                  <span className={`${styles.segTime} u-mono-nums`}>{doc.title}</span>
                  <span className={styles.segText}>{doc.text}</span>
                </button>
              </li>
            );
          })}
          {(segs.data ?? []).length === 0 ? (
            <li className={styles.segEmpty}>{t("viewer.noTranscript")}</li>
          ) : null}
        </ol>
      )}
    </div>
  );
}

/* ───────────────────────── helpers ───────────────────────── */

function LoadingRows() {
  return (
    <div style={{ padding: "1.5rem", display: "flex", flexDirection: "column", gap: "0.5rem" }}>
      {Array.from({ length: 8 }).map((_, i) => (
        <Skeleton key={i} height="1rem" width={`${90 - (i % 3) * 15}%`} />
      ))}
    </div>
  );
}

function AnalyzingState() {
  const { t } = useTranslation();
  return <div className={styles.centered}>{t("states.analyzing")}…</div>;
}

function escapeRe(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function highlight(
  line: string,
  term: string,
  nextIndex: () => number,
  active: number,
): React.ReactNode {
  const re = new RegExp(escapeRe(term), "gi");
  const out: React.ReactNode[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(line))) {
    if (m.index > last) out.push(line.slice(last, m.index));
    const idx = nextIndex();
    out.push(
      <mark key={idx} className={idx === active ? "hit hitActive" : "hit"}>
        {m[0]}
      </mark>,
    );
    last = m.index + m[0].length;
    if (m.index === re.lastIndex) re.lastIndex++;
  }
  if (last < line.length) out.push(line.slice(last));
  return out.length ? out : line || " ";
}

function parseDelimited(text: string, delim: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (c === '"' && text[i + 1] === '"') {
        cell += '"';
        i++;
      } else if (c === '"') {
        quoted = false;
      } else {
        cell += c;
      }
    } else if (c === '"') {
      quoted = true;
    } else if (c === delim) {
      row.push(cell);
      cell = "";
    } else if (c === "\n" || c === "\r") {
      if (c === "\r" && text[i + 1] === "\n") i++;
      row.push(cell);
      rows.push(row);
      row = [];
      cell = "";
    } else {
      cell += c;
    }
  }
  if (cell || row.length) {
    row.push(cell);
    rows.push(row);
  }
  return rows.filter((r) => r.some((c) => c.length));
}

function parseTime(value: number) {
  const seconds = Math.max(0, Math.floor(value));
  return `${Math.floor(seconds / 60).toString().padStart(2, "0")}:${(seconds % 60).toString().padStart(2, "0")}`;
}

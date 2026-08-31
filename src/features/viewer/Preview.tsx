import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Markdown } from "../../components/Markdown";
import { Skeleton } from "../../components/Skeleton";
import { ErrorState } from "../../components/ErrorState";
import { IconButton } from "../../components/IconButton";
import { Button } from "../../components/Button";
import { ChevronRightIcon, CloseIcon } from "../../app/Icons";
import { documentApi } from "../../ipc/viewer";
import { call } from "../../ipc/client";
import type { SourceDetail, SourceKind, ViewerTab } from "../../ipc/types.gen";
import styles from "./previews.module.css";

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

function TextPreview({
  projectId,
  tab,
  markdown,
}: {
  projectId: string;
  tab: ViewerTab;
  markdown: boolean;
}) {
  const { t } = useTranslation();
  const rel = `sources/${tab.sourceId}/${tab.name}`;
  const q = useAssetText(projectId, tab.sourceId, rel);
  const [wrap, setWrap] = useState(false);
  const [rendered, setRendered] = useState(markdown);
  const [find, setFind] = useState<string>("");
  const [findOpen, setFindOpen] = useState(false);
  const [activeHit, setActiveHit] = useState(0);
  const bodyRef = useRef<HTMLDivElement>(null);

  const lines = useMemo(() => (q.data ?? "").split("\n"), [q.data]);

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
    return (q.data ?? "").match(re)?.length ?? 0;
  }, [find, q.data]);

  useEffect(() => {
    setActiveHit(0);
  }, [find]);

  useEffect(() => {
    if (!find) return;
    const el = bodyRef.current?.querySelectorAll("mark.hit")[activeHit];
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [activeHit, find, lines]);

  if (q.isLoading) return <LoadingRows />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => q.refetch()} />;

  if (markdown && rendered) {
    return (
      <div className={styles.wrap}>
        <Toolbar>
          <Button size="sm" variant="quiet" onClick={() => setRendered(false)}>
            {t("viewer.showSource")}
          </Button>
        </Toolbar>
        <div className={styles.body} ref={bodyRef}>
          <div className={styles.reading}>
            <Markdown>{q.data ?? ""}</Markdown>
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
      </Toolbar>
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
        <div className={styles.code} data-wrap={wrap}>
          <div className={styles.gutter} aria-hidden="true">
            {lines.map((_, i) => (
              <span key={i}>{i + 1}</span>
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
      if (e.key === "ArrowRight" || e.key === "PageDown") setPage((p) => Math.min(p + 1, total));
      else if (e.key === "ArrowLeft" || e.key === "PageUp") setPage((p) => Math.max(p - 1, 1));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [total]);

  useEffect(() => {
    void call("viewer_update_locator", {
      projectId,
      tabId: tab.id,
      locator: { t: "page", page: clamped },
    }).catch(() => {});
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
        <p className={styles.note}>{t("viewer.rasterDeferred")}</p>
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
        <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.max(0.25, z - 0.25))}>
          −
        </Button>
        <span className={styles.pageLabel}>{Math.round(zoom * 100)}%</span>
        <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.min(4, z + 0.25))}>
          +
        </Button>
        <Button size="sm" variant="quiet" onClick={() => setZoom(1)}>
          1:1
        </Button>
      </Toolbar>
      <div className={styles.body}>
        <div className={styles.imageBody}>
          <img src={url} alt="" style={{ transform: `scale(${zoom})` }} />
        </div>
      </div>
    </div>
  );
}

/* ───────────────────────── sheet (csv/tsv) ───────────────────────── */

function SheetPreview({ projectId, tab }: { projectId: string; tab: ViewerTab }) {
  const rel = `sources/${tab.sourceId}/${tab.name}`;
  const q = useAssetText(projectId, tab.sourceId, rel);
  const delim = tab.name.toLowerCase().endsWith(".tsv") ? "\t" : ",";
  const rows = useMemo(() => parseDelimited(q.data ?? "", delim), [q.data, delim]);

  if (q.isLoading) return <LoadingRows />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => q.refetch()} />;

  return (
    <div className={styles.wrap}>
      <Toolbar />
      <div className={styles.body}>
        <table className={styles.sheet}>
          <thead>
            <tr>
              {(rows[0] ?? []).map((c, i) => (
                <th key={i}>{c}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.slice(1, 2000).map((r, ri) => (
              <tr key={ri}>
                {r.map((c, ci) => (
                  <td key={ci}>{c}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
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
  const rel = detail.primaryAssetUrl
    ? null
    : `derived/${detail.id}/document.md`;
  const direct = useQuery({
    queryKey: ["reader", detail.id, detail.primaryAssetUrl],
    enabled: !!detail.primaryAssetUrl,
    queryFn: async () => {
      const res = await fetch(detail.primaryAssetUrl!);
      return res.text();
    },
  });
  const viaDerived = useAssetText(projectId, detail.id, rel);
  const text = detail.primaryAssetUrl ? direct.data : viaDerived.data;
  const loading = detail.primaryAssetUrl ? direct.isLoading : viaDerived.isLoading;

  return (
    <div className={styles.wrap}>
      <Toolbar>
        {detail.kind === "weblink" && originalUrl ? (
          <Button
            size="sm"
            variant="quiet"
            onClick={() =>
              void import("@tauri-apps/plugin-opener").then((m) =>
                m.openUrl(originalUrl),
              )
            }
          >
            {t("viewer.openOriginal")}
          </Button>
        ) : null}
      </Toolbar>
      <div className={styles.body}>
        {loading ? (
          <LoadingRows />
        ) : (
          <div className={styles.reading}>
            <Markdown>{text ?? ""}</Markdown>
          </div>
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
  if (d.kind === "pdf" || d.kind === "slides") {
    return <PagedPreview projectId={projectId} tab={tab} total={d.pageCount ?? 1} onContext={onContext} />;
  }
  if (d.kind === "markdown") {
    return <TextPreview projectId={projectId} tab={tab} markdown />;
  }
  if (TEXTY.includes(d.kind)) {
    return <TextPreview projectId={projectId} tab={tab} markdown={false} />;
  }
  if (d.kind === "sheet") {
    return d.name.toLowerCase().match(/\.(csv|tsv)$/)
      ? <SheetPreview projectId={projectId} tab={tab} />
      : <ReadingPreview projectId={projectId} detail={d} />;
  }
  if ((d.kind === "audio" || d.kind === "video") && d.primaryAssetUrl) {
    return <AvPreview projectId={projectId} detail={d} />;
  }
  // doc falls back to the canonical markdown.
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

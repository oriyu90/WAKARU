import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { notesApi } from "../../ipc/notes";
import { viewerApi } from "../../ipc/viewer";
import { IpcError, inTauri } from "../../ipc/client";
import type { Note, ViewerTab } from "../../ipc/types.gen";
import { useToast } from "../../components/useToast";
import { rememberTabLocator } from "./Preview";
import styles from "./NotesOverlay.module.css";

/** Client-side note body ceiling mirrors the backend (plan §5.1). */
const BODY_MAX = 4000;
const SAVE_DEBOUNCE_MS = 600;
/** Band colours shared by the dot and the card edge (backend-validated). */
const NOTE_COLORS = [
  "yellow",
  "pink",
  "blue",
  "green",
  "orange",
  "purple",
  "teal",
] as const;

type Anchor = { page?: number; x?: number; y?: number; [k: string]: unknown };

function asAnchor(note: Note): Anchor {
  const v = note.anchorJson;
  return v && typeof v === "object" ? (v as Anchor) : {};
}

function currentPageOf(locator: unknown): number | null {
  if (locator && typeof locator === "object") {
    const p = (locator as { page?: unknown }).page;
    if (typeof p === "number" && Number.isFinite(p)) return p;
  }
  return null;
}

/** Scroll metrics of the material scroller (`data-note-scroll`). Markers
 * live in document fractions and are projected into the viewport, so they
 * scroll together with the document. */
type ScrollMetrics = {
  sl: number;
  st: number;
  sw: number;
  sh: number;
  cw: number;
  ch: number;
};

function clamp01(v: number) {
  return Math.min(1, Math.max(0, v));
}

function sameLocator(a: unknown, b: unknown) {
  return JSON.stringify(a ?? {}) === JSON.stringify(b ?? {});
}

/** Sticky notes bound to the source (plan §5). Markers float at stored
 * surface fractions so zoom, DPR and window resizes re-project
 * automatically; cards stair-step in a collapsible lane. Memos stay local —
 * they are never sent to AI search or prompts. */
export function NotesOverlay({
  projectId,
  tab,
}: {
  projectId: string;
  tab: ViewerTab;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const layerRef = useRef<HTMLDivElement>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [baseUpdatedAt, setBaseUpdatedAt] = useState<string | null>(null);
  const [conflictId, setConflictId] = useState<string | null>(null);
  const [laneOpen, setLaneOpen] = useState(true);
  const [savingId, setSavingId] = useState<string | null>(null);
  const [saveErrorId, setSaveErrorId] = useState<string | null>(null);
  const [pendingBody, setPendingBody] = useState<{ id: string; body: string } | null>(null);
  const debounce = useRef<ReturnType<typeof setTimeout>>(undefined);
  // Last assigned band colour: consecutive notes spread across the palette
  // instead of colliding on one colour.
  const lastColor = useRef<number>(-1);
  // Latest draft for the unmount/switch flush below (closures would go stale
  // mid-typing and drop keystrokes).
  const latest = useRef<{ id: string | null; body: string }>({ id: null, body: "" });
  useEffect(() => {
    latest.current = { id: editingId, body: pendingBody?.id === editingId ? pendingBody.body : draft };
  }, [editingId, draft, pendingBody]);
  // A new source never inherits the previous note's editor state (the
  // switch flush above saves first via effect cleanup order).
  useEffect(() => {
    setEditingId(null);
    setSelectedId(null);
    setDraft("");
    setBaseUpdatedAt(null);
    setConflictId(null);
    setSaveErrorId(null);
    setPendingBody(null);
  }, [projectId, tab.sourceId]);

  const notesQuery = useQuery({
    queryKey: ["notes", projectId, tab.sourceId],
    enabled: inTauri,
    queryFn: () => notesApi.list(projectId, tab.sourceId),
  });
  const notes = useMemo(() => notesQuery.data ?? [], [notesQuery.data]);
  const page = currentPageOf(tab.locator);

  // Material scroll tracking: markers are stored in document fractions and
  // re-projected on every scroll/resize, so they travel with the document.
  // Without a tagged scroller (e.g. website iframe) viewport fractions apply.
  const [metrics, setMetrics] = useState<ScrollMetrics | null>(null);
  useEffect(() => {
    const stack = layerRef.current?.parentElement;
    const scroller = stack?.querySelector("[data-note-scroll]") as HTMLElement | null;
    if (!stack || !scroller) {
      setMetrics(null);
      return;
    }
    let raf = 0;
    const update = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        setMetrics({
          sl: scroller.scrollLeft,
          st: scroller.scrollTop,
          sw: scroller.scrollWidth,
          sh: scroller.scrollHeight,
          cw: scroller.clientWidth,
          ch: scroller.clientHeight,
        });
      });
    };
    update();
    scroller.addEventListener("scroll", update, { passive: true });
    const ro = typeof ResizeObserver !== "undefined" ? new ResizeObserver(update) : null;
    ro?.observe(scroller);
    return () => {
      cancelAnimationFrame(raf);
      scroller.removeEventListener("scroll", update);
      ro?.disconnect();
    };
  }, [projectId, tab.sourceId, tab.locator]);

  // Responsive buckets from the stack width: dots, cards and controls scale
  // with narrow / standard / wide windows.
  const [sizeBucket, setSizeBucket] = useState<"sm" | "md" | "lg">("md");
  useEffect(() => {
    const stack = layerRef.current?.parentElement;
    if (!stack || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? 0;
      setSizeBucket(w < 480 ? "sm" : w < 900 ? "md" : "lg");
    });
    ro.observe(stack);
    return () => ro.disconnect();
  }, []);

  const invalidate = useCallback(() => {
    void qc.invalidateQueries({ queryKey: ["notes", projectId, tab.sourceId] });
  }, [qc, projectId, tab.sourceId]);

  const createMut = useMutation({
    mutationFn: (input: { x: number; y: number }) => {
      // Random band colour, never repeating the previous one twice in a row.
      let pick = Math.floor(Math.random() * NOTE_COLORS.length);
      if (pick === lastColor.current) {
        pick = (pick + 1) % NOTE_COLORS.length;
      }
      lastColor.current = pick;
      const color = NOTE_COLORS[pick] ?? "yellow";
      return notesApi.create({
        projectId,
        sourceId: tab.sourceId,
        locator: (tab.locator ?? {}) as unknown as null,
        anchorKind: "page",
        anchorJson: { page: page ?? 1, x: input.x, y: input.y } as unknown as null,
        body: "",
        color,
      });
    },
    onSuccess: (note) => {
      invalidate();
      setEditingId(note.id);
      setDraft("");
      setBaseUpdatedAt(note.updatedAt);
    },
    onError: (e) => {
      toast.push({
        tone: "error",
        message: e instanceof IpcError ? t([`errors.${e.code}`, "errors.internal"]) : t("errors.internal"),
      });
    },
  });

  const updateMut = useMutation({
    mutationFn: (input: { id: string; body: string; expected: string | null }) =>
      notesApi.update({
        projectId,
        noteId: input.id,
        locator: null,
        anchorJson: null,
        body: input.body,
        color: null,
        stackOrder: null,
        expectedUpdatedAt: input.expected,
      }),
    onMutate: (input) => setSavingId(input.id),
    onSuccess: (note) => {
      setSavingId(null);
      setSaveErrorId(null);
      if (conflictId === note.id) setConflictId(null);
      setBaseUpdatedAt(note.updatedAt);
      if (pendingBody?.id !== note.id) invalidate();
    },
    onError: (e, input) => {
      setSavingId(null);
      if (e instanceof IpcError && e.code === "NOTE_CONFLICT") {
        setConflictId(input.id);
      } else {
        setSaveErrorId(input.id);
      }
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => notesApi.remove(projectId, id),
    onSuccess: (_note, id) => {
      if (editingId === id) {
        setEditingId(null);
        setDraft("");
      }
      if (selectedId === id) setSelectedId(null);
      invalidate();
      toast.push({
        tone: "info",
        message: t("notes.deleted"),
        action: {
          label: t("notes.undo"),
          onClick: () => {
            void notesApi
              .restore(projectId, id)
              .then(() => invalidate())
              .catch(() =>
                toast.push({ tone: "error", message: t("errors.internal") }),
              );
          },
        },
      });
    },
    onError: () => toast.push({ tone: "error", message: t("errors.internal") }),
  });

  // Debounced autosave; flushed on blur, source switch and unmount.
  const flush = useCallback(() => {
    clearTimeout(debounce.current);
    if (editingId == null) {
      setPendingBody(null);
      return;
    }
    const body = (pendingBody?.id === editingId ? pendingBody.body : draft).slice(0, BODY_MAX);
    setPendingBody(null);
    updateMut.mutate({ id: editingId, body, expected: baseUpdatedAt });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingId, draft, pendingBody, baseUpdatedAt]);

  useEffect(() => {
    if (editingId == null) return;
    clearTimeout(debounce.current);
    debounce.current = setTimeout(flush, SAVE_DEBOUNCE_MS);
    return () => clearTimeout(debounce.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft]);

  // Switching sources or leaving the project flushes the draft first.
  useEffect(() => {
    return () => {
      clearTimeout(debounce.current);
      const { id, body } = latest.current;
      if (id) {
        void notesApi
          .update({
            projectId,
            noteId: id,
            locator: null,
            anchorJson: null,
            body: body.slice(0, BODY_MAX),
            color: null,
            stackOrder: null,
            expectedUpdatedAt: null,
          })
          .then(() => qc.invalidateQueries({ queryKey: ["notes", projectId] }))
          .catch(() => {});
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, tab.sourceId]);

  function beginEdit(note: Note) {
    flush();
    setEditingId(note.id);
    setDraft(note.body);
    setBaseUpdatedAt(note.updatedAt);
    setConflictId(null);
    setSaveErrorId(null);
  }

  /** End the edit and release the single-note focus (draft already saved). */
  function closeNote() {
    flush();
    setEditingId(null);
    setSelectedId(null);
    setDraft("");
    setPendingBody(null);
  }

  /** Open one note from the list or its dot: reveal the lane, hide the
   * rest, and start editing. */
  function openNote(note: Note) {
    setLaneOpen(true);
    setSelectedId(note.id);
    beginEdit(note);
  }

  /** Convert a viewport point into document fractions for storage. */
  function toDoc(clientX: number, clientY: number): { x: number; y: number } {
    const layer = layerRef.current;
    const scroller = layer?.parentElement?.querySelector(
      "[data-note-scroll]",
    ) as HTMLElement | null;
    if (layer && scroller && scroller.scrollWidth > 0 && scroller.scrollHeight > 0) {
      const r = scroller.getBoundingClientRect();
      return {
        x: clamp01((clientX - r.left + scroller.scrollLeft) / scroller.scrollWidth),
        y: clamp01((clientY - r.top + scroller.scrollTop) / scroller.scrollHeight),
      };
    }
    const box = layer?.getBoundingClientRect();
    if (!box || box.width <= 0 || box.height <= 0) return { x: 0.5, y: 0.5 };
    return {
      x: clamp01((clientX - box.left) / box.width),
      y: clamp01((clientY - box.top) / box.height),
    };
  }

  /** Project stored document fractions back into viewport percentages.
   * Returns null while scrolled out of view (reappears on scroll-back). */
  function toViewport(a: Anchor): { left: number; top: number } | null {
    const x = typeof a.x === "number" && Number.isFinite(a.x) ? clamp01(a.x) : 0.5;
    const y = typeof a.y === "number" && Number.isFinite(a.y) ? clamp01(a.y) : 0.08;
    if (!metrics || metrics.sw <= 0 || metrics.sh <= 0 || metrics.cw <= 0 || metrics.ch <= 0) {
      return { left: x * 100, top: y * 100 };
    }
    const left = ((x * metrics.sw - metrics.sl) / metrics.cw) * 100;
    const top = ((y * metrics.sh - metrics.st) / metrics.ch) * 100;
    if (left < -5 || left > 105 || top < -5 || top > 105) return null;
    return { left, top };
  }

  // The overlay layer itself is pointer-transparent, so creation listens on
  // the wrapping stack (the layer's parent: an ancestor of both the material
  // and the overlay). Overlay-owned targets carry `data-note-ui` and never
  // create: marker right-clicks stay delete-only even though the native
  // parent listener runs before React's own contextmenu handlers.
  useEffect(() => {
    const parent = layerRef.current?.parentElement;
    if (!parent) return;
    function onMenu(e: MouseEvent) {
      // Create only on the material itself: never on toolbars, page buttons,
      // links, form controls, note chrome, or while the reader has text
      // selected (the native selection menu keeps priority there). The click
      // must land inside the tagged material scroller — and on real content,
      // not the bare viewport (margins, letterboxing, toolbar gaps), whose
      // clamped coordinates would never match the click point.
      const target = e.target as HTMLElement;
      if (
        target.closest(
          "[data-note-ui], button, input, textarea, select, a, [role='toolbar'], [role='menu'], [role='dialog']",
        )
      ) {
        return;
      }
      const sel = window.getSelection();
      if (sel && !sel.isCollapsed) {
        return;
      }
      const scroller = parent?.querySelector("[data-note-scroll]") as HTMLElement | null;
      if (scroller) {
        if (!scroller.contains(target) || target === scroller) return;
      }
      e.preventDefault();
      e.stopPropagation();
      const { x, y } = toDoc(e.clientX, e.clientY);
      createMut.mutate({ x, y });
    }
    parent.addEventListener("contextmenu", onMenu);
    return () => parent.removeEventListener("contextmenu", onMenu);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, tab.sourceId, page]);

  // An open note closes on any click outside it — except material scrolling,
  // which fires no click (wheel, scrollbar drag and touch scrolls keep the
  // note open). Overlay chrome manages itself and is skipped here.
  useEffect(() => {
    if (editingId == null) return;
    function onClick(e: MouseEvent) {
      const target = e.target as HTMLElement | null;
      if (target && target.closest("[data-note-ui]")) return;
      const card = document.querySelector(`[data-note-card="${editingId}"]`);
      if (card && target && !card.contains(target)) {
        closeNote();
      }
    }
    document.addEventListener("click", onClick);
    return () => document.removeEventListener("click", onClick);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingId]);

  // Shift+F10 / Menu key on focused material → note at the current position.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const wantsNote =
        (e.shiftKey && e.key === "F10") || e.key === "ContextMenu";
      if (!wantsNote) return;
      const scope = layerRef.current?.parentElement;
      if (!scope || !scope.contains(document.activeElement)) return;
      const target = document.activeElement as HTMLElement | null;
      if (target && target.closest("input, textarea, select, [role='dialog']")) return;
      e.preventDefault();
      createMut.mutate({ x: 0.5, y: 0.5 });
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, tab.sourceId, page]);

  function jumpToPage(target: number) {
    const locator = { t: "page", page: target };
    rememberTabLocator(qc, projectId, tab.id, locator);
    void viewerApi
      .updateLocator(projectId, tab.id, locator)
      .then(() => qc.invalidateQueries({ queryKey: ["viewer-tabs", projectId] }))
      .catch(() => toast.push({ tone: "error", message: t("errors.internal") }));
  }

  const visible = notes.filter((n) => {
    const a = asAnchor(n);
    if (typeof a.page !== "number") return true;
    if (page == null) return true;
    return a.page === page;
  });
  // Single-note focus: tapping one card (or its dot) hides the rest and
  // opens it for editing; the list button returns to the side-by-side view.
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const listed = selectedId ? visible.filter((n) => n.id === selectedId) : visible;
  // Moving to another page/locator releases the focus; a pending autosave
  // still flushes through the shared draft timer.
  useEffect(() => {
    setSelectedId(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, tab.sourceId, tab.locator]);
  const hiddenByPage = notes.filter((n) => !visible.includes(n));
  const hiddenCounts = useMemo(() => {
    const map = new Map<number, number>();
    for (const n of hiddenByPage) {
      const p = asAnchor(n).page;
      if (typeof p === "number") map.set(p, (map.get(p) ?? 0) + 1);
    }
    return [...map.entries()].sort((a, b) => a[0] - b[0]);
  }, [hiddenByPage]);

  return (
    <div ref={layerRef} className={styles.layer} data-size={sizeBucket}>
      {visible.map((n) => {
        const pos = toViewport(asAnchor(n));
        if (!pos) return null;
        return (
          <button
            key={n.id}
            type="button"
            className={styles.marker}
            data-note-ui="marker"
            data-color={n.color}
            data-editing={editingId === n.id || undefined}
            style={{ left: `${pos.left}%`, top: `${pos.top}%` }}
            aria-label={t("notes.markerLabel", { color: t(`notes.color_${n.color}`) })}
            onClick={() => openNote(n)}
            onContextMenu={(e) => {
              e.preventDefault();
              e.stopPropagation();
              deleteMut.mutate(n.id);
            }}
          />
        );
      })}

      {notes.length > 0 ? (
      <div className={styles.lane} data-note-ui="lane" data-open={laneOpen || undefined}>
        <div className={styles.laneHead}>
          <button
            type="button"
            className={styles.laneToggle}
            aria-expanded={laneOpen}
            onClick={() => {
              if (laneOpen) closeNote();
              setLaneOpen((v) => !v);
            }}
          >
            {laneOpen ? t("notes.hideLane") : t("notes.showLane", { count: visible.length })}
          </button>
          {selectedId ? (
            <button
              type="button"
              className={styles.laneToggle}
              onClick={() => setSelectedId(null)}
            >
              {t("notes.showAll")}
            </button>
          ) : null}
          {savingId ? <span className={styles.status}>{t("notes.saving")}</span> : null}
        </div>
        {laneOpen ? (
          <ol className={styles.cards}>
            {listed.map((n, i) => {
              const needsCheck =
                n.anchorKind === "text" && !sameLocator(n.locator, tab.locator);
              const editing = editingId === n.id;
              return (
                <li
                  key={n.id}
                  className={styles.card}
                  data-note-card={n.id}
                  data-color={n.color}
                  style={{ marginTop: `${Math.min(i, 8) * 14}px` }}
                >
                  {needsCheck ? (
                    <span className={styles.needsCheck}>{t("notes.needsCheck")}</span>
                  ) : null}
                  {editing ? (
                    <textarea
                      autoFocus
                      className={styles.editor}
                      value={draft}
                      maxLength={BODY_MAX}
                      rows={3}
                      aria-label={t("notes.editLabel")}
                      onChange={(e) => {
                        setDraft(e.target.value);
                        setPendingBody({ id: n.id, body: e.target.value });
                      }}
                      onBlur={flush}
                      onKeyDown={(e) => {
                        if (e.key === "Escape") {
                          e.stopPropagation();
                          closeNote();
                        }
                      }}
                    />
                  ) : (
                    <button
                      type="button"
                      className={styles.cardOpen}
                      aria-label={t("notes.editLabel")}
                      onClick={() => openNote(n)}
                    >
                      <span className={styles.body}>{n.body || t("notes.emptyHint")}</span>
                    </button>
                  )}
                  <span className={styles.cardFoot}>
                    <span className={styles.count}>
                      {t("notes.chars", { count: (editing ? draft : n.body).length, max: BODY_MAX })}
                    </span>
                    {conflictId === n.id ? (
                      <span className={styles.conflict} role="alert">
                        {t("notes.conflict")}
                      </span>
                    ) : null}
                    {saveErrorId === n.id ? (
                      <button
                        type="button"
                        className={styles.retry}
                        onClick={() =>
                          updateMut.mutate({ id: n.id, body: draft.slice(0, BODY_MAX), expected: null })
                        }
                      >
                        {t("notes.retry")}
                      </button>
                    ) : null}
                  </span>
                </li>
              );
            })}
          </ol>
        ) : null}
        {hiddenCounts.length ? (
          <ul className={styles.otherPages}>
            {hiddenCounts.map(([p, c]) => (
              <li key={p}>
                <button type="button" className={styles.jump} onClick={() => jumpToPage(p)}>
                  {t("notes.otherPage", { count: c, page: p })}
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </div>
      ) : null}
    </div>
  );
}

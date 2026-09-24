import { create } from "zustand";

export type ThemePref = "light" | "dark" | "system";
export type Scale = 80 | 90 | 100 | 110 | 125 | 150;
export type ReadingFont = "serif" | "sans";

type UiState = {
  sidebarOpen: boolean;
  theme: ThemePref;
  scale: Scale;
  monochrome: boolean;
  readingFont: ReadingFont;
  /** Live Illustrator global toggle (FR-L1). Backed by app settings in Phase 9. */
  illustratorEnabled: boolean;
  /** Document rendering adjustments (PDF/DOCX/PPTX). Session-only: they
   * describe how the current preview looks, not a saved preference. */
  docInverted: boolean;
  docClarity: number;
  openSidebar: () => void;
  closeSidebar: () => void;
  toggleSidebar: () => void;
  setTheme: (t: ThemePref) => void;
  setScale: (s: Scale) => void;
  setMonochrome: (v: boolean) => void;
  setReadingFont: (f: ReadingFont) => void;
  setIllustratorEnabled: (v: boolean) => void;
  setDocInverted: (v: boolean) => void;
  setDocClarity: (v: number) => void;
};

const KEY = "wakaru.ui";

type Persisted = Pick<
  UiState,
  "theme" | "scale" | "monochrome" | "readingFont" | "illustratorEnabled"
>;

function load(): Persisted {
  const fallback: Persisted = {
    theme: "system",
    scale: 100,
    monochrome: false,
    readingFont: "serif",
    illustratorEnabled: false,
  };
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return fallback;
    return { ...fallback, ...(JSON.parse(raw) as Partial<Persisted>) };
  } catch {
    return fallback;
  }
}

function save(s: Persisted) {
  try {
    localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    /* private mode / disabled storage — ignore */
  }
}

const initial = load();

export const useUiStore = create<UiState>((set, get) => ({
  sidebarOpen: false,
  docInverted: false,
  docClarity: 0,
  ...initial,
  openSidebar: () => set({ sidebarOpen: true }),
  closeSidebar: () => set({ sidebarOpen: false }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
  setTheme: (theme) => {
    set({ theme });
    persist(get);
  },
  setScale: (scale) => {
    set({ scale });
    persist(get);
  },
  setMonochrome: (monochrome) => {
    set({ monochrome });
    persist(get);
  },
  setReadingFont: (readingFont) => {
    set({ readingFont });
    persist(get);
  },
  setIllustratorEnabled: (illustratorEnabled) => {
    set({ illustratorEnabled });
    persist(get);
  },
  setDocInverted: (docInverted) => set({ docInverted }),
  setDocClarity: (docClarity) =>
    set({ docClarity: Math.min(100, Math.max(0, docClarity)) }),
}));

function persist(get: () => UiState) {
  const { theme, scale, monochrome, readingFont, illustratorEnabled } = get();
  save({ theme, scale, monochrome, readingFont, illustratorEnabled });
}

/** Resolve `system` against the OS preference. */
export function resolveTheme(pref: ThemePref): "light" | "dark" {
  if (pref !== "system") return pref;
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/** Apply the current UI prefs to <html>. Called from a subscription in main.tsx. */
export function applyUiToDocument(s: Pick<UiState, "theme" | "scale" | "monochrome" | "readingFont">) {
  const root = document.documentElement;
  root.dataset.theme = resolveTheme(s.theme);
  root.dataset.scale = String(s.scale);
  root.dataset.monochrome = String(s.monochrome);
  root.dataset.readingFont = s.readingFont;
}

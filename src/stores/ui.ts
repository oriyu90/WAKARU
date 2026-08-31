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
  openSidebar: () => void;
  closeSidebar: () => void;
  toggleSidebar: () => void;
  setTheme: (t: ThemePref) => void;
  setScale: (s: Scale) => void;
  setMonochrome: (v: boolean) => void;
  setReadingFont: (f: ReadingFont) => void;
};

const KEY = "wakaru.ui";

type Persisted = Pick<UiState, "theme" | "scale" | "monochrome" | "readingFont">;

function load(): Persisted {
  const fallback: Persisted = {
    theme: "system",
    scale: 100,
    monochrome: false,
    readingFont: "serif",
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
}));

function persist(get: () => UiState) {
  const { theme, scale, monochrome, readingFont } = get();
  save({ theme, scale, monochrome, readingFont });
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

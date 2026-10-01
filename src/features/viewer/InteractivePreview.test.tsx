import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { InteractivePreview } from "./InteractivePreview";
import type { VisualPreview } from "../../ipc/types.gen";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

vi.mock("../../components/useToast", () => ({
  useToast: () => ({ push: vi.fn(), dismiss: vi.fn() }),
}));

vi.mock("../../components/Button", () => ({
  Button: ({ children }: { children: React.ReactNode }) => <button>{children}</button>,
}));

const visual: VisualPreview = {
  id: "v1",
  schemaVersion: 1,
  title: "Test figure",
  html: "<svg></svg>",
  css: "",
  js: "window.__wakaruNotify({a:1});",
  dataJson: {},
  aspectRatio: "16:9",
  sourceRefs: [],
  initialState: {},
  model: "",
  createdAt: "2026-10-01T00:00:00Z",
};

describe("InteractivePreview isolation", () => {
  it("renders an opaque-origin sandbox frame with no IPC bridge", () => {
    render(<InteractivePreview visual={visual} />);
    const frame = document.querySelector("iframe");
    expect(frame).not.toBeNull();
    // allow-scripts only: no same-origin, no forms, no popups.
    expect(frame?.getAttribute("sandbox")).toBe("allow-scripts");
    expect(frame?.getAttribute("referrerpolicy")).toBe("no-referrer");
    const doc = frame?.getAttribute("srcdoc") ?? "";
    expect(doc).toContain("Content-Security-Policy");
    expect(doc).toContain("connect-src 'none'");
    expect(doc).not.toContain("allow-same-origin");
    // No IPC bridge or asset URL is handed to figure code.
    expect(doc).not.toContain("__TAURI");
    expect(doc).not.toContain("wakaru-asset");
    expect(doc).not.toContain("ipc:");
  });

  it("shows shared chrome: retry on failure path labels exist", () => {
    render(<InteractivePreview visual={visual} />);
    expect(screen.getByLabelText("Test figure")).toBeInTheDocument();
  });
});

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SourceDetail } from "../../ipc/types.gen";
import "../../i18n";
import { PdfFilePreview } from "./FilePreviews";
import { ImagePreview } from "./Preview";

vi.mock("../../components/useToast", () => ({
  useToast: () => ({ push: vi.fn(), dismiss: vi.fn() }),
}));

vi.mock("pdfjs-dist", () => ({
  GlobalWorkerOptions: {},
  getDocument: vi.fn(() => ({
    promise: Promise.resolve({
      numPages: 3,
      getPage: async () => ({
        getViewport: ({ scale = 1 }: { scale?: number }) => ({
          width: 100 * scale,
          height: 140 * scale,
        }),
        render: () => ({ promise: Promise.resolve() }),
        getTextContent: async () => ({ items: [] }),
      }),
    }),
    destroy: vi.fn(),
  })),
}));

vi.mock("pdfjs-dist/build/pdf.worker.mjs?url", () => ({ default: "worker.js" }));

const pdfDetail: SourceDetail = {
  id: "source-pdf",
  kind: "pdf",
  name: "a.pdf",
  url: null,
  pageCount: 3,
  status: "ready",
  mime: "application/pdf",
  bytes: 4n,
  primaryAssetUrl: "wakaru-asset://localhost/project/source/a.pdf",
  ocrStatus: null,
};

function pdfSetup() {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response(new Uint8Array([1, 2, 3, 4]))),
  );
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <PdfFilePreview
        detail={pdfDetail}
        projectId="project"
        initialPage={1}
        onPage={() => {}}
        fallback={<span>fallback</span>}
      />
    </QueryClientProvider>,
  );
}

describe("PdfFilePreview zoom modes", () => {
  it("offers fit and fill modes defaulting to fit", async () => {
    pdfSetup();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /全体を表示|Fit whole|显示整页/ })).toBeInTheDocument(),
    );
    const fit = screen.getByRole("button", { name: /全体を表示|Fit whole|显示整页/ });
    const fill = screen.getByRole("button", { name: /画面いっぱい|Fill screen|填满屏幕/ });
    expect(fit.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(fill);
    expect(fill.getAttribute("aria-pressed")).toBe("true");
  });
});

describe("ImagePreview zoom modes", () => {
  it("toggles mode and zooms with ctrl+wheel without scrolling", () => {
    render(<ImagePreview url="wakaru-asset://localhost/project/source/a.png" />);
    const fill = screen.getByRole("button", { name: /画面いっぱい|Fill screen|填满屏幕/ });
    fireEvent.click(fill);
    expect(fill.getAttribute("aria-pressed")).toBe("true");
    const viewport = document.querySelector("[aria-label='画像プレビュー'], [aria-label='Image preview'], [aria-label='图像预览']");
    expect(viewport).not.toBeNull();
    expect(screen.getByRole("button", { name: "100%" })).toBeInTheDocument();
    fireEvent.wheel(viewport!, { ctrlKey: true, deltaY: -100 });
    expect(screen.queryByRole("button", { name: "100%" })).toBeNull();
  });
});

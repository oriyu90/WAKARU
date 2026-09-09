import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SourceDetail } from "../../ipc/types.gen";
import "../../i18n";
import { PptxFilePreview } from "./FilePreviews";

const renderSlideToElement = vi.fn(
  (_presentation: unknown, _index: number, container: HTMLElement) => {
    container.appendChild(document.createElementNS("http://www.w3.org/2000/svg", "svg"));
  },
);

vi.mock("pptx-viewer", () => ({
  loadPresentation: vi.fn(async () => ({
    slides: [{}, {}],
    cleanup: vi.fn(),
  })),
  renderSlideToElement,
}));

const detail: SourceDetail = {
  id: "source-1",
  kind: "slides",
  name: "deck.pptx",
  url: null,
  pageCount: 2,
  status: "ready",
  mime: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  bytes: 4n,
  primaryAssetUrl: "wakaru-asset://localhost/project/source/deck.pptx",
  ocrStatus: null,
};

describe("PptxFilePreview", () => {
  beforeEach(() => {
    renderSlideToElement.mockClear();
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(new Uint8Array([1, 2, 3, 4]))),
    );
  });

  it("renders after loading even when parsed and ingested slide counts are equal", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const onPage = vi.fn();
    const { container } = render(
      <QueryClientProvider client={client}>
        <PptxFilePreview
          detail={detail}
          initialPage={1}
          onPage={onPage}
          fallback={<span>fallback</span>}
        />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(renderSlideToElement).toHaveBeenCalledTimes(1));
    expect(container.querySelector("svg")).not.toBeNull();
    expect(onPage).toHaveBeenCalledWith(1, 2);
  });
});

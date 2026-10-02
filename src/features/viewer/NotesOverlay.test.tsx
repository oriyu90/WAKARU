import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { NotesOverlay } from "./NotesOverlay";
import { notesApi } from "../../ipc/notes";
import type { ViewerTab } from "../../ipc/types.gen";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (k: string, o?: Record<string, unknown>) =>
      o ? `${k} ${JSON.stringify(o)}` : k,
  }),
}));

vi.mock("../../components/useToast", () => ({
  useToast: () => ({ push: vi.fn(), dismiss: vi.fn() }),
}));

vi.mock("../../ipc/client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../ipc/client")>();
  return { ...actual, inTauri: true };
});

vi.mock("../../ipc/viewer", () => ({
  viewerApi: { updateLocator: vi.fn(async () => {}) },
  documentApi: {},
}));

vi.mock("./Preview", () => ({
  rememberTabLocator: vi.fn(),
}));

const tab = {
  id: "tab1",
  sourceId: "s1",
  kind: "pdf",
  name: "a.pdf",
  locator: { t: "page", page: 3 },
  pinned: false,
  ordinal: 1,
} as unknown as ViewerTab;

function note(id: string, page: number, body: string, x = 0.25, y = 0.5) {
  return {
    id,
    sourceId: "s1",
    locator: { t: "page", page },
    anchorKind: "page",
    anchorJson: { page, x, y },
    body,
    color: "yellow",
    stackOrder: 0,
    createdAt: "2026-10-01T00:00:00Z",
    updatedAt: "2026-10-01T00:00:00Z",
    deletedAt: null,
  };
}

describe("NotesOverlay", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function setup(list: ReturnType<typeof note>[]) {
    vi.spyOn(notesApi, "list").mockResolvedValue(list as never);
    // jsdom reports zero rects: give the overlay a measurable box.
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 800,
      bottom: 600,
      width: 800,
      height: 600,
      toJSON: () => "",
    });
    // jsdom reports zero scroll sizes: emulate a scrolled document.
    const scrollSizes: Record<string, number> = {
      scrollWidth: 800,
      scrollHeight: 2000,
      clientWidth: 800,
      clientHeight: 600,
      scrollLeft: 0,
      scrollTop: 400,
    };
    for (const [prop, value] of Object.entries(scrollSizes)) {
      vi.spyOn(HTMLElement.prototype, prop as "scrollWidth", "get").mockReturnValue(value);
    }
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <div data-testid="stack" style={{ position: "relative" }}>
          <div data-testid="material" data-note-scroll>
            <p data-testid="content">material</p>
          </div>
          <NotesOverlay projectId="p1" tab={tab} />
        </div>
      </QueryClientProvider>,
    );
    return qc;
  }

  it("shows current-page markers and cards, other pages as jump counts", async () => {
    setup([note("n1", 3, "hello"), note("n2", 5, "elsewhere")]);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    // Current-page marker present with a colour label, not colour alone.
    expect(screen.getByLabelText(/markerLabel/)).toBeInTheDocument();
    // Other page collapses to a count + jump entry.
    expect(screen.getByText(/otherPage/)).toBeInTheDocument();
    expect(screen.queryByText("elsewhere")).toBeNull();
  });

  it("right-click on content creates a note; on bare viewport or marker it does not", async () => {
    const create = vi.spyOn(notesApi, "create").mockResolvedValue(note("n9", 3, "") as never);
    setup([note("n1", 3, "hello")]);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    // Bare viewport (margins, letterboxing) never creates: its clamped
    // coordinates could never match the click point.
    fireEvent.contextMenu(screen.getByTestId("material"));
    expect(create).not.toHaveBeenCalled();
    // Real content (bubbles to the stack listener) creates.
    fireEvent.contextMenu(screen.getByTestId("content"));
    await waitFor(() => expect(create).toHaveBeenCalledTimes(1));
    // The band colour is one of the seven validated colours.
    const color = (create.mock.calls[0]?.[0] as { color?: unknown } | undefined)?.color;
    expect(["yellow", "pink", "blue", "green", "orange", "purple", "teal"]).toContain(color);
    // Marker right-click is overlay chrome: no creation, only delete.
    const remove = vi.spyOn(notesApi, "remove").mockResolvedValue(note("n1", 3, "hello") as never);
    fireEvent.contextMenu(screen.getByLabelText(/markerLabel/));
    await waitFor(() => expect(remove).toHaveBeenCalled());
    expect(create).toHaveBeenCalledTimes(1);
  });

  it("cards offer no delete button; deletion stays on the marker", async () => {
    setup([note("n1", 3, "hello")]);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: /delete/i })).toBeNull();
  });

  it("tapping a card hides the rest and edits it; the list button restores all", async () => {
    const update = vi.spyOn(notesApi, "update").mockResolvedValue(note("n1", 3, "hello") as never);
    setup([note("n1", 3, "first"), note("n2", 3, "second")]);
    await waitFor(() => expect(screen.getByText("first")).toBeInTheDocument());
    expect(screen.getByText("second")).toBeInTheDocument();
    // Tap the first card: the other one disappears and the editor opens.
    fireEvent.click(screen.getByText("first"));
    await waitFor(() => expect(screen.queryByText("second")).toBeNull());
    expect(document.querySelector("textarea")).not.toBeNull();
    // The list button brings every card back.
    fireEvent.click(screen.getByText(/showAll/));
    await waitFor(() => expect(screen.getByText("second")).toBeInTheDocument());
    // Opening a card schedules its (unchanged) autosave flush.
    await waitFor(() => expect(update).toHaveBeenCalled());
  });

  it("tapping a dot opens its note even when the lane is closed", async () => {
    setup([note("n1", 3, "hello"), note("n2", 3, "other")]);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    // Open a note, then click outside: the note and the lane both close.
    fireEvent.click(screen.getByText("hello"));
    await waitFor(() => expect(document.querySelector("textarea")).not.toBeNull());
    fireEvent.click(document.body);
    await waitFor(() => expect(document.querySelector("[data-note-ui='lane']")).toBeNull());
    // Tap the dot: the lane reopens on that note.
    const marker = screen.getAllByLabelText(/markerLabel/)[0];
    if (!marker) throw new Error("no marker");
    fireEvent.click(marker);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    expect(screen.queryByText("other")).toBeNull();
  });

  it("projects document fractions into the layer box, not the scroller box", async () => {
    setup([note("n1", 3, "hello", 0.5, 0.5)]);
    // Scroller sits 100px below the stack top (toolbar strip): a document
    // midpoint must land mid-layer, not mid-scroller.
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (
      this: HTMLElement,
    ) {
      const box = {
        x: 0, y: 0, left: 0, top: 0, right: 800, bottom: 600,
        width: 800, height: 600, toJSON: () => "",
      };
      if (this.dataset.testid === "stack") return { ...box, bottom: 800, height: 800 };
      if (this.dataset.testid === "material") return { ...box, top: 100, height: 600 };
      return box;
    });
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    const marker = screen.getByLabelText(/markerLabel/);
    // Document (0.5, 0.5) of an 800x2000 scroller scrolled to 400 → layer
    // point (400, 100 + 1000 - 400 = 700) → left 50%, top 87.5%.
    await waitFor(() => expect(marker.style.left).toBe("50%"));
    // A scroll refreshes the metrics (resize/content observers in browsers).
    fireEvent.scroll(screen.getByTestId("material"));
    await waitFor(() => expect(marker.style.top).toBe("87.5%"));
  });

  it("flags pre-1.6.4 notes whose stored coordinates used the old meaning", async () => {
    setup([
      { ...note("n1", 3, "legacy"), createdAt: "2026-10-01T00:00:00Z", updatedAt: "2026-10-01T00:00:00Z" },
      { ...note("n2", 3, "fresh"), createdAt: "2026-10-02T01:00:00Z", updatedAt: "2026-10-02T01:00:00Z" },
    ]);
    await waitFor(() => expect(screen.getByText("legacy")).toBeInTheDocument());
    // Exactly one position-check badge: the legacy note. The fresh note,
    // stored as document fractions, renders without one.
    await waitFor(() => expect(screen.getAllByText(/needsCheck/).length).toBe(1));
  });

  it("hides the lane entirely when there are no notes", async () => {
    setup([]);
    await waitFor(() => expect(notesListed()).toBe(true));
    expect(screen.queryByText(/showAll/)).toBeNull();
  });

  it("clicking outside an open note closes it and the lane", async () => {
    setup([note("n1", 3, "first"), note("n2", 3, "second")]);
    await waitFor(() => expect(screen.getByText("first")).toBeInTheDocument());
    fireEvent.click(screen.getByText("first"));
    await waitFor(() => expect(document.querySelector("textarea")).not.toBeNull());
    expect(screen.queryByText("second")).toBeNull();
    // A click anywhere outside the card ends the edit and closes the lane.
    // Scrolling fires no click, so it keeps the note open.
    fireEvent.click(document.body);
    await waitFor(() => expect(document.querySelector("textarea")).toBeNull());
    await waitFor(() => expect(document.querySelector("[data-note-ui='lane']")).toBeNull());
  });

  it("dragging a dot moves the note without opening it", async () => {
    const update = vi.spyOn(notesApi, "update").mockResolvedValue(note("n1", 3, "hello") as never);
    setup([note("n1", 3, "hello")]);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    const marker = screen.getByLabelText(/markerLabel/);
    // jsdom has no PointerEvent: drive the pointer handlers with real
    // MouseEvents of the same type names (production browsers supply the
    // real PointerEvents). Press, move past the drag threshold, release.
    const down = new MouseEvent("pointerdown", { button: 0, clientX: 100, clientY: 100, bubbles: true });
    marker.dispatchEvent(down);
    document.body.dispatchEvent(
      new MouseEvent("pointermove", { clientX: 200, clientY: 300, bubbles: true }),
    );
    document.body.dispatchEvent(
      new MouseEvent("pointerup", { clientX: 200, clientY: 300, bubbles: true }),
    );
    await waitFor(() => expect(update).toHaveBeenCalledTimes(1));
    const anchor = (update.mock.calls[0]?.[0] as { anchorJson?: unknown } | undefined)?.anchorJson as {
      x?: number;
      y?: number;
    };
    // Scroller is 800x2000 at scroll 400: (200, 300) lands at (0.25, 0.35).
    expect(anchor?.x).toBeCloseTo(0.25);
    expect(anchor?.y).toBeCloseTo(0.35);
    // The suppressed tap opens nothing.
    fireEvent.click(marker);
    expect(document.querySelector("textarea")).toBeNull();
  });

  function notesListed() {
    return document.querySelector("[data-note-ui='lane']") == null;
  }

  it("overlay chrome is tagged so material right-clicks never hit it", async () => {
    setup([note("n1", 3, "hello")]);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    // Markers and the lane carry the guard attribute the stack-level
    // contextmenu listener checks before creating a note.
    expect(document.querySelector("[data-note-ui='lane']")).not.toBeNull();
    expect(document.querySelector("[data-note-ui='marker']")).not.toBeNull();
  });
});

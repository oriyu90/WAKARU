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
    // Close the lane, then tap the dot: the lane reopens on that note.
    fireEvent.click(screen.getByText(/hideLane/));
    await waitFor(() => expect(screen.queryByText("hello")).toBeNull());
    const marker = screen.getAllByLabelText(/markerLabel/)[0];
    if (!marker) throw new Error("no marker");
    fireEvent.click(marker);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    expect(screen.queryByText("other")).toBeNull();
  });

  it("hides the lane entirely when there are no notes", async () => {
    setup([]);
    await waitFor(() => expect(notesListed()).toBe(true));
    expect(screen.queryByText(/hideLane|showLane/)).toBeNull();
  });

  it("clicking outside an open note closes it, scrolling does not", async () => {
    setup([note("n1", 3, "first"), note("n2", 3, "second")]);
    await waitFor(() => expect(screen.getByText("first")).toBeInTheDocument());
    fireEvent.click(screen.getByText("first"));
    await waitFor(() => expect(document.querySelector("textarea")).not.toBeNull());
    expect(screen.queryByText("second")).toBeNull();
    // A click anywhere outside the card ends the edit and restores the list.
    fireEvent.click(document.body);
    await waitFor(() => expect(document.querySelector("textarea")).toBeNull());
    await waitFor(() => expect(screen.getByText("second")).toBeInTheDocument());
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

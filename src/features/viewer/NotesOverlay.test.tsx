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
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <div style={{ position: "relative" }}>
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

  it("right-click on a marker deletes without creating a new note", async () => {
    const remove = vi.spyOn(notesApi, "remove").mockResolvedValue(note("n1", 3, "hello") as never);
    const create = vi.spyOn(notesApi, "create").mockResolvedValue(note("n9", 3, "") as never);
    setup([note("n1", 3, "hello")]);
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    const marker = screen.getByLabelText(/markerLabel/);
    fireEvent.contextMenu(marker);
    await waitFor(() => expect(remove).toHaveBeenCalled());
    expect(create).not.toHaveBeenCalled();
  });
});

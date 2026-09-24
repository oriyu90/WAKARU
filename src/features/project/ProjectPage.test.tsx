import { render, screen } from "@testing-library/react";
import { RouterProvider, createMemoryRouter } from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, expect, test } from "vitest";
import { ToastProvider } from "../../components/Toast";
import { useUiStore } from "../../stores/ui";
import "../../i18n";
import { ProjectPage } from "./ProjectPage";

// jsdom has no layout; Viewer scrolls its rail on tab changes.
Element.prototype.scrollIntoView ??= () => {};

function renderPage() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const router = createMemoryRouter(
    [{ path: "/project/:projectId", element: <ProjectPage /> }],
    { initialEntries: ["/project/p1"] },
  );
  return render(
    <QueryClientProvider client={qc}>
      <ToastProvider>
        <RouterProvider router={router} />
      </ToastProvider>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  useUiStore.setState({ docInverted: false, docClarity: 0 });
});

test("pane bar shows only the pane tabs when no document is previewed", () => {
  renderPage();
  // Pane tabs are always there.
  expect(screen.getByRole("tab", { name: "Sources" })).toBeTruthy();
  expect(screen.getByRole("tab", { name: "Studio" })).toBeTruthy();
  // Without an open document preview, the adjustment controls stay hidden.
  expect(
    screen.queryByRole("button", { name: "Invert black & white" }),
  ).toBeNull();
  expect(
    screen.queryByRole("slider", { name: "Sharpness / contrast" }),
  ).toBeNull();
});

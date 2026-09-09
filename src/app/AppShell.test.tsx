import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider, createMemoryRouter } from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, expect, test } from "vitest";
import { AppShell } from "./AppShell";
import { ToastProvider } from "../components/Toast";
import { useUiStore } from "../stores/ui";
import "../i18n";

function renderShell(initialEntries: string[] = ["/"]) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const router = createMemoryRouter(
    [
      {
        path: "/",
        element: <AppShell />,
        children: [
          { index: true, element: <div>home</div> },
          { path: "settings", element: <div>settings-page</div> },
          { path: "search", element: <div>search-page</div> },
        ],
      },
    ],
    { initialEntries },
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
  useUiStore.setState({ sidebarOpen: false });
});

test("AC-0-2 · Cmd/Ctrl+B toggles the sidebar; it overlays (does not push)", async () => {
  const user = userEvent.setup();
  renderShell();
  const sidebar = screen.getByRole("navigation");
  expect(sidebar).toHaveAttribute("data-open", "false");

  await user.keyboard("{Control>}b{/Control}");
  expect(sidebar).toHaveAttribute("data-open", "true");
  expect(sidebar.nextElementSibling?.tagName).toBe("MAIN");

  await user.keyboard("{Control>}b{/Control}");
  expect(sidebar).toHaveAttribute("data-open", "false");
});

test("AC-0-3 · Escape closes the sidebar", async () => {
  const user = userEvent.setup();
  renderShell();
  const sidebar = screen.getByRole("navigation");

  await user.keyboard("{Control>}b{/Control}");
  expect(sidebar).toHaveAttribute("data-open", "true");

  await user.keyboard("{Escape}");
  expect(sidebar).toHaveAttribute("data-open", "false");
});

test("sidebar order is Home → File Modifier → … → Search (FR-N2)", () => {
  renderShell();
  const links = screen.getAllByRole("link").map((a) => a.getAttribute("href"));
  expect(links.filter((h) => h && h !== "/settings")).toEqual([
    "/",
    "/file-modifier",
    "/search",
  ]);
});

test("P3 · the settings button opens settings, then returns to the previous view", async () => {
  const user = userEvent.setup();
  renderShell(["/search"]);
  expect(screen.getByText("search-page")).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: /settings/i }));
  expect(screen.getByText("settings-page")).toBeInTheDocument();

  // Same control again — back to where we were, not a no-op.
  await user.click(screen.getByRole("button", { name: /close settings|設定を閉じる/i }));
  expect(screen.getByText("search-page")).toBeInTheDocument();
});

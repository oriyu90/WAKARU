import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider, createMemoryRouter } from "react-router";
import { beforeEach, expect, test } from "vitest";
import { AppShell } from "./AppShell";
import { useUiStore } from "../stores/ui";
import "../i18n";

function renderShell() {
  const router = createMemoryRouter(
    [{ path: "/", element: <AppShell />, children: [{ index: true, element: <div>home</div> }] }],
    { initialEntries: ["/"] },
  );
  return render(<RouterProvider router={router} />);
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
  // overlay, not a flow element: the sidebar and main are siblings, so opening
  // it cannot reflow the content (visual overlay verified manually / in E2E).
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

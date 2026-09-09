import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, test, vi } from "vitest";
import { ContextMenu } from "./ContextMenu";

function setup(onClose = vi.fn(), onExport = vi.fn(), onDelete = vi.fn()) {
  render(
    <ContextMenu
      x={20}
      y={20}
      label="Project"
      onClose={onClose}
      items={[
        { label: "Export", onSelect: onExport },
        { label: "Delete", danger: true, onSelect: onDelete },
      ]}
    />,
  );
  return { onClose, onExport, onDelete };
}

test("renders a labelled menu with its items", () => {
  setup();
  expect(screen.getByRole("menu", { name: "Project" })).toBeInTheDocument();
  expect(screen.getAllByRole("menuitem")).toHaveLength(2);
});

test("selecting an item closes the menu and runs its action", async () => {
  const user = userEvent.setup();
  const { onClose, onExport } = setup();
  await user.click(screen.getByRole("menuitem", { name: "Export" }));
  expect(onExport).toHaveBeenCalledOnce();
  expect(onClose).toHaveBeenCalledOnce();
});

test("Escape closes the menu", async () => {
  const user = userEvent.setup();
  const { onClose } = setup();
  await user.keyboard("{Escape}");
  expect(onClose).toHaveBeenCalled();
});

test("ArrowDown moves focus between items", async () => {
  const user = userEvent.setup();
  setup();
  const [first, second] = screen.getAllByRole("menuitem");
  expect(first).toHaveFocus();
  await user.keyboard("{ArrowDown}");
  expect(second).toHaveFocus();
});

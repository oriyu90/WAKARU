import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Button } from "./Button";
import { IconButton } from "./IconButton";
import { PlusIcon } from "../app/Icons";

test("loading state keeps the label and marks aria-busy + disabled", () => {
  render(<Button loading>Save</Button>);
  const btn = screen.getByRole("button", { name: /save/i });
  expect(btn).toHaveAttribute("aria-busy", "true");
  expect(btn).toBeDisabled();
});

test("disabled explains why via title", () => {
  render(
    <Button disabled disabledReason="Set up AI first">
      Generate
    </Button>,
  );
  expect(screen.getByRole("button")).toHaveAttribute("title", "Set up AI first");
});

test("icon-only button carries an accessible name", () => {
  render(
    <IconButton label="Add source">
      <PlusIcon />
    </IconButton>,
  );
  expect(screen.getByRole("button", { name: "Add source" })).toBeInTheDocument();
});

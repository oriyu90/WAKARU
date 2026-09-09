import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { expect, test } from "vitest";
import { Switch } from "./Switch";

function Controlled() {
  const [on, setOn] = useState(false);
  return (
    <Switch
      label="Enable Live Illustrator"
      checked={on}
      onChange={(e) => setOn(e.target.checked)}
    />
  );
}

test("switch exposes an accessible name and the switch role", () => {
  render(<Switch label="Enable Live Illustrator" checked={false} readOnly />);
  expect(
    screen.getByRole("switch", { name: "Enable Live Illustrator" }),
  ).toBeInTheDocument();
});

// Regression: the decorative track/thumb spans used to sit on top of the hidden
// <input> and swallow the click, so the control only toggled by keyboard. A
// plain click on the rendered control must flip it.
test("a pointer click on the control toggles it", async () => {
  const user = userEvent.setup();
  render(<Controlled />);
  const sw = screen.getByRole("switch") as HTMLInputElement;
  expect(sw.checked).toBe(false);
  await user.click(sw);
  expect(sw.checked).toBe(true);
  await user.click(sw);
  expect(sw.checked).toBe(false);
});

// Regression (P1 / v0.1.3): the click that reaches the reader is often on the
// decorative track, not the 0-opacity input. The <label> wrapper must forward it.
test("a click on the decorative track toggles it", async () => {
  const user = userEvent.setup();
  render(<Controlled />);
  const sw = screen.getByRole("switch") as HTMLInputElement;
  const wrapper = sw.closest("label");
  expect(wrapper).not.toBeNull();
  const track = wrapper!.querySelector("span[aria-hidden='true']") as HTMLElement;
  expect(sw.checked).toBe(false);
  await user.click(track);
  expect(sw.checked).toBe(true);
});

test("keyboard toggling keeps working", async () => {
  const user = userEvent.setup();
  render(<Controlled />);
  const sw = screen.getByRole("switch") as HTMLInputElement;
  sw.focus();
  await user.keyboard(" ");
  expect(sw.checked).toBe(true);
});

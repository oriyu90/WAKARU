import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { Drawer } from "./Drawer";

test("a closed drawer releases all inline layout space", () => {
  const { rerender } = render(
    <Drawer open={false} onClose={vi.fn()} label="Live explanation" widthRem={30}>
      content
    </Drawer>,
  );

  const drawer = screen.getByRole("region", { hidden: true });
  expect(drawer.style.inlineSize).toBe("0px");
  expect(drawer.style.minInlineSize).toBe("0px");
  expect(drawer).toHaveAttribute("aria-hidden", "true");
  expect(drawer).toHaveAttribute("inert");

  rerender(
    <Drawer open onClose={vi.fn()} label="Live explanation" widthRem={30}>
      content
    </Drawer>,
  );
  expect(
    screen.getByRole("region", { name: "Live explanation" }).style.inlineSize,
  ).toBe("30rem");
});

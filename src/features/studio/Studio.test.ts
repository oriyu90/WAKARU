import { describe, expect, it } from "vitest";
import { shouldSubmitStudioKey } from "./Studio";

describe("Studio composer keyboard contract", () => {
  it("sends with Enter and keeps Shift+Enter for a new line", () => {
    expect(shouldSubmitStudioKey("Enter", false, false)).toBe(true);
    expect(shouldSubmitStudioKey("Enter", true, false)).toBe(false);
    expect(shouldSubmitStudioKey("a", false, false)).toBe(false);
  });

  it("does not submit while an IME composition is active", () => {
    expect(shouldSubmitStudioKey("Enter", false, true)).toBe(false);
  });
});

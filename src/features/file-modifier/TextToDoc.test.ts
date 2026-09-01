import { describe, expect, it } from "vitest";
import { isSuspiciouslyShort } from "./TextToDoc";

describe("isSuspiciouslyShort", () => {
  it("flags likely information loss", () => {
    expect(isSuspiciouslyShort("abcdefghij", "123456")).toBe(true);
  });

  it("allows output at the safety boundary", () => {
    expect(isSuspiciouslyShort("abcdefghij", "1234567")).toBe(false);
  });

  it("does not flag an empty pending output", () => {
    expect(isSuspiciouslyShort("abcdefghij", "")).toBe(false);
  });
});

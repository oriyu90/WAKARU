import { describe, expect, it, beforeEach } from "vitest";
import { useUiStore } from "./ui";

describe("document rendering adjustments", () => {
  beforeEach(() => {
    useUiStore.setState({ docInverted: false, docClarity: 0 });
  });

  it("starts unadjusted", () => {
    expect(useUiStore.getState().docInverted).toBe(false);
    expect(useUiStore.getState().docClarity).toBe(0);
  });

  it("toggles inversion", () => {
    useUiStore.getState().setDocInverted(true);
    expect(useUiStore.getState().docInverted).toBe(true);
  });

  it("clamps clarity to 0–100", () => {
    useUiStore.getState().setDocClarity(40);
    expect(useUiStore.getState().docClarity).toBe(40);
    useUiStore.getState().setDocClarity(999);
    expect(useUiStore.getState().docClarity).toBe(100);
    useUiStore.getState().setDocClarity(-5);
    expect(useUiStore.getState().docClarity).toBe(0);
  });
});

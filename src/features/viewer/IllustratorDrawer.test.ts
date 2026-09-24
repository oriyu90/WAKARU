import { describe, expect, it } from "vitest";
import { partitionLiveMessages } from "./IllustratorDrawer";

const msg = (id: string, createdAt: string) => ({ id, createdAt });

describe("partitionLiveMessages (past vs this Live session)", () => {
  it("keeps pre-session turns in the past and new turns live", () => {
    const messages = [
      msg("a", "2026-09-20T10:00:00Z"),
      msg("b", "2026-09-20T10:01:00Z"),
      msg("c", "2026-09-24T09:00:00Z"),
    ];
    const { past, live } = partitionLiveMessages(messages, {
      at: "2026-09-20T10:01:00Z",
      id: "b",
    });
    expect(past.map((m) => m.id)).toEqual(["a", "b"]);
    expect(live.map((m) => m.id)).toEqual(["c"]);
  });

  it("treats everything as live before the anchor is captured", () => {
    const messages = [msg("a", "2026-09-24T09:00:00Z")];
    const { past, live } = partitionLiveMessages(messages, null);
    expect(past).toEqual([]);
    expect(live.map((m) => m.id)).toEqual(["a"]);
  });

  it("never shows the newest turn as past after asking", () => {
    const messages = [
      msg("old", "2026-09-20T10:00:00Z"),
      msg("q", "2026-09-24T09:00:01Z"),
      msg("a", "2026-09-24T09:00:02Z"),
    ];
    const { past, live } = partitionLiveMessages(messages, {
      at: "2026-09-20T10:00:00Z",
      id: "old",
    });
    expect(past.map((m) => m.id)).toEqual(["old"]);
    expect(live.map((m) => m.id)).toEqual(["q", "a"]);
  });

  it("breaks same-second ties by id order (second-precision timestamps)", () => {
    const messages = [
      msg("0196aaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "2026-09-24T09:00:00Z"),
      msg("0196bbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb", "2026-09-24T09:00:00Z"),
    ];
    const { past, live } = partitionLiveMessages(messages, {
      at: "2026-09-24T09:00:00Z",
      id: "0196aaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
    });
    expect(past.map((m) => m.id)).toEqual([
      "0196aaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
    ]);
    expect(live.map((m) => m.id)).toEqual([
      "0196bbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
    ]);
  });
});

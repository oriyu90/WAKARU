import { describe, expect, it } from "vitest";
import {
  isPrePersistStudioError,
  newClientRequestId,
  shouldSubmitStudioKey,
  usableChatProfiles,
} from "./Studio";
import type { AiProfile } from "../../ipc/types.gen";

const profile = (id: string, defaultModel: string | null): AiProfile => ({
  id,
  name: id,
  baseUrl: "http://localhost:1234/v1",
  protocol: "openai",
  hasKey: false,
  defaultModel,
  supportsVision: false,
  supportsTools: false,
  supportsEmbed: false,
  jsonSchema: false,
  extraHeaders: {},
  timeoutMs: 120000,
  createdAt: "",
  lastOkAt: null,
});

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

describe("usableChatProfiles (session model override)", () => {
  it("keeps only profiles with a default model", () => {
    const out = usableChatProfiles([
      profile("a", "model-a"),
      profile("b", null),
      profile("c", "  "),
    ]);
    expect(out.map((p) => p.id)).toEqual(["a"]);
  });
});

describe("v1.5.0 optimistic send keys", () => {
  it("issues unique non-empty client request ids", () => {
    const seen = new Set([newClientRequestId(), newClientRequestId(), newClientRequestId()]);
    expect(seen.size).toBe(3);
    for (const id of seen) expect(id.trim().length).toBeGreaterThan(0);
  });
});

describe("v1.5.0 pre/post persist classification", () => {
  it("restores the draft only for pre-persist failures", () => {
    expect(isPrePersistStudioError("STUDIO_EMPTY_MESSAGE")).toBe(true);
    expect(isPrePersistStudioError("STUDIO_REWIND_LOCKED")).toBe(true);
    expect(isPrePersistStudioError("AI_NOT_CONFIGURED")).toBe(true);
    expect(isPrePersistStudioError("AI_REQUEST")).toBe(false);
    expect(isPrePersistStudioError("AI_TRUNCATED")).toBe(false);
    expect(isPrePersistStudioError("")).toBe(false);
  });
});

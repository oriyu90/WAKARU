import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { ViewerTab } from "../../ipc/types.gen";
import { rememberTabLocator } from "./Preview";

describe("viewer locator persistence", () => {
  it("updates the cached page before a tab is left and remounted", () => {
    const client = new QueryClient();
    const tabs: ViewerTab[] = [
      {
        id: "tab-1",
        sourceId: "source-1",
        kind: "pdf",
        name: "guide.pdf",
        locator: { page: 1 },
        pinned: false,
        ordinal: 0,
      },
      {
        id: "tab-2",
        sourceId: "source-2",
        kind: "pdf",
        name: "other.pdf",
        locator: { page: 4 },
        pinned: false,
        ordinal: 1,
      },
    ];
    client.setQueryData(["viewer-tabs", "project-1"], tabs);

    rememberTabLocator(client, "project-1", "tab-1", { page: 17 });

    const remembered = client.getQueryData<ViewerTab[]>([
      "viewer-tabs",
      "project-1",
    ]);
    expect(remembered?.at(0)?.locator).toEqual({ page: 17 });
    expect(remembered?.at(1)).toEqual(tabs.at(1));
  });
});

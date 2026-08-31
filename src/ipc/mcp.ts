import { call } from "./client";
import type {
  McpConnectResult,
  McpServer,
  McpUpsertInput,
} from "./types.gen";

export const mcpApi = {
  list: () => call<McpServer[]>("mcp_list_servers"),
  upsert: (input: McpUpsertInput) => call<McpServer>("mcp_upsert_server", { input }),
  remove: (id: string) => call<void>("mcp_delete_server", { id }),
  connect: (id: string) => call<McpConnectResult>("mcp_connect", { id }),
  disconnect: (id: string) => call<void>("mcp_disconnect", { id }),
  setPolicy: (serverId: string, toolName: string, policy: string) =>
    call<void>("mcp_set_tool_policy", { serverId, toolName, policy }),
  stderr: (id: string) => call<string[]>("mcp_server_stderr", { id }),
};

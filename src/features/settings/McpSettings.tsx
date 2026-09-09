import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { Select } from "../../components/Select";
import { EmptyState } from "../../components/EmptyState";
import { useToast } from "../../components/useToast";
import { inTauri } from "../../ipc/client";
import { mcpApi } from "../../ipc/mcp";
import type { McpServer, McpTool } from "../../ipc/types.gen";
import styles from "./McpSettings.module.css";

type Draft = {
  id?: string;
  name: string;
  transport: "stdio" | "http";
  command: string;
  url: string;
  args: string;
  env: string;
};

const BLANK: Draft = { name: "", transport: "stdio", command: "", url: "", args: "", env: "" };

/** One-click starting points for common servers. The user still reviews every
 * field and saves manually; nothing connects on its own. `command` stays a bare
 * name — the backend resolves `npx` from the usual install locations. */
const PRESETS: Record<string, Omit<Draft, "id">> = {
  searxng: {
    name: "SearXNG",
    transport: "stdio",
    command: "npx",
    args: "-y mcp-searxng",
    url: "",
    env: "SEARXNG_URL=http://localhost:8888",
  },
};

function toDraft(s: McpServer): Draft {
  return {
    id: s.id,
    name: s.name,
    transport: s.transport === "http" ? "http" : "stdio",
    command: s.command ?? "",
    url: s.url ?? "",
    args: s.args.join(" "),
    env: s.envKeys.map((k) => `${k}=`).join("\n"),
  };
}

export function McpSettings() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const [editing, setEditing] = useState<Draft | null>(null);
  const [tools, setTools] = useState<Record<string, McpTool[]>>({});

  const servers = useQuery({
    queryKey: ["mcp-servers"],
    queryFn: mcpApi.list,
    enabled: inTauri,
  });
  const refresh = () => qc.invalidateQueries({ queryKey: ["mcp-servers"] });

  const save = useMutation({
    mutationFn: (d: Draft) => {
      const env: Record<string, string> = {};
      for (const line of d.env.split("\n")) {
        const eq = line.indexOf("=");
        if (eq > 0) env[line.slice(0, eq).trim()] = line.slice(eq + 1);
      }
      return mcpApi.upsert({
        id: d.id ?? null,
        name: d.name.trim(),
        transport: d.transport,
        command: d.transport === "stdio" ? d.command.trim() || null : null,
        args: d.transport === "stdio" && d.args.trim() ? d.args.trim().split(/\s+/) : [],
        url: d.transport === "http" ? d.url.trim() || null : null,
        env,
      });
    },
    onSuccess: () => {
      setEditing(null);
      void refresh();
    },
    onError: (e) => toast.push({ tone: "error", message: (e as Error).message }),
  });

  const remove = useMutation({
    mutationFn: (id: string) => mcpApi.remove(id),
    onSuccess: refresh,
  });

  const connect = useMutation({
    mutationFn: (id: string) => mcpApi.connect(id),
    onSuccess: (res, id) => {
      setTools((m) => ({ ...m, [id]: res.tools }));
      void refresh();
    },
    onError: (e) => toast.push({ tone: "error", message: (e as Error).message }),
  });
  const disconnect = useMutation({
    mutationFn: (id: string) => mcpApi.disconnect(id),
    onSuccess: (_r, id) => {
      setTools((m) => {
        const n = { ...m };
        delete n[id];
        return n;
      });
      void refresh();
    },
  });
  const setPolicy = useMutation({
    mutationFn: (v: { serverId: string; toolName: string; policy: string }) =>
      mcpApi.setPolicy(v.serverId, v.toolName, v.policy),
    onSuccess: (_r, v) =>
      setTools((m) => ({
        ...m,
        [v.serverId]: (m[v.serverId] ?? []).map((tool) =>
          tool.name === v.toolName ? { ...tool, policy: v.policy } : tool,
        ),
      })),
  });

  const rows = servers.data ?? [];

  return (
    <div className={styles.wrap}>
      <p className={styles.note}>{t("mcp.transportSupport")}</p>

      {rows.length === 0 ? (
        <EmptyState title={t("mcp.empty")} body={t("mcp.emptyBody")} />
      ) : (
        <ul className={styles.list}>
          {rows.map((s) => (
            <li key={s.id} className={styles.server}>
              <div className={styles.head}>
                <span className={styles.name}>{s.name}</span>
                <span className={s.connected ? styles.on : styles.off}>
                  {s.connected ? t("mcp.connected") : t("mcp.disconnected")}
                </span>
                <div className={styles.actions}>
                  {s.connected ? (
                    <Button size="sm" variant="quiet" loading={disconnect.isPending} onClick={() => disconnect.mutate(s.id)}>
                      {t("mcp.disconnect")}
                    </Button>
                  ) : (
                    <Button size="sm" variant="secondary" loading={connect.isPending} onClick={() => connect.mutate(s.id)}>
                      {t("mcp.connect")}
                    </Button>
                  )}
                  <Button size="sm" variant="quiet" onClick={() => setEditing(toDraft(s))}>
                    {t("common.edit")}
                  </Button>
                  <Button size="sm" variant="danger" loading={remove.isPending} onClick={() => remove.mutate(s.id)}>
                    {t("common.delete")}
                  </Button>
                </div>
              </div>
              <code className={styles.cmd}>
                {s.transport === "http" ? s.url : `${s.command ?? ""} ${s.args.join(" ")}`}
              </code>

              {(tools[s.id] ?? []).length > 0 ? (
                <ul className={styles.tools}>
                  {(tools[s.id] ?? []).map((tool) => (
                    <li key={tool.name} className={styles.tool}>
                      <span className={styles.toolName}>{tool.name}</span>
                      <Select
                        aria-label={t("mcp.policy")}
                        value={tool.policy}
                        onChange={(e) =>
                          setPolicy.mutate({ serverId: s.id, toolName: tool.name, policy: e.target.value })
                        }
                      >
                        <option value="ask">{t("mcp.policyAsk")}</option>
                        <option value="always_allow">{t("mcp.policyAllow")}</option>
                        <option value="deny">{t("mcp.policyDeny")}</option>
                      </Select>
                    </li>
                  ))}
                </ul>
              ) : null}
            </li>
          ))}
        </ul>
      )}

      <Button variant="secondary" size="sm" onClick={() => setEditing({ ...BLANK })}>
        {t("mcp.add")}
      </Button>

      <Dialog
        open={editing !== null}
        onClose={() => setEditing(null)}
        title={editing?.id ? t("mcp.editServer") : t("mcp.addServer")}
        footer={
          <>
            <Button variant="quiet" onClick={() => setEditing(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              loading={save.isPending}
              disabled={!editing?.name.trim() || (editing.transport === "stdio" ? !editing.command.trim() : !editing.url.trim())}
              onClick={() => editing && save.mutate(editing)}
            >
              {t("common.save")}
            </Button>
          </>
        }
      >
        {editing ? (
          <div className={styles.form}>
            {!editing.id ? (
              <Field label={t("mcp.preset")} hint={t("mcp.presetSearxngHint")}>
                {({ id }) => (
                  <Select
                    id={id}
                    value=""
                    onChange={(e) => {
                      const p = PRESETS[e.target.value];
                      if (p) setEditing({ ...p });
                    }}
                  >
                    <option value="">{t("mcp.presetCustom")}</option>
                    <option value="searxng">{t("mcp.presetSearxng")}</option>
                  </Select>
                )}
              </Field>
            ) : null}
            <Field label={t("mcp.name")}>
              {({ id }) => (
                <Input
                  id={id}
                  value={editing.name}
                  onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                />
              )}
            </Field>
            <Field label={t("mcp.transport")}>
              {({ id }) => (
                <Select id={id} value={editing.transport} onChange={(e) => setEditing({ ...editing, transport: e.target.value as Draft["transport"] })}>
                  <option value="stdio">{t("mcp.transportStdio")}</option>
                  <option value="http">{t("mcp.transportHttp")}</option>
                </Select>
              )}
            </Field>
            {editing.transport === "stdio" ? <><Field label={t("mcp.command")} hint={t("mcp.commandHint")}>
              {({ id }) => (
                <Input
                  id={id}
                  value={editing.command}
                  onChange={(e) => setEditing({ ...editing, command: e.target.value })}
                  placeholder="/usr/local/bin/my-mcp-server"
                />
              )}
            </Field>
            <Field label={t("mcp.args")}>
              {({ id }) => (
                <Input
                  id={id}
                  value={editing.args}
                  onChange={(e) => setEditing({ ...editing, args: e.target.value })}
                />
              )}
            </Field></> : <Field label={t("mcp.url")} hint={t("mcp.urlHint")}>
              {({ id }) => <Input id={id} value={editing.url} onChange={(e) => setEditing({ ...editing, url: e.target.value })} placeholder="https://example.com/mcp" />}
            </Field>}
            <Field label={editing.transport === "http" ? t("mcp.headers") : t("mcp.env")} hint={editing.transport === "http" ? t("mcp.headersHint") : t("mcp.envHint")}>
              {({ id }) => (
                <textarea
                  id={id}
                  className={styles.envBox}
                  rows={3}
                  value={editing.env}
                  onChange={(e) => setEditing({ ...editing, env: e.target.value })}
                />
              )}
            </Field>
          </div>
        ) : null}
      </Dialog>
    </div>
  );
}

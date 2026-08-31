import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { IconButton } from "../../components/IconButton";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { Select } from "../../components/Select";
import { EmptyState } from "../../components/EmptyState";
import { Spinner } from "../../components/Spinner";
import { CloseIcon, PlusIcon, InfoIcon } from "../../app/Icons";
import { aiApi, AI_PRESETS } from "../../ipc/ai";
import { useToast } from "../../components/useToast";
import type { AiProfile, ApiProtocol, Role, TestResult } from "../../ipc/types.gen";
import styles from "./AiSettings.module.css";

const ROLES: Role[] = ["chat", "vision", "embedding", "organizer"];

export function AiSettings() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const [editing, setEditing] = useState<AiProfile | "new" | null>(null);
  const [tested, setTested] = useState<Record<string, TestResult>>({});

  const profiles = useQuery({ queryKey: ["ai-profiles"], queryFn: aiApi.listProfiles });
  const bindings = useQuery({ queryKey: ["ai-bindings"], queryFn: aiApi.getRoleBindings });

  const del = useMutation({
    mutationFn: (id: string) => aiApi.deleteProfile(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["ai-profiles"] });
      void qc.invalidateQueries({ queryKey: ["ai-bindings"] });
    },
    onError: () => toast.push({ tone: "error", message: t("ai.deleteFailed") }),
  });

  const test = useMutation({
    mutationFn: (id: string) => aiApi.testProfile(id),
    onSuccess: (res, id) => {
      setTested((m) => ({ ...m, [id]: res }));
      void qc.invalidateQueries({ queryKey: ["ai-profiles"] });
      if (!res.ok) toast.push({ tone: "error", message: res.note ?? t("ai.unreachable") });
    },
    onError: () => toast.push({ tone: "error", message: t("ai.unreachable") }),
  });

  const setBinding = useMutation({
    mutationFn: ({ role, profileId, model }: { role: Role; profileId: string; model: string }) =>
      profileId ? aiApi.setRoleBinding(role, profileId, model) : aiApi.clearRoleBinding(role),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ai-bindings"] }),
    onError: () => toast.push({ tone: "error", message: t("ai.bindingFailed") }),
  });

  const list = profiles.data ?? [];

  return (
    <div className={styles.wrap}>
      <div className={styles.head}>
        <h2 className={styles.title}>{t("ai.connections")}</h2>
        <Button variant="primary" size="sm" icon={<PlusIcon size={14} />} onClick={() => setEditing("new")}>
          {t("ai.add")}
        </Button>
      </div>

      {list.length === 0 ? (
        <EmptyState title={t("ai.noneTitle")} body={t("ai.noneBody")} />
      ) : (
        <ul className={styles.profiles}>
          {list.map((p) => {
            const r = tested[p.id];
            return (
              <li key={p.id} className={styles.profile}>
                <div className={styles.profileMain}>
                  <span className={styles.profileName}>{p.name}</span>
                  <span className={styles.cap}>{t(`ai.protocol.${p.protocol}`)}</span>
                  <span className={styles.profileUrl}>{p.baseUrl}</span>
                  <span className={styles.caps}>
                    {p.hasKey ? <span className={styles.cap}>key</span> : null}
                    {(r?.supportsVision ?? p.supportsVision) ? <span className={styles.cap}>vision</span> : null}
                    {(r?.supportsTools ?? p.supportsTools) ? <span className={styles.cap}>tools</span> : null}
                    {(r?.supportsEmbed ?? p.supportsEmbed) ? <span className={styles.cap}>embed</span> : null}
                    {r ? <span className={styles.latency}>{r.latencyMs} ms · {r.models.length} models</span> : null}
                  </span>
                </div>
                <div className={styles.profileActions}>
                  <Button size="sm" variant="quiet" loading={test.isPending && test.variables === p.id} onClick={() => test.mutate(p.id)}>
                    {t("ai.test")}
                  </Button>
                  <Button size="sm" variant="quiet" onClick={() => setEditing(p)}>
                    {t("ai.edit")}
                  </Button>
                  <IconButton label={t("common.close")} size="sm" onClick={() => del.mutate(p.id)}>
                    <CloseIcon size={13} />
                  </IconButton>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <h2 className={styles.title}>{t("ai.roles")}</h2>
      {profiles.isLoading || bindings.isLoading ? (
        <Spinner />
      ) : (
        <div className={styles.roleGrid}>
          {ROLES.map((role) => {
            const b = bindings.data?.[role] ?? null;
            return (
              <div key={role} className={styles.roleRow}>
                <span className={styles.roleLabel}>{t(`ai.role.${role}`)}</span>
                <Select
                  value={b?.profileId ?? ""}
                  onChange={(e) =>
                    setBinding.mutate({
                      role,
                      profileId: e.target.value,
                      model: b?.model ?? list.find((p) => p.id === e.target.value)?.defaultModel ?? "",
                    })
                  }
                >
                  <option value="">{t("ai.roleNone")}</option>
                  {list.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </Select>
                <Input
                  placeholder={t("ai.model")}
                  defaultValue={b?.model ?? ""}
                  disabled={!b}
                  onBlur={(e) => {
                    if (b && e.target.value !== b.model)
                      setBinding.mutate({ role, profileId: b.profileId, model: e.target.value });
                  }}
                />
              </div>
            );
          })}
        </div>
      )}

      {!bindings.data?.embedding ? (
        <p className={styles.degrade}>
          <InfoIcon size={14} />
          {t("ai.embeddingNote")}
        </p>
      ) : null}

      {editing ? (
        <ProfileDialog
          profile={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            void qc.invalidateQueries({ queryKey: ["ai-profiles"] });
          }}
        />
      ) : null}
    </div>
  );
}

function ProfileDialog({
  profile,
  onClose,
  onSaved,
}: {
  profile: AiProfile | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation();
  const toast = useToast();
  const [name, setName] = useState(profile?.name ?? "");
  const [baseUrl, setBaseUrl] = useState(profile?.baseUrl ?? "");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState(profile?.defaultModel ?? "");
  const [protocol, setProtocol] = useState<ApiProtocol>(profile?.protocol ?? "openai");

  const save = useMutation({
    mutationFn: () =>
      aiApi.upsertProfile({
        id: profile?.id ?? null,
        name,
        baseUrl,
        protocol,
        apiKey: apiKey || null,
        defaultModel: model || null,
        extraHeaders: null,
        timeoutMs: null,
      }),
    onSuccess: onSaved,
    onError: () => toast.push({ tone: "error", message: t("ai.saveFailed") }),
  });

  return (
    <Dialog
      open
      onClose={onClose}
      title={profile ? t("ai.editTitle") : t("ai.addTitle")}
      footer={
        <>
          <Button variant="quiet" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            loading={save.isPending}
            disabled={!name.trim() || !baseUrl.trim()}
            onClick={() => save.mutate()}
          >
            {t("common.save")}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        <Field label={t("ai.preset")}>
          {({ id }) => (
            <Select
              id={id}
              onChange={(e) => {
                const p = AI_PRESETS.find((x) => x.label === e.target.value);
                if (p) {
                  setProtocol(p.protocol);
                  if (p.baseUrl) setBaseUrl(p.baseUrl);
                }
              }}
            >
              {AI_PRESETS.map((p) => (
                <option key={p.label}>{p.label}</option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("ai.protocolLabel")} required>
          {({ id }) => (
            <Select id={id} value={protocol} onChange={(e) => setProtocol(e.target.value as ApiProtocol)}>
              <option value="openai">{t("ai.protocol.openai")}</option>
              <option value="anthropic">{t("ai.protocol.anthropic")}</option>
            </Select>
          )}
        </Field>
        <Field label={t("ai.name")} required>
          {({ id }) => <Input id={id} value={name} autoFocus onChange={(e) => setName(e.target.value)} />}
        </Field>
        <Field label={t("ai.baseUrl")} required hint={t("ai.baseUrlHint")}>
          {({ id }) => (
            <Input id={id} value={baseUrl} placeholder="http://localhost:1234/v1" onChange={(e) => setBaseUrl(e.target.value)} />
          )}
        </Field>
        <Field label={t("ai.apiKey")} hint={profile?.hasKey ? t("ai.keyStored") : t("ai.optional")}>
          {({ id }) => (
            <Input id={id} type="password" value={apiKey} placeholder={profile?.hasKey ? "••••••••" : ""} onChange={(e) => setApiKey(e.target.value)} />
          )}
        </Field>
        <Field label={t("ai.defaultModel")}>
          {({ id }) => <Input id={id} value={model} onChange={(e) => setModel(e.target.value)} />}
        </Field>
      </div>
    </Dialog>
  );
}

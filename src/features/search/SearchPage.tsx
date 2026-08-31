import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Input } from "../../components/Input";
import { Select } from "../../components/Select";
import { Button } from "../../components/Button";
import { EmptyState } from "../../components/EmptyState";
import { ErrorState } from "../../components/ErrorState";
import { Spinner } from "../../components/Spinner";
import { searchApi } from "../../ipc/search";
import { projectsApi } from "../../ipc/projects";
import { inTauri } from "../../ipc/client";
import type { SearchHit } from "../../ipc/types.gen";
import styles from "./SearchPage.module.css";

type Mode = "keyword" | "semantic" | "hybrid";

export function SearchPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [term, setTerm] = useState("");
  const [q, setQ] = useState("");
  const [projectId, setProjectId] = useState<string>("");
  const [mode, setMode] = useState<Mode>("hybrid");

  const projects = useQuery({
    queryKey: ["projects", false],
    queryFn: () => projectsApi.list(false),
    enabled: inTauri,
  });

  const results = useQuery({
    queryKey: ["search", q, projectId, mode],
    enabled: inTauri && q.trim().length > 0,
    queryFn: () =>
      searchApi.query({
        scope: projectId ? "project" : "global",
        projectId: projectId || null,
        q,
        sourceId: null,
        limit: 40,
        mode: projectId ? mode : null,
      }),
  });

  const grouped = groupByProject(results.data?.hits ?? []);

  function jump(hit: SearchHit) {
    navigate(`/p/${hit.projectId}`);
  }

  return (
    <section className={styles.page}>
      <h1 className={styles.title}>{t("search.title")}</h1>

      <form
        className={styles.bar}
        onSubmit={(e) => {
          e.preventDefault();
          setQ(term);
        }}
      >
        <Input
          value={term}
          autoFocus
          placeholder={t("search.placeholder")}
          onChange={(e) => setTerm(e.target.value)}
        />
        <Select
          value={projectId}
          onChange={(e) => setProjectId(e.target.value)}
        >
          <option value="">{t("search.allProjects")}</option>
          {(projects.data ?? []).map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </Select>
        {projectId ? (
          <Select
            value={mode}
            onChange={(e) => setMode(e.target.value as Mode)}
          >
            <option value="hybrid">{t("search.mode.hybrid")}</option>
            <option value="keyword">{t("search.mode.keyword")}</option>
            <option value="semantic">{t("search.mode.semantic")}</option>
          </Select>
        ) : null}
        <Button type="submit" variant="primary">
          {t("search.go")}
        </Button>
      </form>

      <div className={styles.body}>
        {!q.trim() ? (
          <EmptyState title={t("search.title")} body={t("search.hint")} />
        ) : results.isLoading ? (
          <Spinner label={t("common.loading")} />
        ) : results.isError ? (
          <ErrorState error={results.error} onRetry={() => results.refetch()} />
        ) : grouped.length === 0 ? (
          <EmptyState title={t("search.noResults")} body="" />
        ) : (
          <>
            {results.data && projectId && !results.data.semantic ? (
              <p className={styles.degrade}>{t("search.keywordOnly")}</p>
            ) : null}
            {grouped.map((g) => (
              <div key={g.projectId} className={styles.group}>
                <h2 className={styles.groupName}>{g.projectName}</h2>
                {g.hits.map((h, i) => (
                  <button key={i} type="button" className={styles.hit} onClick={() => jump(h)}>
                    <span className={styles.hitSource}>
                      {h.sourceName}
                      {h.ordinal ? <span className={styles.hitLoc}> · {h.ordinal}</span> : null}
                    </span>
                    <span className={styles.hitSnippet}>{h.snippet}</span>
                  </button>
                ))}
              </div>
            ))}
          </>
        )}
      </div>
    </section>
  );
}

function groupByProject(hits: SearchHit[]) {
  const map = new Map<string, { projectId: string; projectName: string; hits: SearchHit[] }>();
  for (const h of hits) {
    const g = map.get(h.projectId) ?? { projectId: h.projectId, projectName: h.projectName, hits: [] };
    g.hits.push(h);
    map.set(h.projectId, g);
  }
  return [...map.values()];
}

import { useEffect, useRef, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useUiStore } from "../stores/ui";
import { FOCUSABLE, trapTab } from "../components/focus";
import { IconButton } from "../components/IconButton";
import { Button } from "../components/Button";
import { Dialog } from "../components/Dialog";
import { ContextMenu } from "../components/ContextMenu";
import { useToast } from "../components/useToast";
import { projectsApi } from "../ipc/projects";
import { exportApi } from "../ipc/exportImport";
import { pickSaveDir } from "../ipc/fileModifier";
import { IpcError, inTauri } from "../ipc/client";
import type { ProjectSummary } from "../ipc/types.gen";
import {
  MenuIcon,
  HomeIcon,
  FileSwapIcon,
  SearchIcon,
  SettingsIcon,
  FileIcon,
} from "./Icons";
import { FirstRun } from "./FirstRun";
import styles from "./AppShell.module.css";

const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.platform || navigator.userAgent);

export function AppShell() {
  const { t } = useTranslation();
  const location = useLocation();
  const navigate = useNavigate();
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);

  // Where "settings" should return to when pressed a second time (P3).
  const onSettings = location.pathname === "/settings";
  const lastNonSettingsPath = useRef("/");
  if (!onSettings) lastNonSettingsPath.current = location.pathname + location.search;
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const closeSidebar = useUiStore((s) => s.closeSidebar);

  const sidebarRef = useRef<HTMLElement>(null);
  const menuBtnRef = useRef<HTMLButtonElement>(null);
  const contentRef = useRef<HTMLElement>(null);

  const projects = useQuery({
    queryKey: ["projects", false],
    queryFn: () => projectsApi.list(false),
    enabled: inTauri,
  });

  const qc = useQueryClient();
  const toast = useToast();
  // Right-click / context-menu-key on a project link (P2).
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    project: ProjectSummary;
    trigger: HTMLElement | null;
  } | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<ProjectSummary | null>(null);
  const [deleting, setDeleting] = useState(false);

  async function exportProject(p: ProjectSummary) {
    try {
      const destDir = await pickSaveDir();
      if (!destDir) return;
      const r = await exportApi.export({
        ids: [p.id],
        destDir,
        includeEmbeddings: false,
      });
      toast.push({ tone: "success", message: t("pm.exported", { count: r.files.length }) });
    } catch (e) {
      if (e instanceof Error && e.message === "cancelled") return;
      toast.push({ tone: "error", message: t("errors.internal") });
    }
  }

  async function deleteProject(p: ProjectSummary) {
    setDeleting(true);
    try {
      await projectsApi.delete(p.id, p.name);
      await qc.invalidateQueries({ queryKey: ["projects"] });
      setConfirmDelete(null);
      if (location.pathname === `/p/${p.id}`) navigate("/", { replace: true });
    } catch (e) {
      toast.push({
        tone: "error",
        message:
          e instanceof IpcError
            ? t([`errors.${e.code}`, "errors.internal"])
            : t("errors.internal"),
      });
    } finally {
      setDeleting(false);
    }
  }

  // Cmd/Ctrl+B toggles; Esc closes (docs/06 §3.2).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const meta = e.metaKey || e.ctrlKey;
      if (meta && e.key.toLowerCase() === "b") {
        e.preventDefault();
        toggleSidebar();
      } else if (e.key === "Escape" && useUiStore.getState().sidebarOpen) {
        e.preventDefault();
        closeSidebar();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggleSidebar, closeSidebar]);

  // Focus trap while the sidebar is open; restore focus on close.
  useEffect(() => {
    if (!sidebarOpen) return;
    const el = sidebarRef.current;
    const menuBtn = menuBtnRef.current;
    if (!el) return;
    el.querySelector<HTMLElement>(FOCUSABLE)?.focus();
    const onKey = (e: KeyboardEvent) => trapTab(el, e);
    el.addEventListener("keydown", onKey);
    return () => {
      el.removeEventListener("keydown", onKey);
      menuBtn?.focus();
    };
  }, [sidebarOpen]);

  // Close the sidebar and reset scroll on navigation.
  useEffect(() => {
    closeSidebar();
    if (contentRef.current) contentRef.current.scrollTop = 0;
  }, [location.pathname, closeSidebar]);

  const navItem = ({ isActive }: { isActive: boolean }) =>
    `${styles.navItem} ${isActive ? styles.navItemActive : ""}`;

  return (
    <div className={styles.shell}>
      <header className={styles.topBar}>
        <IconButton
          ref={menuBtnRef}
          label={sidebarOpen ? t("app.closeSidebar") : t("app.openSidebar")}
          onClick={toggleSidebar}
          aria-expanded={sidebarOpen}
          aria-controls="app-sidebar"
        >
          <MenuIcon />
        </IconButton>
        <span className={styles.wordmark}>{t("app.name")}</span>
        <span className={styles.tagline}>{t("app.tagline")}</span>
        <span className={styles.spacer} />
        <kbd className={styles.kbd}>{isMac ? "⌘B" : "Ctrl B"}</kbd>
        <button
          type="button"
          className={`${styles.topAction} ${onSettings ? styles.topActionActive : ""}`}
          aria-current={onSettings ? "page" : undefined}
          aria-label={onSettings ? t("nav.settingsClose") : t("nav.settings")}
          onClick={() =>
            navigate(onSettings ? lastNonSettingsPath.current : "/settings")
          }
        >
          <SettingsIcon />
        </button>
      </header>

      <div className={styles.body}>
        <button
          type="button"
          className={styles.scrim}
          data-open={sidebarOpen}
          aria-hidden={!sidebarOpen}
          tabIndex={-1}
          onClick={closeSidebar}
        />

        <nav
          id="app-sidebar"
          ref={sidebarRef}
          className={styles.sidebar}
          data-open={sidebarOpen}
          aria-label={t("nav.projects")}
        >
          <div className={styles.navTop}>
            <NavLink to="/" end className={navItem}>
              <HomeIcon />
              {t("nav.home")}
            </NavLink>
            <NavLink to="/file-modifier" className={navItem}>
              <FileSwapIcon />
              {t("nav.fileModifier")}
            </NavLink>
          </div>

          <div className={styles.navProjects}>
            {(projects.data ?? []).map((p) => (
              <NavLink
                key={p.id}
                to={`/p/${p.id}`}
                className={navItem}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setMenu({
                    x: e.clientX,
                    y: e.clientY,
                    project: p,
                    trigger: e.currentTarget as HTMLElement,
                  });
                }}
              >
                <FileIcon />
                <span className={styles.projName}>{p.name}</span>
                <span className={`${styles.projCount} u-mono-nums`}>
                  {p.sourceCount}
                </span>
              </NavLink>
            ))}
            {projects.data && projects.data.length === 0 ? (
              <p className={styles.navEmpty}>{t("nav.noProjects")}</p>
            ) : null}
          </div>

          <div className={styles.navBottom}>
            <NavLink to="/search" className={navItem}>
              <SearchIcon />
              {t("nav.search")}
            </NavLink>
          </div>
        </nav>

        <main ref={contentRef} className={styles.content}>
          <Outlet />
        </main>
      </div>

      {menu ? (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          label={menu.project.name}
          returnFocus={menu.trigger}
          onClose={() => setMenu(null)}
          items={[
            {
              label: t("nav.exportProject"),
              onSelect: () => void exportProject(menu.project),
            },
            {
              label: t("nav.deleteProject"),
              danger: true,
              onSelect: () => setConfirmDelete(menu.project),
            },
          ]}
        />
      ) : null}

      <Dialog
        open={!!confirmDelete}
        onClose={() => (deleting ? undefined : setConfirmDelete(null))}
        title={t("nav.deleteProjectTitle")}
        footer={
          <>
            <Button variant="quiet" onClick={() => setConfirmDelete(null)} disabled={deleting}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              loading={deleting}
              onClick={() => confirmDelete && void deleteProject(confirmDelete)}
            >
              {t("common.delete")}
            </Button>
          </>
        }
      >
        <p>{t("nav.deleteProjectBody", { name: confirmDelete?.name ?? "" })}</p>
      </Dialog>

      <FirstRun />
    </div>
  );
}

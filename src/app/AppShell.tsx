import { useEffect, useRef } from "react";
import { NavLink, Outlet, useLocation } from "react-router";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { useUiStore } from "../stores/ui";
import { FOCUSABLE, trapTab } from "../components/focus";
import { IconButton } from "../components/IconButton";
import { projectsApi } from "../ipc/projects";
import { inTauri } from "../ipc/client";
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
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);
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
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            `${styles.topAction} ${isActive ? styles.topActionActive : ""}`
          }
          aria-label={t("nav.settings")}
        >
          <SettingsIcon />
        </NavLink>
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
              <NavLink key={p.id} to={`/p/${p.id}`} className={navItem}>
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
      <FirstRun />
    </div>
  );
}

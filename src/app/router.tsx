import { createHashRouter } from "react-router";
import { AppShell } from "./AppShell";
import { HomePage } from "../features/home/HomePage";
import { FileModifierPage } from "../features/file-modifier/FileModifierPage";
import { SearchPage } from "../features/search/SearchPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { ProjectPage } from "../features/project/ProjectPage";
import { StatesPreview } from "../components/__preview__/StatesPreview";

export const router = createHashRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "file-modifier", element: <FileModifierPage /> },
      { path: "search", element: <SearchPage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "p/:projectId", element: <ProjectPage /> },
      ...(import.meta.env.DEV
        ? [{ path: "__preview/states", element: <StatesPreview /> }]
        : []),
    ],
  },
]);

import { call } from "./client";
import type {
  Project,
  ProjectSummary,
  CreateProjectInput,
  UpdateProjectInput,
} from "./types.gen";

export const projectsApi = {
  list: (includeArchived = false) =>
    call<ProjectSummary[]>("project_list", { includeArchived }),
  create: (input: CreateProjectInput) => call<Project>("project_create", { input }),
  get: (id: string) => call<Project>("project_get", { id }),
  update: (input: UpdateProjectInput) => call<Project>("project_update", { input }),
  setArchived: (id: string, archived: boolean) =>
    call<void>("project_set_archived", { id, archived }),
  open: (id: string) => call<Project>("project_open", { id }),
  delete: (id: string, confirmName: string) =>
    call<void>("project_delete", { id, confirmName }),
};

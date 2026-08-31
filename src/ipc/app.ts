import { call } from "./client";
import type { AppInfo, Job } from "./types.gen";

export const appApi = {
  getInfo: () => call<AppInfo>("app_get_info"),
  openDataDir: () => call<void>("app_open_data_dir"),
  dbHealth: () => call<string>("db_health"),
  jobsList: (projectId?: string) =>
    call<Job[]>("jobs_list", projectId ? { projectId } : {}),
  jobsCancel: (jobId: string) => call<void>("jobs_cancel", { jobId }),
};

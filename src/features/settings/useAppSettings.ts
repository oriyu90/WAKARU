import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { settingsApi } from "../../ipc/settings";
import type { SettingsPatch } from "../../ipc/settings";
import { inTauri } from "../../ipc/client";

const KEY = ["app-settings"];

export function useAppSettings() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: KEY,
    queryFn: settingsApi.get,
    enabled: inTauri,
  });

  const mutate = useMutation({
    mutationFn: (patch: SettingsPatch) => settingsApi.update(patch),
    onSuccess: (next) => qc.setQueryData(KEY, next),
  });

  return {
    settings: query.data,
    isLoading: query.isLoading,
    patch: (p: SettingsPatch) => mutate.mutate(p),
  };
}

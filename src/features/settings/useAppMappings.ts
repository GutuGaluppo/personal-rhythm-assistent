import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listAppMappings, resetAppCategory, setAppCategory } from "@/lib/tauri/commands";
import { MY_DAY_KEY } from "@/features/my-day/useMyDay";
import type { Category } from "@/types";

const KEY = ["app-mappings"] as const;

export function useAppMappings() {
  return useQuery({ queryKey: KEY, queryFn: listAppMappings });
}

/** Remapping changes the session in progress, so My Day is refreshed too. */
function useRefreshAfter() {
  const client = useQueryClient();
  return () =>
    Promise.all([
      client.invalidateQueries({ queryKey: KEY }),
      client.invalidateQueries({ queryKey: MY_DAY_KEY }),
    ]);
}

export function useSetAppCategory() {
  const refresh = useRefreshAfter();
  return useMutation({
    mutationFn: ({ bundleId, category }: { bundleId: string; category: Category }) =>
      setAppCategory(bundleId, category),
    onSuccess: refresh,
  });
}

export function useResetAppCategory() {
  const refresh = useRefreshAfter();
  return useMutation({
    mutationFn: (bundleId: string) => resetAppCategory(bundleId),
    onSuccess: refresh,
  });
}

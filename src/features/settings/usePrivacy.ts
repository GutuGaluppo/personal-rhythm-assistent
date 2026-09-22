import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deleteAllLocalData,
  deleteRawData,
  getDataOverview,
  getPrivacyToggles,
  getRetentionPolicy,
  listRecentActivityEvents,
  setPrivacyToggles,
  setRetentionPolicy,
} from "@/lib/tauri/commands";
import type { PrivacyToggles, RetentionPolicy } from "@/types";

const TOGGLES = ["privacy-toggles"] as const;
const RETENTION = ["retention-policy"] as const;
const OVERVIEW = ["data-overview"] as const;

export const usePrivacyToggles = () => useQuery({ queryKey: TOGGLES, queryFn: getPrivacyToggles });
export const useRetentionPolicy = () =>
  useQuery({ queryKey: RETENTION, queryFn: getRetentionPolicy });
export const useDataOverview = () => useQuery({ queryKey: OVERVIEW, queryFn: getDataOverview });

export const useRecentEvents = (enabled: boolean) =>
  useQuery({
    queryKey: ["recent-activity"],
    queryFn: () => listRecentActivityEvents(20),
    enabled,
  });

export function useSetToggles() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (toggles: PrivacyToggles) => setPrivacyToggles(toggles),
    // Show what was actually stored, not what was asked for.
    onSuccess: (stored) => client.setQueryData(TOGGLES, stored),
  });
}

/** Retention applies at once, so what is stored may have changed. */
export function useSetRetention() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (policy: RetentionPolicy) => setRetentionPolicy(policy),
    onSuccess: async () => {
      await client.invalidateQueries();
    },
  });
}

export function useDeleteRaw() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: deleteRawData,
    onSuccess: async () => {
      await client.invalidateQueries();
    },
  });
}

export function useDeleteAll() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: deleteAllLocalData,
    onSuccess: async () => {
      // Everything on screen may have come from what was just removed. Resetting
      // discards it and reloads what is showing; clearing the cache would leave the
      // mounted screens holding on to the old data.
      await client.resetQueries();
    },
  });
}

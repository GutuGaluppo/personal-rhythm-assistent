import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  addDailyPlanItem,
  deleteDailyPlanItem,
  listDailyPlan,
  toggleDailyPlanItem,
} from "@/lib/tauri/commands";

export const DAILY_PLAN_KEY = ["daily-plan"] as const;

export const useDailyPlan = () => useQuery({ queryKey: DAILY_PLAN_KEY, queryFn: listDailyPlan });

function useRefreshAfter() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: DAILY_PLAN_KEY });
}

export function useAddDailyPlanItem() {
  const refresh = useRefreshAfter();
  return useMutation({ mutationFn: (text: string) => addDailyPlanItem(text), onSuccess: refresh });
}
export function useToggleDailyPlanItem() {
  const refresh = useRefreshAfter();
  return useMutation({ mutationFn: (id: number) => toggleDailyPlanItem(id), onSuccess: refresh });
}
export function useDeleteDailyPlanItem() {
  const refresh = useRefreshAfter();
  return useMutation({ mutationFn: (id: number) => deleteDailyPlanItem(id), onSuccess: refresh });
}

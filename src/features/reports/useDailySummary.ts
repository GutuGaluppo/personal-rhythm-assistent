import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getDailySummary, listSummaryDays, saveReflection } from "@/lib/tauri/commands";

export const summaryKey = (date: string) => ["daily-summary", date] as const;

export const useSummaryDays = () =>
  useQuery({ queryKey: ["summary-days"], queryFn: listSummaryDays });

/** Today's is refreshed as the day goes on; a finished day never changes. */
export const useDailySummary = (date: string, isToday: boolean) =>
  useQuery({
    queryKey: summaryKey(date),
    queryFn: () => getDailySummary(date),
    refetchInterval: isToday ? 30_000 : false,
  });

export function useSaveReflection(date: string) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (text: string) => saveReflection(date, text),
    onSuccess: () => client.invalidateQueries({ queryKey: summaryKey(date) }),
  });
}

import { useQuery } from "@tanstack/react-query";
import { getMyDay } from "@/lib/tauri/commands";

export const MY_DAY_KEY = ["my-day"] as const;

export function useMyDay() {
  return useQuery({ queryKey: MY_DAY_KEY, queryFn: getMyDay, refetchInterval: 15_000 });
}

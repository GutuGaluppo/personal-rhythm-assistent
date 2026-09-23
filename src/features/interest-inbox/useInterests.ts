import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  addInterest,
  archiveInterest,
  deleteInterest,
  getInterestSuggestion,
  listInterests,
  restoreInterest,
} from "@/lib/tauri/commands";

export const INTERESTS_KEY = ["interests"] as const;
export const SUGGESTION_KEY = ["interest-suggestion"] as const;

export const useInterests = (archived: boolean) =>
  useQuery({ queryKey: [...INTERESTS_KEY, archived], queryFn: () => listInterests(archived) });

export const useInterestSuggestion = () =>
  useQuery({ queryKey: SUGGESTION_KEY, queryFn: getInterestSuggestion });

/** Every change can affect both lists and the day's suggestion. */
function useRefreshAfter() {
  const client = useQueryClient();
  return () =>
    Promise.all([
      client.invalidateQueries({ queryKey: INTERESTS_KEY }),
      client.invalidateQueries({ queryKey: SUGGESTION_KEY }),
    ]);
}

export function useAddInterest() {
  const refresh = useRefreshAfter();
  return useMutation({ mutationFn: (text: string) => addInterest(text), onSuccess: refresh });
}
export function useArchiveInterest() {
  const refresh = useRefreshAfter();
  return useMutation({ mutationFn: (id: number) => archiveInterest(id), onSuccess: refresh });
}
export function useRestoreInterest() {
  const refresh = useRefreshAfter();
  return useMutation({ mutationFn: (id: number) => restoreInterest(id), onSuccess: refresh });
}
export function useDeleteInterest() {
  const refresh = useRefreshAfter();
  return useMutation({ mutationFn: (id: number) => deleteInterest(id), onSuccess: refresh });
}

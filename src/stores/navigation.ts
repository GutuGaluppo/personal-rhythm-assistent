import { create } from "zustand";

/** Pages are added here as the milestones that own them land. */
export const PAGES = [
  { id: "my-day", label: "My Day" },
  { id: "settings", label: "Settings" },
] as const;

export type PageId = (typeof PAGES)[number]["id"];

type NavigationState = {
  page: PageId;
  go: (page: PageId) => void;
};

export const useNavigation = create<NavigationState>((set) => ({
  page: "my-day",
  go: (page) => set({ page }),
}));

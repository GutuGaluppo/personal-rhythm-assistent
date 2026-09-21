import { create } from "zustand";

/** Pages are added here as the milestones that own them land. */
const PRODUCT_PAGES = [
  { id: "my-day", label: "My Day" },
  { id: "interest-inbox", label: "Interest Inbox" },
  { id: "history", label: "History" },
  { id: "settings", label: "Settings" },
] as const;

/** Developer-only pages are never offered in a production build. */
const DEVELOPER_PAGES = [{ id: "context-debug", label: "Context (developer)" }] as const;

export type PageId = (typeof PRODUCT_PAGES)[number]["id"] | (typeof DEVELOPER_PAGES)[number]["id"];

export function pagesFor(isDevelopment: boolean): readonly { id: PageId; label: string }[] {
  return isDevelopment ? [...PRODUCT_PAGES, ...DEVELOPER_PAGES] : PRODUCT_PAGES;
}

type NavigationState = {
  page: PageId;
  go: (page: PageId) => void;
};

export const useNavigation = create<NavigationState>((set) => ({
  page: "my-day",
  go: (page) => set({ page }),
}));

export function isPageId(value: unknown): value is PageId {
  return [...PRODUCT_PAGES, ...DEVELOPER_PAGES].some((p) => p.id === value);
}

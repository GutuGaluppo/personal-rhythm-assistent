import { listen } from "@tauri-apps/api/event";

/**
 * The menu bar asks the main window to show a page ("Interest Inbox", "How is my
 * rhythm?"). Resolves to a function that stops listening. Outside the desktop
 * shell (plain browser, tests) there is nothing to listen to, and that is fine.
 */
export async function onNavigate(handler: (page: string) => void): Promise<() => void> {
  try {
    return await listen<string>("navigate", (event) => handler(event.payload));
  } catch {
    return () => {};
  }
}

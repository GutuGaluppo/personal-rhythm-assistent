import { act, screen } from "@testing-library/react";
import { App } from "@/app/App";
import * as commands from "@/lib/tauri/commands";
import * as events from "@/lib/tauri/events";
import { isPageId, pagesFor, useNavigation } from "@/stores/navigation";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");
vi.mock("@/lib/tauri/events");

beforeEach(() => {
  useNavigation.setState({ page: "my-day" });
  vi.mocked(commands.getMyDay).mockResolvedValue({
    trackingEnabled: true,
    state: "active",
    frontmostApplication: null,
    currentSession: null,
    activeMinutesToday: 0,
    contextSwitchesToday: 0,
    lastBreak: null,
  });
  vi.mocked(commands.getInterestSuggestion).mockResolvedValue(null);
  vi.mocked(commands.listInterests).mockResolvedValue([]);
});

describe("menu bar navigation", () => {
  it("shows the page the menu bar asks for", async () => {
    let send: (page: string) => void = () => {};
    vi.mocked(events.onNavigate).mockImplementation(async (handler) => {
      send = handler;
      return () => {};
    });
    renderWithProviders(<App />);
    await vi.waitFor(() => expect(events.onNavigate).toHaveBeenCalled());

    act(() => send("interest-inbox"));
    expect(
      await screen.findByRole("heading", { name: "Interest Inbox", level: 1 }),
    ).toBeInTheDocument();

    act(() => send("my-day"));
    expect(await screen.findByRole("heading", { name: "My Day", level: 1 })).toBeInTheDocument();
  });

  it("ignores a page it does not know", async () => {
    let send: (page: string) => void = () => {};
    vi.mocked(events.onNavigate).mockImplementation(async (handler) => {
      send = handler;
      return () => {};
    });
    renderWithProviders(<App />);
    await vi.waitFor(() => expect(events.onNavigate).toHaveBeenCalled());
    act(() => send("no-such-page"));
    expect(useNavigation.getState().page).toBe("my-day");
  });

  it("stops listening when the app goes away", async () => {
    const unlisten = vi.fn();
    vi.mocked(events.onNavigate).mockResolvedValue(unlisten);
    const { unmount } = renderWithProviders(<App />);
    await vi.waitFor(() => expect(events.onNavigate).toHaveBeenCalled());
    await Promise.resolve();
    unmount();
    expect(unlisten).toHaveBeenCalled();
  });
});

describe("pages", () => {
  it("lists My Day, Interest Inbox, History, Settings, in that order", () => {
    expect(pagesFor(false).map((p) => p.id)).toEqual([
      "my-day",
      "interest-inbox",
      "history",
      "settings",
    ]);
  });

  it("recognises only real pages", () => {
    expect(isPageId("interest-inbox")).toBe(true);
    expect(isPageId("context-debug")).toBe(true);
    expect(isPageId("nope")).toBe(false);
    expect(isPageId(undefined)).toBe(false);
  });
});

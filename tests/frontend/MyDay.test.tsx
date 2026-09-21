import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MyDay } from "@/features/my-day/MyDay";
import * as commands from "@/lib/tauri/commands";
import type { MyDay as MyDayData } from "@/types";
import { useNavigation } from "@/stores/navigation";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const base: MyDayData = {
  trackingEnabled: true,
  state: "active",
  frontmostApplication: "Code",
  currentSession: {
    startedAt: "2026-03-02T09:00:00.000Z",
    elapsedMinutes: 96,
    activeMinutes: 90,
    category: "Create",
    contextSwitches: 7,
  },
  activeMinutesToday: 205,
  contextSwitchesToday: 23,
  lastBreak: { minutes: 25, endedMinutesAgo: 96 },
};

beforeEach(() => {
  vi.mocked(commands.getInterestSuggestion).mockResolvedValue(null);
});

function show(day: Partial<MyDayData> = {}) {
  vi.mocked(commands.getMyDay).mockResolvedValue({ ...base, ...day });
  renderWithProviders(<MyDay />);
}
const card = async (name: string) => within(await screen.findByRole("region", { name }));

describe("My Day", () => {
  it("shows the current session", async () => {
    show();
    const session = await card("Current session");
    expect(await session.findByText("1h 36m")).toBeInTheDocument();
    expect(session.getByText("Create")).toBeInTheDocument();
    expect(session.getByText("7")).toBeInTheDocument();
  });

  it("shows today's totals", async () => {
    show();
    const today = await card("Today");
    expect(await today.findByText("3h 25m")).toBeInTheDocument();
    expect(today.getByText("23")).toBeInTheDocument();
  });

  it("shows where the user is, in words", async () => {
    show({ state: "idle", frontmostApplication: null });
    expect(await screen.findByText("Away")).toBeInTheDocument();
  });

  it("names the frontmost app", async () => {
    show();
    expect(await screen.findByText(/in Code/)).toBeInTheDocument();
  });

  it("describes the last finished break", async () => {
    show();
    const lastBreak = await card("Last real break");
    expect(await lastBreak.findByText(/Ended 1h 36m ago · lasted 25m/)).toBeInTheDocument();
  });

  it("says so when there is no session and a break is in progress", async () => {
    show({
      currentSession: null,
      state: "idle",
      lastBreak: { minutes: 32, endedMinutesAgo: null },
    });
    expect(await screen.findByText("You're on a break.")).toBeInTheDocument();
    expect(
      (await card("Last real break")).getByText(/In progress · 32m so far/),
    ).toBeInTheDocument();
  });

  it("is calm about an empty day", async () => {
    show({ currentSession: null, lastBreak: null, activeMinutesToday: 0, contextSwitchesToday: 0 });
    expect(await screen.findByText("No session right now.")).toBeInTheDocument();
    expect(
      (await card("Last real break")).getByText("None recorded yet today."),
    ).toBeInTheDocument();
  });

  it("explains when tracking is off instead of showing zeros", async () => {
    show({ trackingEnabled: false });
    expect(await screen.findByText(/Active time is switched off/)).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Today" })).not.toBeInTheDocument();
  });

  it("never presents a score", async () => {
    show();
    await screen.findByText("3h 25m");
    expect(document.body.textContent).not.toMatch(/score|streak|goal|points/i);
  });

  it("reports a read failure without alarm styling words", async () => {
    vi.mocked(commands.getMyDay).mockRejectedValue(new Error("boom"));
    renderWithProviders(<MyDay />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't read your day");
  });
});

describe("Take a break", () => {
  it("opens the pause window", async () => {
    vi.mocked(commands.openPause).mockResolvedValue();
    show();
    const button = await screen.findByRole("button", { name: "Take a break" });
    button.click();
    expect(commands.openPause).toHaveBeenCalled();
  });
});

describe("the day's summary", () => {
  it("is one click away from Today", async () => {
    show();
    const today = await card("Today");
    await userEvent.click(today.getByRole("button", { name: "See the day's summary" }));
    expect(useNavigation.getState().page).toBe("history");
  });
});

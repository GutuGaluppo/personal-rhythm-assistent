import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { History } from "@/features/history/History";
import { WeeklyReviewView } from "@/features/reports/WeeklyReviewView";
import * as commands from "@/lib/tauri/commands";
import type { WeeklyReview } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const dates = [
  "2026-03-01",
  "2026-03-02",
  "2026-03-03",
  "2026-03-04",
  "2026-03-05",
  "2026-03-06",
  "2026-03-07",
];

const week: WeeklyReview = {
  from: "2026-03-01",
  to: "2026-03-07",
  activeMinutes: 1500,
  days: dates.map((date, i) => ({
    date,
    activeMinutes: i === 0 ? 0 : 250,
    longestSessionMinutes: i === 0 ? 0 : 194,
    contextSwitches: i === 0 ? 0 : 120,
    sessions: i === 0 ? 0 : 2,
    averageSessionMinutes: i === 0 ? null : 125,
  })),
  categoryDistribution: [
    { category: "Create", minutes: 1050, share: 0.7 },
    { category: "Learn", minutes: 450, share: 0.3 },
  ],
  sessions: {
    count: 12,
    averageMinutes: 125,
    longestMinutes: 194,
    longSessionThresholdMinutes: 90,
    longSessions: 9,
    longSessionDays: 5,
  },
  switching: {
    total: 600,
    perActiveHour: 24,
    busiestDay: { date: "2026-03-03", perActiveHour: 41 },
  },
  checkIns: { shown: 7, accepted: 4, declined: 2, ignored: 1, onFire: 0 },
  observations: [
    {
      id: "top_category",
      text: "Create took the largest share of your active time.",
      evidence: "Create: 17h 30m of 25h active this week (70%).",
    },
    {
      id: "longest_session",
      text: "Your longest session was 3h 14m.",
      evidence: "Longest of 12 sessions; average 2h 5m.",
    },
    {
      id: "check_ins",
      text: "You accepted 4 of 7 break suggestions.",
      evidence: '7 shown: 4 accepted, 2 declined, 1 dismissed or timed out, 0 "I\'m on fire".',
    },
  ],
  reflectiveQuestion: "Does anything here feel different from what you would like?",
};

beforeEach(() => {
  vi.mocked(commands.getWeeklyReview).mockResolvedValue(week);
});

const card = async (name: string) => within(await screen.findByRole("region", { name }));

describe("the weekly review", () => {
  it("states the period", async () => {
    renderWithProviders(<WeeklyReviewView />);
    expect(await screen.findByText(/The last 7 days · Mar 1 – Mar 7/)).toBeInTheDocument();
  });

  it("gives the observations, each with the numbers behind it", async () => {
    renderWithProviders(<WeeklyReviewView />);
    const list = within(await screen.findByRole("list", { name: "What stands out" }));
    expect(list.getByText("Your longest session was 3h 14m.")).toBeInTheDocument();
    expect(list.getByText("You accepted 4 of 7 break suggestions.")).toBeInTheDocument();
    expect(list.getByText("Longest of 12 sessions; average 2h 5m.")).toBeInTheDocument();
    expect(list.getByText(/Create: 17h 30m of 25h active this week \(70%\)/)).toBeInTheDocument();
  });

  it("shows where the time went across the week", async () => {
    renderWithProviders(<WeeklyReviewView />);
    const where = await card("Where the time went");
    const rows = where.getAllByRole("listitem");
    expect(rows[0]).toHaveTextContent("Create");
    expect(rows[0]).toHaveTextContent("17h 30m · 70%");
    expect(rows[1]).toHaveTextContent("7h 30m · 30%");
  });

  it("shows session figures, including the long-session pattern", async () => {
    renderWithProviders(<WeeklyReviewView />);
    const sessions = await card("Sessions");
    expect(sessions.getByText("Average length").nextSibling).toHaveTextContent("2h 5m");
    expect(sessions.getByText("Longest").nextSibling).toHaveTextContent("3h 14m");
    expect(sessions.getByText("Sessions of 1h 30m or more").nextSibling).toHaveTextContent("9");
  });

  it("shows the switching pattern", async () => {
    renderWithProviders(<WeeklyReviewView />);
    const switching = await card("App switching");
    expect(switching.getByText("Switches").nextSibling).toHaveTextContent("600");
    expect(switching.getByText("Per active hour").nextSibling).toHaveTextContent("24");
    expect(switching.getByText("Busiest day").nextSibling).toHaveTextContent("Tue · 41 per hour");
  });

  it("splits the check-ins into what actually happened", async () => {
    renderWithProviders(<WeeklyReviewView />);
    const c = await card("Check-ins");
    expect(c.getByText("Shown").nextSibling).toHaveTextContent("7");
    expect(c.getByText("Took a break").nextSibling).toHaveTextContent("4");
    expect(c.getByText("Chose to continue").nextSibling).toHaveTextContent("2");
    expect(c.getByText("Dismissed or timed out").nextSibling).toHaveTextContent("1");
  });

  it("lays the week out day by day, so a trend can be seen", async () => {
    renderWithProviders(<WeeklyReviewView />);
    const table = await screen.findByRole("table");
    const rows = within(table).getAllByRole("row");
    expect(rows).toHaveLength(8); // header + seven days
    expect(within(rows[1]).getByRole("rowheader")).toHaveTextContent("Sun Mar 1");
    expect(
      within(rows[1])
        .getAllByRole("cell")
        .map((c) => c.textContent),
    ).toEqual(["—", "—", "—", "—"]);
    expect(
      within(rows[2])
        .getAllByRole("cell")
        .map((c) => c.textContent),
    ).toEqual(["4h 10m", "3h 14m", "2h 5m", "120"]);
  });

  it("closes with the question, not a verdict", async () => {
    renderWithProviders(<WeeklyReviewView />);
    expect(
      await screen.findByText("Does anything here feel different from what you would like?"),
    ).toBeInTheDocument();
  });

  it("is honest and calm about a week with nothing in it", async () => {
    vi.mocked(commands.getWeeklyReview).mockResolvedValue({
      ...week,
      activeMinutes: 0,
      categoryDistribution: [],
      checkIns: { shown: 0, accepted: 0, declined: 0, ignored: 0, onFire: 0 },
      observations: [
        {
          id: "check_ins",
          text: "There were no check-ins this week.",
          evidence: "0 check-ins shown.",
        },
      ],
    });
    renderWithProviders(<WeeklyReviewView />);
    expect(await screen.findByText("Nothing recorded this week.")).toBeInTheDocument();
    expect(screen.getByText("There were no check-ins this week.")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Sessions" })).not.toBeInTheDocument();
  });

  it("carries no score, verdict, advice or comparison with other people", async () => {
    renderWithProviders(<WeeklyReviewView />);
    await screen.findByRole("table");
    expect(document.body.textContent).not.toMatch(
      /score|grade|goal|target|streak|productiv|should|must|try to|improve|better|worse|well done|great|average person|most people/i,
    );
  });

  it("reports a read failure calmly", async () => {
    vi.mocked(commands.getWeeklyReview).mockRejectedValue(new Error("boom"));
    renderWithProviders(<WeeklyReviewView />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't read your week");
  });
});

describe("History tabs", () => {
  beforeEach(() => {
    vi.mocked(commands.listSummaryDays).mockResolvedValue(["2026-03-07"]);
    vi.mocked(commands.getDailySummary).mockResolvedValue({
      date: "2026-03-07",
      isFinal: false,
      activeMinutes: 0,
      categoryDistribution: [],
      longestSessionMinutes: 0,
      contextSwitches: 0,
      pausesTaken: 0,
      reflectiveQuestion: "How did today's rhythm feel to you?",
      reflection: null,
    });
  });

  it("opens on the day and offers the week", async () => {
    renderWithProviders(<History />);
    expect(await screen.findByRole("tab", { name: "Day", selected: true })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Week", selected: false })).toBeInTheDocument();
    expect(commands.getWeeklyReview).not.toHaveBeenCalled();
  });

  it("switches to the weekly review", async () => {
    renderWithProviders(<History />);
    await userEvent.click(await screen.findByRole("tab", { name: "Week" }));
    expect(await screen.findByText("Your longest session was 3h 14m.")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Week", selected: true })).toBeInTheDocument();
    expect(screen.getByRole("tabpanel")).toHaveAccessibleName("Week");
  });

  it("can be operated with the arrow keys", async () => {
    renderWithProviders(<History />);
    const day = await screen.findByRole("tab", { name: "Day" });
    day.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Week" })).toHaveFocus();
    expect(screen.getByRole("tab", { name: "Week", selected: true })).toBeInTheDocument();
    await userEvent.keyboard("{ArrowLeft}");
    expect(screen.getByRole("tab", { name: "Day", selected: true })).toBeInTheDocument();
  });

  it("keeps only the selected tab in the tab order", async () => {
    renderWithProviders(<History />);
    expect(await screen.findByRole("tab", { name: "Day" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("tab", { name: "Week" })).toHaveAttribute("tabindex", "-1");
  });
});

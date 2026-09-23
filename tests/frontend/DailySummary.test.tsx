import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DailySummaryView } from "@/features/reports/DailySummaryView";
import { History } from "@/features/history/History";
import * as commands from "@/lib/tauri/commands";
import { formatDate, formatPercent } from "@/lib/utils/format";
import type { DailySummary } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const day: DailySummary = {
  date: "2026-03-04",
  isFinal: false,
  activeMinutes: 312,
  categoryDistribution: [
    { category: "Create", minutes: 210, share: 210 / 312 },
    { category: "Learn", minutes: 72, share: 72 / 312 },
    { category: "Recover", minutes: 30, share: 30 / 312 },
  ],
  longestSessionMinutes: 123,
  contextSwitches: 84,
  pausesTaken: 3,
  reflectiveQuestion: "How did today's rhythm feel to you?",
  reflection: null,
};

beforeEach(() => {
  vi.mocked(commands.getDailySummary).mockResolvedValue(day);
  vi.mocked(commands.saveReflection).mockResolvedValue();
});

const card = async (name: string) => within(await screen.findByRole("region", { name }));

describe("the summary", () => {
  it("shows the facts the spec lists", async () => {
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    const facts = await card("The day");
    expect(facts.getByText("Active time").nextSibling).toHaveTextContent("5h 12m");
    expect(facts.getByText("Longest session").nextSibling).toHaveTextContent("2h 3m");
    expect(facts.getByText("Context switches").nextSibling).toHaveTextContent("84");
    expect(facts.getByText("Pauses taken").nextSibling).toHaveTextContent("3");
  });

  it("shows where the time went, as words and numbers, largest first", async () => {
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    const where = await card("Where the time went");
    const rows = where.getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Create");
    expect(rows[0]).toHaveTextContent("3h 30m · 67%");
    expect(rows[1]).toHaveTextContent("Learn");
    expect(rows[2]).toHaveTextContent("Recover");
  });

  it("never relies on the bars alone", async () => {
    const { container } = renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    await screen.findByRole("region", { name: "Where the time went" });
    const bars = container.querySelectorAll('[aria-hidden="true"]');
    expect(bars.length).toBeGreaterThan(0);
    for (const bar of bars) expect(bar.textContent).toBe("");
  });

  it("says plainly when nothing was recorded", async () => {
    vi.mocked(commands.getDailySummary).mockResolvedValue({
      ...day,
      activeMinutes: 0,
      categoryDistribution: [],
      longestSessionMinutes: 0,
      contextSwitches: 0,
      pausesTaken: 0,
    });
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday={false} />);
    expect(await screen.findByText("Nothing recorded for this day.")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "The day" })).not.toBeInTheDocument();
  });

  it("reports a read failure calmly", async () => {
    vi.mocked(commands.getDailySummary).mockRejectedValue(new Error("boom"));
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't read this day");
  });

  it("carries no score, grade, verdict or comparison", async () => {
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    await screen.findByRole("region", { name: "The day" });
    expect(document.body.textContent).not.toMatch(
      /score|grade|rating|goal|target|streak|productiv|great|good job|well done|too much|too little|should|better|worse|than yesterday/i,
    );
  });

  it("refreshes today's figures as the day goes on, but not a finished day's", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
      await screen.findByRole("region", { name: "The day" });
      await vi.advanceTimersByTimeAsync(31_000);
      expect(vi.mocked(commands.getDailySummary).mock.calls.length).toBeGreaterThan(1);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("the reflective question", () => {
  it("is the one the spec gives, and optional", async () => {
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    const box = await screen.findByLabelText("How did today's rhythm feel to you?");
    expect(box).toHaveValue("");
    expect(screen.getByText(/Optional/)).toBeInTheDocument();
    expect(screen.getByText(/stays on this device/)).toBeInTheDocument();
  });

  it("saves what the user writes", async () => {
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    await userEvent.type(await screen.findByLabelText(/rhythm feel/), "Steady, mostly.");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(commands.saveReflection).toHaveBeenCalledWith("2026-03-04", "Steady, mostly.");
    expect(await screen.findByText("Saved.")).toBeInTheDocument();
  });

  it("shows a saved reflection and only offers Save once it changes", async () => {
    vi.mocked(commands.getDailySummary).mockResolvedValue({ ...day, reflection: "Calm." });
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    const box = await screen.findByLabelText(/rhythm feel/);
    expect(box).toHaveValue("Calm.");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await userEvent.type(box, " More.");
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
  });

  it("can be cleared by saving nothing", async () => {
    vi.mocked(commands.getDailySummary).mockResolvedValue({ ...day, reflection: "Calm." });
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    await userEvent.clear(await screen.findByLabelText(/rhythm feel/));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(commands.saveReflection).toHaveBeenCalledWith("2026-03-04", "");
  });

  it("is bounded", async () => {
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    expect(await screen.findByLabelText(/rhythm feel/)).toHaveAttribute("maxlength", "500");
  });

  it("is offered even on a day with nothing recorded", async () => {
    vi.mocked(commands.getDailySummary).mockResolvedValue({
      ...day,
      activeMinutes: 0,
      categoryDistribution: [],
      pausesTaken: 0,
    });
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    expect(await screen.findByLabelText(/rhythm feel/)).toBeInTheDocument();
  });

  it("says when it could not save", async () => {
    vi.mocked(commands.saveReflection).mockRejectedValue(new Error("disk"));
    renderWithProviders(<DailySummaryView date="2026-03-04" isToday />);
    await userEvent.type(await screen.findByLabelText(/rhythm feel/), "x");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't save that");
  });
});

describe("History", () => {
  const days = ["2026-03-06", "2026-03-04", "2026-03-03"];
  beforeEach(() => {
    vi.mocked(commands.listSummaryDays).mockResolvedValue(days);
    vi.mocked(commands.getDailySummary).mockImplementation(async (date) => ({
      ...day,
      date: date ?? days[0],
    }));
  });

  it("opens on today", async () => {
    renderWithProviders(<History />);
    expect(
      await screen.findByRole("heading", { level: 2, name: /today so far/ }),
    ).toHaveTextContent(formatDate("2026-03-06"));
    expect(commands.getDailySummary).toHaveBeenCalledWith("2026-03-06");
  });

  it("steps back through the days that have something to show", async () => {
    renderWithProviders(<History />);
    await screen.findByRole("heading", { level: 2 });
    await userEvent.click(screen.getByRole("button", { name: "Earlier day" }));
    expect(
      await screen.findByRole("heading", { level: 2, name: formatDate("2026-03-04") }),
    ).toBeInTheDocument();
    expect(commands.getDailySummary).toHaveBeenCalledWith("2026-03-04");
    expect(screen.queryByText(/today so far/)).not.toBeInTheDocument();
  });

  it("stops at both ends", async () => {
    renderWithProviders(<History />);
    await screen.findByRole("heading", { level: 2 });
    expect(screen.getByRole("button", { name: "Later day" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Earlier day" }));
    await userEvent.click(screen.getByRole("button", { name: "Earlier day" }));
    expect(screen.getByRole("button", { name: "Earlier day" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Later day" }));
    expect(screen.getByRole("button", { name: "Later day" })).toBeEnabled();
  });

  it("starts each day's reflection afresh", async () => {
    vi.mocked(commands.getDailySummary).mockImplementation(async (date) => ({
      ...day,
      date: date ?? "",
      reflection: date === "2026-03-04" ? "That day." : null,
    }));
    renderWithProviders(<History />);
    const box = await screen.findByLabelText(/rhythm feel/);
    expect(box).toHaveValue("");
    await userEvent.type(box, "unsaved draft");
    await userEvent.click(screen.getByRole("button", { name: "Earlier day" }));
    await vi.waitFor(() => expect(screen.getByLabelText(/rhythm feel/)).toHaveValue("That day."));
  });

  it("reports a read failure calmly", async () => {
    vi.mocked(commands.listSummaryDays).mockRejectedValue(new Error("boom"));
    renderWithProviders(<History />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't read your history");
  });
});

describe("formatting", () => {
  it("rounds percentages", () => {
    expect(formatPercent(0.6666)).toBe("67%");
    expect(formatPercent(0)).toBe("0%");
    expect(formatPercent(1)).toBe("100%");
  });

  it("formats a local date without shifting the day", () => {
    expect(formatDate("2026-03-04")).toBe("Wednesday, March 4");
    expect(formatDate("2026-01-01")).toBe("Thursday, January 1");
  });
});

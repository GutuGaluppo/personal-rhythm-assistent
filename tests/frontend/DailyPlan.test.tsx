import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DailyPlan } from "@/features/my-day/DailyPlan";
import * as commands from "@/lib/tauri/commands";
import type { DailyPlanItem } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const writeTests: DailyPlanItem = {
  id: 1,
  day: "2026-03-04",
  text: "Write tests",
  createdAt: "2026-03-04T09:00:00.000Z",
  doneAt: null,
};
const standup: DailyPlanItem = {
  id: 2,
  day: "2026-03-04",
  text: "Stand-up",
  createdAt: "2026-03-04T08:00:00.000Z",
  doneAt: "2026-03-04T08:15:00.000Z",
};

beforeEach(() => {
  vi.mocked(commands.listDailyPlan).mockResolvedValue([standup, writeTests]);
  vi.mocked(commands.addDailyPlanItem).mockImplementation(async (text) => ({
    id: 99,
    day: "2026-03-04",
    text,
    createdAt: "2026-03-04T10:00:00.000Z",
    doneAt: null,
  }));
  vi.mocked(commands.toggleDailyPlanItem).mockResolvedValue({ ...writeTests, doneAt: null });
  vi.mocked(commands.deleteDailyPlanItem).mockResolvedValue(true);
});

describe("adding", () => {
  it("takes a thought with typing and one Enter", async () => {
    renderWithProviders(<DailyPlan />);
    await screen.findByRole("list", { name: "Today's plan" });
    await userEvent.type(
      screen.getByLabelText("Add something to today's plan"),
      "Ship the release{Enter}",
    );
    expect(commands.addDailyPlanItem).toHaveBeenCalledWith("Ship the release");
  });

  it("has an Add button too, disabled when empty", async () => {
    renderWithProviders(<DailyPlan />);
    const button = screen.getByRole("button", { name: "Add" });
    expect(button).toBeDisabled();
    await userEvent.type(screen.getByLabelText("Add something to today's plan"), "Two");
    expect(button).not.toBeDisabled();
    await userEvent.click(button);
    expect(commands.addDailyPlanItem).toHaveBeenCalledWith("Two");
  });

  it("ignores an empty submission", async () => {
    renderWithProviders(<DailyPlan />);
    await screen.findByRole("list", { name: "Today's plan" });
    await userEvent.type(screen.getByLabelText("Add something to today's plan"), "   {Enter}");
    expect(commands.addDailyPlanItem).not.toHaveBeenCalled();
  });

  it("limits the length", async () => {
    renderWithProviders(<DailyPlan />);
    expect(await screen.findByLabelText("Add something to today's plan")).toHaveAttribute(
      "maxlength",
      "280",
    );
  });
});

describe("the list", () => {
  it("shows each item, checked state matching whether it is done", async () => {
    renderWithProviders(<DailyPlan />);
    const list = await screen.findByRole("list", { name: "Today's plan" });
    const items = within(list).getAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(screen.getByRole("checkbox", { name: "Stand-up" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Write tests" })).not.toBeChecked();
  });

  it("is calm when empty", async () => {
    vi.mocked(commands.listDailyPlan).mockResolvedValue([]);
    renderWithProviders(<DailyPlan />);
    expect(await screen.findByText(/Nothing planned yet/)).toBeInTheDocument();
  });

  it("toggles done with a click", async () => {
    renderWithProviders(<DailyPlan />);
    await userEvent.click(await screen.findByRole("checkbox", { name: "Write tests" }));
    expect(commands.toggleDailyPlanItem).toHaveBeenCalledWith(1);
  });

  it("deletes with one click, no confirmation needed", async () => {
    renderWithProviders(<DailyPlan />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete Write tests" }));
    expect(commands.deleteDailyPlanItem).toHaveBeenCalledWith(1);
  });

  it("never scores, ranks or nags", async () => {
    renderWithProviders(<DailyPlan />);
    await screen.findByRole("list", { name: "Today's plan" });
    expect(document.body.textContent).not.toMatch(/score|streak|priority|overdue|goal|points/i);
  });
});

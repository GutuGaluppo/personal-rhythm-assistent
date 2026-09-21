import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { isInterventionWindow } from "@/app/root";
import { InterventionWindow } from "@/features/intervention/InterventionWindow";
import * as commands from "@/lib/tauri/commands";
import type { InterventionView } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const energy: InterventionView = {
  id: "intervention-1",
  headline: "You've been active for 96 minutes.",
  question: "How is your energy right now?",
  reasons: [
    "Active for 96 minutes in this session.",
    "0 minutes without input in a 96-minute session.",
  ],
  step: "energy",
};
const positive: InterventionView = { ...energy, step: "positive" };
const low: InterventionView = { ...energy, step: "low" };

beforeEach(() => {
  vi.mocked(commands.getCurrentIntervention).mockResolvedValue(energy);
  vi.mocked(commands.answerIntervention).mockResolvedValue(null);
  vi.mocked(commands.dismissIntervention).mockResolvedValue();
});

async function open() {
  renderWithProviders(<InterventionWindow />);
  await screen.findByRole("dialog");
}

describe("the check-in", () => {
  it("asks how the user's energy is", async () => {
    await open();
    expect(screen.getByText("You've been active for 96 minutes.")).toBeInTheDocument();
    expect(screen.getByText("How is your energy right now?")).toBeInTheDocument();
    for (const name of ["Good", "Okay", "Low"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
  });

  it("always shows why it appeared", async () => {
    await open();
    expect(screen.getByText("Why you're seeing this")).toBeInTheDocument();
    expect(screen.getByText("Active for 96 minutes in this session.")).toBeVisible();
    expect(screen.getByRole("dialog")).toHaveAccessibleDescription(/Active for 96 minutes/);
  });

  it("is a non-modal, labelled dialog", async () => {
    await open();
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("aria-modal", "false");
    expect(dialog).toHaveAccessibleName("You've been active for 96 minutes.");
  });

  it("renders nothing when there is no check-in to show", async () => {
    vi.mocked(commands.getCurrentIntervention).mockResolvedValue(null);
    renderWithProviders(<InterventionWindow />);
    await vi.waitFor(() => expect(commands.getCurrentIntervention).toHaveBeenCalled());
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

describe("answering", () => {
  it("Good leads to Continue / Take a break", async () => {
    vi.mocked(commands.answerIntervention).mockResolvedValueOnce(positive);
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Good" }));

    expect(commands.answerIntervention).toHaveBeenCalledWith("intervention-1", {
      kind: "energy",
      value: "good",
    });
    expect(await screen.findByRole("button", { name: "Continue" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Take a break" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Good" })).not.toBeInTheDocument();
  });

  it("Okay is answered as okay", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Okay" }));
    expect(commands.answerIntervention).toHaveBeenCalledWith("intervention-1", {
      kind: "energy",
      value: "okay",
    });
  });

  it("Low leads to Move / Meditate / Do nothing", async () => {
    vi.mocked(commands.answerIntervention).mockResolvedValueOnce(low);
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Low" }));
    for (const name of ["Move", "Meditate", "Do nothing"]) {
      expect(await screen.findByRole("button", { name })).toBeInTheDocument();
    }
  });

  it.each([
    ["Continue", positive, { kind: "action", value: "continue" }],
    ["Take a break", positive, { kind: "action", value: "take_break" }],
    ["Move", low, { kind: "action", value: "move" }],
    ["Meditate", low, { kind: "action", value: "meditate" }],
    ["Do nothing", low, { kind: "action", value: "do_nothing" }],
  ])("%s sends its action and ends the check-in", async (label, view, expected) => {
    vi.mocked(commands.getCurrentIntervention).mockResolvedValue(view);
    await open();
    await userEvent.click(screen.getByRole("button", { name: label }));
    expect(commands.answerIntervention).toHaveBeenCalledWith("intervention-1", expected);
    await vi.waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("offers I'm on fire after a good answer, and only there", async () => {
    await open();
    expect(screen.queryByRole("button", { name: "I'm on fire" })).not.toBeInTheDocument();
  });

  it("sends I'm on fire from the follow-up step", async () => {
    vi.mocked(commands.getCurrentIntervention).mockResolvedValue(positive);
    await open();
    await userEvent.click(screen.getByRole("button", { name: "I'm on fire" }));
    expect(commands.answerIntervention).toHaveBeenCalledWith("intervention-1", {
      kind: "action",
      value: "on_fire",
    });
  });

  it.each([
    ["energy", energy],
    ["positive", positive],
    ["low", low],
  ])("Leave me alone is available on the %s step", async (_name, view) => {
    vi.mocked(commands.getCurrentIntervention).mockResolvedValue(view);
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Leave me alone for now" }));
    expect(commands.answerIntervention).toHaveBeenCalledWith("intervention-1", {
      kind: "leave_me_alone",
    });
  });
});

describe("dismissing", () => {
  it("takes one click", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(commands.dismissIntervention).toHaveBeenCalledTimes(1);
    expect(commands.dismissIntervention).toHaveBeenCalledWith("intervention-1");
    await vi.waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("works on every step", async () => {
    vi.mocked(commands.getCurrentIntervention).mockResolvedValue(low);
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(commands.dismissIntervention).toHaveBeenCalledTimes(1);
  });

  it("works with Escape", async () => {
    await open();
    await userEvent.keyboard("{Escape}");
    expect(commands.dismissIntervention).toHaveBeenCalledWith("intervention-1");
  });
});

describe("keyboard and accessibility", () => {
  it("focuses the first action when it appears", async () => {
    await open();
    expect(screen.getByRole("button", { name: "Good" })).toHaveFocus();
  });

  it("every action is reachable with Tab and activated with Enter", async () => {
    vi.mocked(commands.answerIntervention).mockResolvedValueOnce(positive);
    await open();

    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Okay" })).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Low" })).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Leave me alone for now" })).toHaveFocus();

    await userEvent.tab({ shift: true });
    await userEvent.tab({ shift: true });
    await userEvent.tab({ shift: true });
    expect(screen.getByRole("button", { name: "Good" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(commands.answerIntervention).toHaveBeenCalledWith("intervention-1", {
      kind: "energy",
      value: "good",
    });
  });

  it("moves focus to the first action of the next step", async () => {
    vi.mocked(commands.answerIntervention).mockResolvedValueOnce(positive);
    await open();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByRole("button", { name: "Continue" })).toHaveFocus();
  });
});

describe("tone", () => {
  it("shows no countdown, urgency or score", async () => {
    await open();
    const text = document.body.textContent ?? "";
    expect(text).not.toMatch(/\d+\s*(s|sec|seconds)\b/i);
    expect(text).not.toMatch(/!|urgent|hurry|score|streak|warning/i);
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
    expect(screen.queryByRole("timer")).not.toBeInTheDocument();
  });
});

describe("window selection", () => {
  it("recognises the check-in window by its URL", () => {
    expect(isInterventionWindow("?view=intervention")).toBe(true);
    expect(isInterventionWindow("")).toBe(false);
    expect(isInterventionWindow("?view=other")).toBe(false);
  });
});

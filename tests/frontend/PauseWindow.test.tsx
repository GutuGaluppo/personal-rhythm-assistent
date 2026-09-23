import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PauseWindow, PROMPTS } from "@/features/pause/PauseWindow";
import * as commands from "@/lib/tauri/commands";
import { writeMotionPreference } from "@/lib/utils/motion";
import type { PauseView } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const T0 = new Date("2026-03-04T10:00:00.000Z");
const minutes = (n: number) => new Date(T0.getTime() + n * 60_000).toISOString();

const setup: PauseView = {
  phase: "setup",
  kind: "meditation",
  startedAt: null,
  endsAt: null,
  durationSeconds: null,
  reason: null,
};
const running = (durationMin: number, kind: PauseView["kind"] = "meditation"): PauseView => ({
  phase: "running",
  kind,
  startedAt: T0.toISOString(),
  endsAt: minutes(durationMin),
  durationSeconds: durationMin * 60,
  reason: null,
});

beforeEach(() => {
  window.localStorage.clear();
  vi.mocked(commands.getPauseView).mockResolvedValue(setup);
  vi.mocked(commands.startPause).mockImplementation(async (kind, m) => ({
    ...running(m, kind),
  }));
  vi.mocked(commands.endPause).mockResolvedValue();
  vi.mocked(commands.setPauseReason).mockImplementation(async (reason) => ({
    ...running(5),
    reason,
  }));
});
afterEach(() => {
  vi.useRealTimers();
});

async function openAt(view: PauseView) {
  vi.mocked(commands.getPauseView).mockResolvedValue(view);
  renderWithProviders(<PauseWindow />);
  await screen.findByRole("main");
}

describe("setup", () => {
  it("preselects the kind suggested by the check-in", async () => {
    await openAt({ ...setup, kind: "walking" });
    expect(screen.getByRole("radio", { name: "Walk" })).toBeChecked();
    expect(screen.getByText(PROMPTS.walking)).toBeInTheDocument();
  });

  it("offers Nothing, Meditate, Walk and Stretch", async () => {
    await openAt(setup);
    for (const name of ["Nothing", "Meditate", "Walk", "Stretch"]) {
      expect(screen.getByRole("radio", { name })).toBeInTheDocument();
    }
  });

  it("offers 3, 5 and 10 minutes and a custom length, 5 by default", async () => {
    await openAt(setup);
    expect(screen.getByRole("radio", { name: "5 min" })).toBeChecked();
    for (const name of ["3 min", "10 min", "Custom"]) {
      expect(screen.getByRole("radio", { name })).toBeInTheDocument();
    }
  });

  it("uses the words the spec gives for meditation", async () => {
    await openAt(setup);
    expect(
      screen.getByText("Close your eyes for a moment and follow your breathing."),
    ).toBeVisible();
  });

  it("starts the pause the user set up", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true, now: T0 });
    await openAt(setup);
    await userEvent.click(screen.getByRole("radio", { name: "Stretch" }));
    await userEvent.click(screen.getByRole("radio", { name: "10 min" }));
    await userEvent.click(screen.getByRole("button", { name: "Start" }));
    expect(commands.startPause).toHaveBeenCalledWith("stretching", 10, null);
    expect(await screen.findByRole("timer")).toBeInTheDocument();
  });

  it("accepts a custom length", async () => {
    await openAt(setup);
    await userEvent.click(screen.getByRole("radio", { name: "Custom" }));
    const input = screen.getByLabelText("Minutes");
    await userEvent.clear(input);
    await userEvent.type(input, "17");
    await userEvent.click(screen.getByRole("button", { name: "Start" }));
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 17, null);
  });

  it.each(["0", "61", "", "2.5"])("refuses a custom length of %j", async (value) => {
    await openAt(setup);
    await userEvent.click(screen.getByRole("radio", { name: "Custom" }));
    const input = screen.getByLabelText("Minutes");
    await userEvent.clear(input);
    if (value) await userEvent.type(input, value);
    expect(screen.getByRole("button", { name: "Start" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("between 1 and 60");
  });

  it("has Start focused, so Enter is enough", async () => {
    await openAt(setup);
    expect(screen.getByRole("button", { name: "Start" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 5, null);
  });

  it("can be declined without any fuss", async () => {
    await openAt(setup);
    await userEvent.click(screen.getByRole("button", { name: "Not now" }));
    expect(commands.endPause).toHaveBeenCalled();
    expect(commands.startPause).not.toHaveBeenCalled();
  });

  it("Escape declines too", async () => {
    await openAt(setup);
    await userEvent.keyboard("{Escape}");
    expect(commands.endPause).toHaveBeenCalled();
  });
});

describe("reason", () => {
  it("is optional and omitted by default", async () => {
    await openAt(setup);
    await userEvent.click(screen.getByRole("button", { name: "Start" }));
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 5, null);
  });

  it("a preset chip is sent as the reason", async () => {
    await openAt(setup);
    await userEvent.click(screen.getByRole("button", { name: "Lunch" }));
    await userEvent.click(screen.getByRole("button", { name: "Start" }));
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 5, {
      kind: "lunch",
      note: null,
    });
  });

  it("clicking the same chip again clears it", async () => {
    await openAt(setup);
    const lunch = screen.getByRole("button", { name: "Lunch" });
    await userEvent.click(lunch);
    await userEvent.click(lunch);
    await userEvent.click(screen.getByRole("button", { name: "Start" }));
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 5, null);
  });

  it("Other needs a note before Start is enabled", async () => {
    await openAt(setup);
    await userEvent.click(screen.getByRole("button", { name: "Other" }));
    expect(screen.getByRole("button", { name: "Start" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent(/other/i);

    await userEvent.type(screen.getByLabelText("What's up?"), "waiting for a delivery");
    expect(screen.getByRole("button", { name: "Start" })).not.toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Start" }));
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 5, {
      kind: "other",
      note: "waiting for a delivery",
    });
  });
});

describe("running", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true, now: T0 });
  });

  it("shows the prompt and a calm remaining time, never ticking seconds", async () => {
    await openAt(running(5));
    expect(screen.getByText(PROMPTS.meditation)).toBeInTheDocument();
    expect(screen.getByRole("timer")).toHaveTextContent("About 5 min left");
    expect(document.body.textContent).not.toMatch(/\d+:\d\d/);
  });

  it("counts down from the deadline", async () => {
    await openAt(running(5));
    act(() => {
      vi.setSystemTime(new Date(T0.getTime() + 2 * 60_000 + 10_000));
      vi.advanceTimersByTime(600);
    });
    expect(screen.getByRole("timer")).toHaveTextContent("About 3 min left");
    act(() => {
      vi.setSystemTime(new Date(T0.getTime() + 4 * 60_000 + 30_000));
      vi.advanceTimersByTime(600);
    });
    expect(screen.getByRole("timer")).toHaveTextContent("Less than a minute left");
  });

  it("survives losing focus: the display catches up from the deadline the moment it is shown again", async () => {
    await openAt(running(5));
    expect(screen.getByRole("timer")).toHaveTextContent("About 5 min left");

    // The window goes to the background and its timers are throttled to nothing.
    // Three minutes pass without a single tick...
    vi.setSystemTime(new Date(T0.getTime() + 3 * 60_000));
    expect(screen.getByRole("timer")).toHaveTextContent("About 5 min left");

    // ...and when the window comes back, it is exactly right at once.
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(screen.getByRole("timer")).toHaveTextContent("About 2 min left");
  });

  it("also catches up when the window becomes visible again", async () => {
    await openAt(running(10));
    vi.setSystemTime(new Date(T0.getTime() + 6 * 60_000));
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    expect(screen.getByRole("timer")).toHaveTextContent("About 4 min left");
  });

  it("finishes even if the window was hidden for the whole pause", async () => {
    await openAt(running(3));
    vi.setSystemTime(new Date(T0.getTime() + 30 * 60_000));
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(await screen.findByRole("heading", { name: "Welcome back." })).toBeInTheDocument();
  });

  it("can be left early with no judgement", async () => {
    await openAt(running(5));
    await userEvent.click(screen.getByRole("button", { name: "Return now" }));
    expect(commands.endPause).toHaveBeenCalled();
    expect(document.body.textContent).not.toMatch(/give up|failed|only|quit/i);
  });

  it("a reason can be tagged mid-pause without ending it", async () => {
    await openAt(running(5));
    await userEvent.click(screen.getByRole("button", { name: "Call" }));
    expect(commands.setPauseReason).toHaveBeenCalledWith({ kind: "call", note: null });
    expect(commands.endPause).not.toHaveBeenCalled();
  });
});

describe("quick pause (untimed)", () => {
  const untimed = (kind: PauseView["kind"] = "silence"): PauseView => ({
    phase: "running",
    kind,
    startedAt: T0.toISOString(),
    endsAt: null,
    durationSeconds: null,
    reason: null,
  });

  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true, now: T0 });
  });

  it("shows elapsed time, not a countdown, and skips setup entirely", async () => {
    await openAt(untimed());
    expect(screen.getByRole("heading", { name: "You're on pause." })).toBeInTheDocument();
    expect(screen.queryByRole("timer")).not.toBeInTheDocument();
  });

  it("never becomes done on its own, however long it runs", async () => {
    await openAt(untimed());
    act(() => {
      vi.setSystemTime(new Date(T0.getTime() + 90 * 60_000));
      vi.advanceTimersByTime(600);
    });
    expect(screen.getByText("1h 30m so far")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Welcome back." })).not.toBeInTheDocument();
  });

  it("Back to work is the primary way out", async () => {
    await openAt(untimed());
    await userEvent.click(screen.getByRole("button", { name: "Back to work" }));
    expect(commands.endPause).toHaveBeenCalled();
  });
});

describe("resync on reopen", () => {
  it("shows the new pause instead of staying blank when the window is reused", async () => {
    // The window is hidden and reused between pauses rather than rebuilt (so
    // reopening is instant): once a pause ends, the window goes quiet...
    await openAt(setup);
    await userEvent.click(screen.getByRole("button", { name: "Not now" }));
    await vi.waitFor(() => expect(screen.queryByRole("main")).not.toBeInTheDocument());

    // ...and when it is shown again for a new pause, it must not sit there
    // blank until the 5s safety-net poll happens to fire.
    vi.mocked(commands.getPauseView).mockResolvedValue({ ...setup, kind: "walking" });
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(await screen.findByRole("radio", { name: "Walk" })).toBeChecked();
  });
});

describe("returning", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true, now: new Date(T0.getTime() + 10 * 60_000) });
  });

  it("welcomes the user back and focuses the one way forward", async () => {
    await openAt(running(3));
    expect(screen.getByRole("heading", { name: "Welcome back." })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back to work" })).toHaveFocus();
  });

  it("Back to work ends the pause", async () => {
    await openAt(running(3));
    await userEvent.click(screen.getByRole("button", { name: "Back to work" }));
    expect(commands.endPause).toHaveBeenCalledTimes(1);
  });

  it("Enter returns", async () => {
    await openAt(running(3));
    await userEvent.keyboard("{Enter}");
    expect(commands.endPause).toHaveBeenCalledTimes(1);
  });

  it("Escape returns", async () => {
    await openAt(running(3));
    fireEvent.keyDown(document, { key: "Escape" });
    expect(commands.endPause).toHaveBeenCalledTimes(1);
  });

  it("asks nothing further and shows no summary or score", async () => {
    await openAt(running(3));
    expect(screen.getAllByRole("button")).toHaveLength(1);
    expect(document.body.textContent).not.toMatch(/score|streak|goal|points|great job|well done/i);
  });
});

describe("motion", () => {
  it("uses the full transformation by default", async () => {
    await openAt(setup);
    expect(screen.getByRole("img")).toHaveAttribute("data-motion", "morph");
  });

  it("uses a crossfade when Reduce Motion is on", async () => {
    writeMotionPreference("reduce");
    await openAt(setup);
    expect(screen.getByRole("img")).toHaveAttribute("data-motion", "crossfade");
  });
});

describe("nothing to show", () => {
  it("renders nothing when there is no pause", async () => {
    vi.mocked(commands.getPauseView).mockResolvedValue(null);
    renderWithProviders(<PauseWindow />);
    await vi.waitFor(() => expect(commands.getPauseView).toHaveBeenCalled());
    expect(screen.queryByRole("main")).not.toBeInTheDocument();
  });
});

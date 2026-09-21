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
};
const running = (durationMin: number, kind: PauseView["kind"] = "meditation"): PauseView => ({
  phase: "running",
  kind,
  startedAt: T0.toISOString(),
  endsAt: minutes(durationMin),
  durationSeconds: durationMin * 60,
});

beforeEach(() => {
  window.localStorage.clear();
  vi.mocked(commands.getPauseView).mockResolvedValue(setup);
  vi.mocked(commands.startPause).mockImplementation(async (kind, m) => ({
    ...running(m, kind),
  }));
  vi.mocked(commands.endPause).mockResolvedValue();
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
    expect(commands.startPause).toHaveBeenCalledWith("stretching", 10);
    expect(await screen.findByRole("timer")).toBeInTheDocument();
  });

  it("accepts a custom length", async () => {
    await openAt(setup);
    await userEvent.click(screen.getByRole("radio", { name: "Custom" }));
    const input = screen.getByLabelText("Minutes");
    await userEvent.clear(input);
    await userEvent.type(input, "17");
    await userEvent.click(screen.getByRole("button", { name: "Start" }));
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 17);
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
    expect(commands.startPause).toHaveBeenCalledWith("meditation", 5);
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

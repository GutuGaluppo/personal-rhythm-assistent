import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PermissionsNote } from "@/features/settings/PermissionsNote";
import { PrivacySettings } from "@/features/settings/PrivacySettings";
import { RetentionSettings } from "@/features/settings/RetentionSettings";
import { Settings } from "@/features/settings/Settings";
import { YourData, describeEvent } from "@/features/settings/YourData";
import * as commands from "@/lib/tauri/commands";
import { formatBytes } from "@/lib/utils/format";
import type { DataOverview } from "@/types";
import { defaultToggles, emptyOverview, mockSettingsCommands } from "./settingsMocks";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

beforeEach(() => {
  window.localStorage.clear();
  mockSettingsCommands();
});

// ---- privacy toggles ------------------------------------------------------------------

describe("privacy toggles", () => {
  it("has a labelled switch for each thing that is collected", async () => {
    renderWithProviders(<PrivacySettings />);
    for (const name of [/Active application/, /Active time/, /Idle detection/]) {
      expect(await screen.findByRole("switch", { name })).toBeChecked();
    }
  });

  it("says in words whether each is on or off", async () => {
    vi.mocked(commands.getPrivacyToggles).mockResolvedValue({
      ...defaultToggles,
      idleDetection: false,
    });
    renderWithProviders(<PrivacySettings />);
    const idle = await screen.findByRole("switch", { name: /Idle detection/ });
    expect(idle).not.toBeChecked();
    expect(within(idle.closest("label")!).getByText("Off")).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: /Active application/ }).closest("label"),
    ).toHaveTextContent("On");
  });

  it("turns a sensor off with one click, changing only that one", async () => {
    renderWithProviders(<PrivacySettings />);
    await userEvent.click(await screen.findByRole("switch", { name: /Idle detection/ }));
    expect(commands.setPrivacyToggles).toHaveBeenCalledWith({
      ...defaultToggles,
      idleDetection: false,
    });
  });

  it("can turn a sensor back on", async () => {
    vi.mocked(commands.getPrivacyToggles).mockResolvedValue({
      ...defaultToggles,
      activeApplication: false,
    });
    renderWithProviders(<PrivacySettings />);
    await userEvent.click(await screen.findByRole("switch", { name: /Active application/ }));
    expect(commands.setPrivacyToggles).toHaveBeenCalledWith({
      ...defaultToggles,
      activeApplication: true,
    });
  });

  it("explains that turning one off stops collection but keeps what is stored", async () => {
    vi.mocked(commands.getPrivacyToggles).mockResolvedValue({
      ...defaultToggles,
      activeTime: false,
    });
    renderWithProviders(<PrivacySettings />);
    expect(
      await screen.findByText(/Nothing new is collected\. What is already stored stays/),
    ).toBeInTheDocument();
  });

  it("reflects what was actually stored, not what was asked for", async () => {
    vi.mocked(commands.setPrivacyToggles).mockResolvedValue(defaultToggles); // the change did not stick
    renderWithProviders(<PrivacySettings />);
    const box = await screen.findByRole("switch", { name: /Idle detection/ });
    await userEvent.click(box);
    await vi.waitFor(() => expect(commands.setPrivacyToggles).toHaveBeenCalled());
    expect(box).toBeChecked();
  });

  it("says plainly when a change could not be saved", async () => {
    vi.mocked(commands.setPrivacyToggles).mockRejectedValue(new Error("disk"));
    renderWithProviders(<PrivacySettings />);
    await userEvent.click(await screen.findByRole("switch", { name: /Idle detection/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't save that change");
  });
});

describe("what each thing tells you", () => {
  it("gives what is collected, why, where, for how long and how to delete it", async () => {
    renderWithProviders(<PrivacySettings />);
    await screen.findByRole("switch", { name: /Active application/ });
    for (const heading of [
      "What is collected",
      "Why",
      "Where it is stored",
      "How long it is kept",
      "How to delete it",
    ]) {
      expect(screen.getAllByText(heading)).toHaveLength(3); // one per thing collected
    }
  });

  it("states the retention that is actually set", async () => {
    vi.mocked(commands.getRetentionPolicy).mockResolvedValue({
      activityEventsDays: 3,
      sessionsDays: 1,
      dailySummariesDays: null,
    });
    renderWithProviders(<PrivacySettings />);
    await screen.findByRole("switch", { name: /Active application/ });
    await vi.waitFor(() => expect(screen.getAllByText("3 days")).toHaveLength(2)); // events + idle
    expect(screen.getByText("1 day")).toBeInTheDocument(); // sessions
  });

  it("says the data stays on this device and never sends anything", async () => {
    renderWithProviders(<PrivacySettings />);
    expect(
      await screen.findByText(/Everything stays on this device\. Nothing is sent anywhere/),
    ).toBeInTheDocument();
    expect(await screen.findAllByText(/On this device only/)).toHaveLength(3);
  });

  it("says what is never collected", async () => {
    renderWithProviders(<PrivacySettings />);
    expect(
      await screen.findByText(/Never window titles, documents, or anything you type/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Only the elapsed time, never which keys or where the pointer was/),
    ).toBeInTheDocument();
  });
});

describe("features that are not built yet", () => {
  it("are listed, off, and cannot be switched on", async () => {
    renderWithProviders(<PrivacySettings />);
    for (const name of [
      /Window title/,
      /Keyboard and mouse rhythm/,
      /Calendar/,
      /Cloud processing/,
    ]) {
      const box = await screen.findByRole("switch", { name });
      expect(box).not.toBeChecked();
      expect(box).toBeDisabled();
      expect(within(box.closest("label")!).getByText("Off · not available")).toBeInTheDocument();
    }
  });

  it("do not pretend to be active: clicking them does nothing", async () => {
    renderWithProviders(<PrivacySettings />);
    await userEvent.click(await screen.findByRole("switch", { name: /Cloud processing/ }));
    expect(commands.setPrivacyToggles).not.toHaveBeenCalled();
  });

  it("say what that means for your data", async () => {
    renderWithProviders(<PrivacySettings />);
    expect(await screen.findByText(/There is no cloud processing/)).toBeInTheDocument();
  });
});

// ---- retention --------------------------------------------------------------------------

describe("retention", () => {
  it("shows the current periods", async () => {
    renderWithProviders(<RetentionSettings />);
    expect(await screen.findByLabelText("Raw activity (days)")).toHaveValue(7);
    expect(screen.getByLabelText("Sessions and check-ins (days)")).toHaveValue(30);
    expect(screen.getByRole("radio", { name: /Keep indefinitely/ })).toBeChecked();
  });

  it("has nothing to save until something changes", async () => {
    renderWithProviders(<RetentionSettings />);
    await screen.findByLabelText("Raw activity (days)");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
  });

  it("saves a new period and says what it removed", async () => {
    vi.mocked(commands.setRetentionPolicy).mockResolvedValue({
      activityEventsDeleted: 12,
      sessionsDeleted: 1,
      interventionsDeleted: 0,
      pausesDeleted: 0,
      dailySummariesDeleted: 0,
    });
    renderWithProviders(<RetentionSettings />);
    const events = await screen.findByLabelText("Raw activity (days)");
    await userEvent.clear(events);
    await userEvent.type(events, "3");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(commands.setRetentionPolicy).toHaveBeenCalledWith({
      activityEventsDays: 3,
      sessionsDays: 30,
      dailySummariesDays: null,
    });
    expect(await screen.findByText("Saved. Removed 13 older records.")).toBeInTheDocument();
  });

  it("says so when nothing needed removing", async () => {
    renderWithProviders(<RetentionSettings />);
    const events = await screen.findByLabelText("Raw activity (days)");
    await userEvent.clear(events);
    await userEvent.type(events, "5");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Saved. Nothing needed removing.")).toBeInTheDocument();
  });

  it("can keep daily summaries for a chosen number of days", async () => {
    renderWithProviders(<RetentionSettings />);
    await userEvent.click(await screen.findByRole("radio", { name: /Keep for/ }));
    const days = screen.getByLabelText("Days to keep daily summaries");
    await userEvent.clear(days);
    await userEvent.type(days, "90");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(commands.setRetentionPolicy).toHaveBeenCalledWith({
      activityEventsDays: 7,
      sessionsDays: 30,
      dailySummariesDays: 90,
    });
  });

  it("can go back to keeping summaries indefinitely", async () => {
    vi.mocked(commands.getRetentionPolicy).mockResolvedValue({
      activityEventsDays: 7,
      sessionsDays: 30,
      dailySummariesDays: 90,
    });
    renderWithProviders(<RetentionSettings />);
    await userEvent.click(await screen.findByRole("radio", { name: /Keep indefinitely/ }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(commands.setRetentionPolicy).toHaveBeenCalledWith(
      expect.objectContaining({ dailySummariesDays: null }),
    );
  });

  it.each(["0", "", "2.5", "-3"])("refuses %j and says why", async (value) => {
    renderWithProviders(<RetentionSettings />);
    const events = await screen.findByLabelText("Raw activity (days)");
    await userEvent.clear(events);
    if (value) await userEvent.type(events, value);
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("whole number of days, at least 1");
  });

  it("says that a change takes effect at once", async () => {
    renderWithProviders(<RetentionSettings />);
    expect(await screen.findByText(/applies right away/)).toBeInTheDocument();
  });
});

// ---- your data --------------------------------------------------------------------------

const stored: DataOverview = {
  activityEvents: {
    count: 1204,
    oldest: "2026-03-01T08:00:00.000Z",
    newest: "2026-03-07T18:00:00.000Z",
  },
  sessions: { count: 31, oldest: "2026-02-10T09:00:00.000Z", newest: "2026-03-07T15:00:00.000Z" },
  checkIns: { count: 5, oldest: "2026-03-02T10:00:00.000Z", newest: "2026-03-02T16:00:00.000Z" },
  pauses: { count: 4, oldest: "2026-03-03T10:00:00.000Z", newest: "2026-03-05T16:00:00.000Z" },
  dailySummaries: { count: 12, oldest: "2026-02-20", newest: "2026-03-06" },
  interests: { count: 3, oldest: "2026-03-01T08:00:00.000Z", newest: "2026-03-01T09:00:00.000Z" },
  appCategoryOverrides: 2,
  databaseBytes: 1_468_006,
};

describe("your data", () => {
  it("shows what is stored, how much and from when to when", async () => {
    vi.mocked(commands.getDataOverview).mockResolvedValue(stored);
    renderWithProviders(<YourData />);
    const table = within(await screen.findByRole("table"));
    const raw = table.getByRole("row", { name: /Raw activity/ });
    expect(raw).toHaveTextContent("1204");
    expect(raw).toHaveTextContent("2026-03-01 to 2026-03-07");
    expect(table.getByRole("row", { name: /Sessions/ })).toHaveTextContent("31");
    expect(table.getByRole("row", { name: /Interests/ })).toHaveTextContent("3");
    expect(table.getByRole("row", { name: /App categories you chose/ })).toHaveTextContent("2");
    expect(table.getByRole("row", { name: /Size on disk/ })).toHaveTextContent("1.4 MB");
  });

  it("shows a dash instead of dates when there is nothing", async () => {
    renderWithProviders(<YourData />);
    const row = within(await screen.findByRole("table")).getByRole("row", { name: /Raw activity/ });
    expect(row).toHaveTextContent("0");
    expect(row).toHaveTextContent("—");
  });

  it("formats sizes", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(90_112)).toBe("88 KB");
    expect(formatBytes(1_468_006)).toBe("1.4 MB");
  });
});

describe("looking at the raw activity", () => {
  it("is hidden until asked for, and does not fetch it before", async () => {
    renderWithProviders(<YourData />);
    const button = await screen.findByRole("button", { name: "Show recent activity" });
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(commands.listRecentActivityEvents).not.toHaveBeenCalled();
  });

  it("lists exactly what is stored, in words", async () => {
    vi.mocked(commands.listRecentActivityEvents).mockResolvedValue([
      {
        type: "active_application",
        timestamp: "2026-03-07T18:00:00.000Z",
        bundleId: "com.microsoft.VSCode",
        applicationName: "Code",
      },
      {
        type: "application_switch",
        timestamp: "2026-03-07T17:50:00.000Z",
        fromBundleId: "com.microsoft.VSCode",
        toBundleId: "com.apple.Terminal",
      },
      { type: "idle", timestamp: "2026-03-07T17:00:00.000Z", seconds: 300 },
    ]);
    renderWithProviders(<YourData />);
    await userEvent.click(await screen.findByRole("button", { name: "Show recent activity" }));

    const list = within(await screen.findByRole("list", { name: "Recent raw activity" }));
    const items = list.getAllByRole("listitem");
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveTextContent("Code in front");
    expect(items[1]).toHaveTextContent("Switched from com.microsoft.VSCode to com.apple.Terminal");
    expect(items[2]).toHaveTextContent("No input for 5m");
    expect(screen.getByRole("button", { name: "Hide recent activity" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("makes the point that nothing else is kept", async () => {
    renderWithProviders(<YourData />);
    await userEvent.click(await screen.findByRole("button", { name: "Show recent activity" }));
    expect(
      await screen.findByText(/app names, switches and how long you were away/),
    ).toBeInTheDocument();
  });

  it("says when there is nothing stored", async () => {
    renderWithProviders(<YourData />);
    await userEvent.click(await screen.findByRole("button", { name: "Show recent activity" }));
    expect(await screen.findByText("No raw activity is stored.")).toBeInTheDocument();
  });

  it("describes each kind of event", () => {
    expect(describeEvent({ type: "idle", timestamp: "t", seconds: 90 })).toBe("No input for 2m");
    expect(
      describeEvent({
        type: "active_application",
        timestamp: "t",
        bundleId: "b",
        applicationName: "Notes",
      }),
    ).toBe("Notes in front");
  });
});

describe("deleting", () => {
  it("asks before deleting raw activity, and can be cancelled", async () => {
    renderWithProviders(<YourData />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete raw activity…" }));
    const dialog = screen.getByRole("alertdialog");
    expect(dialog).toHaveTextContent("Sessions, summaries, check-ins and interests stay");
    expect(commands.deleteRawData).not.toHaveBeenCalled();

    await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(commands.deleteRawData).not.toHaveBeenCalled();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("deletes raw activity once confirmed and says how much", async () => {
    vi.mocked(commands.deleteRawData).mockResolvedValue(1204);
    renderWithProviders(<YourData />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete raw activity…" }));
    await userEvent.click(screen.getByRole("button", { name: "Yes, delete raw activity" }));
    expect(commands.deleteRawData).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Deleted 1204 raw events.")).toBeInTheDocument();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("asks before deleting everything, and spells out what goes", async () => {
    renderWithProviders(<YourData />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete all local data…" }));
    const dialog = screen.getByRole("alertdialog");
    for (const thing of [
      "activity",
      "sessions",
      "check-ins",
      "summaries",
      "interests",
      "settings",
    ]) {
      expect(dialog).toHaveTextContent(thing);
    }
    expect(dialog).toHaveTextContent("can't be undone");
    expect(dialog).toHaveTextContent("Reduce motion choice is kept");
    expect(commands.deleteAllLocalData).not.toHaveBeenCalled();
  });

  it("deletes everything once confirmed, and refreshes what is on screen", async () => {
    vi.mocked(commands.getDataOverview).mockResolvedValue(stored);
    renderWithProviders(<YourData />);
    await screen.findByRole("table");
    const before = vi.mocked(commands.getDataOverview).mock.calls.length;

    vi.mocked(commands.getDataOverview).mockResolvedValue(emptyOverview);
    await userEvent.click(screen.getByRole("button", { name: "Delete all local data…" }));
    await userEvent.click(screen.getByRole("button", { name: "Yes, delete everything" }));

    expect(commands.deleteAllLocalData).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Everything was deleted.")).toBeInTheDocument();
    await vi.waitFor(() =>
      expect(vi.mocked(commands.getDataOverview).mock.calls.length).toBeGreaterThan(before),
    );
    const row = within(await screen.findByRole("table")).getByRole("row", { name: /Raw activity/ });
    await vi.waitFor(() => expect(row).toHaveTextContent("—"));
  });

  it("says nothing changed when deleting failed", async () => {
    vi.mocked(commands.deleteAllLocalData).mockRejectedValue(new Error("locked"));
    renderWithProviders(<YourData />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete all local data…" }));
    await userEvent.click(screen.getByRole("button", { name: "Yes, delete everything" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "That didn't work. Nothing was changed.",
    );
  });

  it("moves focus to the safe choice when it asks, and back to the button when it is dismissed", async () => {
    renderWithProviders(<YourData />);
    const trigger = await screen.findByRole("button", { name: "Delete raw activity…" });
    trigger.focus();
    await userEvent.keyboard("{Enter}");

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();

    await userEvent.keyboard("{Enter}");
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
    expect(commands.deleteRawData).not.toHaveBeenCalled();
  });

  it("Escape backs out of the question", async () => {
    renderWithProviders(<YourData />);
    const trigger = await screen.findByRole("button", { name: "Delete all local data…" });
    await userEvent.click(trigger);
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
    expect(commands.deleteAllLocalData).not.toHaveBeenCalled();
  });

  it("is never one accidental keypress from deleting", async () => {
    renderWithProviders(<YourData />);
    const trigger = await screen.findByRole("button", { name: "Delete all local data…" });
    trigger.focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard("{Enter}"); // lands on the safe choice
    expect(commands.deleteAllLocalData).not.toHaveBeenCalled();
  });

  it("can be confirmed from the keyboard too", async () => {
    renderWithProviders(<YourData />);
    const trigger = await screen.findByRole("button", { name: "Delete raw activity…" });
    trigger.focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.tab({ shift: true }); // from Cancel back to the destructive button
    expect(screen.getByRole("button", { name: "Yes, delete raw activity" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(commands.deleteRawData).toHaveBeenCalledTimes(1);
  });
});

// ---- permissions and the whole screen ------------------------------------------------------

describe("system permissions", () => {
  it("says none are requested and names the ones that are not needed", () => {
    renderWithProviders(<PermissionsNote />);
    expect(screen.getByText(/asks macOS for no special permissions/)).toBeInTheDocument();
    for (const permission of [
      "Accessibility",
      "Screen Recording",
      "Input Monitoring",
      "Full Disk Access",
    ]) {
      expect(screen.getByText(new RegExp(permission))).toBeInTheDocument();
    }
  });

  it("says what it does read, and that it has no network features", () => {
    renderWithProviders(<PermissionsNote />);
    expect(screen.getByText(/which app is in front, by name only/)).toBeInTheDocument();
    expect(screen.getByText(/never sees what you type or where you click/)).toBeInTheDocument();
    expect(screen.getByText(/no network features and needs no internet/)).toBeInTheDocument();
  });
});

describe("the settings screen", () => {
  it("brings it all together in a sensible order", async () => {
    renderWithProviders(<Settings />);
    await screen.findByRole("switch", { name: /Idle detection/ });
    const headings = screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent);
    expect(headings).toEqual([
      "Privacy",
      "How long data is kept",
      "Your data",
      "System permissions",
      "Motion",
      "Apps & categories",
    ]);
  });

  it("uses no alarming or coercive wording", async () => {
    renderWithProviders(<Settings />);
    await screen.findByRole("switch", { name: /Idle detection/ });
    expect(document.body.textContent).not.toMatch(
      /danger|warning!|are you sure you want to lose|we need|required to|recommended|for your own good/i,
    );
  });
});

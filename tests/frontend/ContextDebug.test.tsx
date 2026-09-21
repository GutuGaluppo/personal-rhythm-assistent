import { screen, within } from "@testing-library/react";
import { ContextDebug, describeDecision, formatMeasure } from "@/features/debug/ContextDebug";
import * as commands from "@/lib/tauri/commands";
import { pagesFor } from "@/stores/navigation";
import type { ContextAssessment, PolicyDebug } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const assessment: ContextAssessment = {
  assessedAt: "2026-03-04T10:40:00.000Z",
  activeSignalCount: 2,
  decision: "candidate_intervention",
  signals: [
    {
      kind: "continuous_activity",
      active: true,
      evidence: {
        measured: 96,
        threshold: 90,
        unit: "minutes",
        explanation: "Active for 96 minutes in this session.",
      },
    },
    {
      kind: "insufficient_idle",
      active: true,
      evidence: {
        measured: 0.02,
        threshold: 0.05,
        unit: "ratio",
        explanation: "2 minutes without input in a 98-minute session.",
      },
    },
    {
      kind: "rapid_switching",
      active: false,
      evidence: {
        measured: 2.4,
        threshold: 30,
        unit: "per_hour",
        explanation: "4 app switches in 100 minutes.",
      },
    },
    {
      kind: "create_dominance",
      active: false,
      evidence: {
        measured: 0,
        threshold: 0.75,
        unit: "ratio",
        explanation:
          "12 minutes of activity in the last 3 days; at least 240 are needed to compare.",
      },
    },
  ],
};

const policy: PolicyDebug = {
  view: {
    silent: false,
    onFireUntil: null,
    cooldownUntil: "2026-03-04T11:00:00Z",
    shownToday: 2,
    dailyLimit: 4,
    retryPending: false,
    frequencyPromptDue: false,
  },
  decision: { kind: "silent", reason: { kind: "cooldown", until: "2026-03-04T11:00:00Z" } },
};

beforeEach(() => {
  vi.mocked(commands.getPolicyDebug).mockResolvedValue(policy);
});

describe("ContextDebug", () => {
  it("shows every signal with its status, numbers and evidence", async () => {
    vi.mocked(commands.getContextAssessment).mockResolvedValue(assessment);
    renderWithProviders(<ContextDebug />);

    const row = within(await screen.findByRole("row", { name: /Continuous activity/ }));
    expect(row.getByText("Met")).toBeInTheDocument();
    expect(row.getByText("96 min")).toBeInTheDocument();
    expect(row.getByText("90 min")).toBeInTheDocument();
    expect(row.getByText("Active for 96 minutes in this session.")).toBeInTheDocument();

    const switching = within(screen.getByRole("row", { name: /Rapid app switching/ }));
    expect(switching.getByText("Not met")).toBeInTheDocument();
    expect(switching.getByText("2.4 / h")).toBeInTheDocument();
  });

  it("states the decision and the count", async () => {
    vi.mocked(commands.getContextAssessment).mockResolvedValue(assessment);
    renderWithProviders(<ContextDebug />);
    expect(await screen.findByText(/2 of 4 signals met/)).toBeInTheDocument();
    expect(screen.getByText("Candidate")).toBeInTheDocument();
  });

  it("says observing when the evidence is not enough", async () => {
    vi.mocked(commands.getContextAssessment).mockResolvedValue({
      ...assessment,
      activeSignalCount: 1,
      decision: "observe",
    });
    renderWithProviders(<ContextDebug />);
    expect(await screen.findByText("Observing")).toBeInTheDocument();
  });

  it("does not use an unlabelled status: meaning is in words, not only colour", async () => {
    vi.mocked(commands.getContextAssessment).mockResolvedValue(assessment);
    renderWithProviders(<ContextDebug />);
    await screen.findByText("2 of 4 signals met", { exact: false });
    expect(screen.getAllByText(/^(Met|Not met)$/)).toHaveLength(4);
  });
});

describe("formatMeasure", () => {
  it("formats each unit", () => {
    expect(formatMeasure(96.4, "minutes")).toBe("96 min");
    expect(formatMeasure(0.756, "ratio")).toBe("76%");
    expect(formatMeasure(30, "per_hour")).toBe("30.0 / h");
  });
});

describe("developer pages", () => {
  it("are offered in development only", () => {
    expect(pagesFor(true).map((p) => p.id)).toContain("context-debug");
    expect(pagesFor(false).map((p) => p.id)).not.toContain("context-debug");
  });
});

describe("policy section", () => {
  it("explains the decision and shows the state behind it", async () => {
    vi.mocked(commands.getContextAssessment).mockResolvedValue(assessment);
    renderWithProviders(<ContextDebug />);
    expect(
      await screen.findByText("Silent: cooldown until 2026-03-04T11:00:00Z."),
    ).toBeInTheDocument();
    expect(screen.getByText("2 of 4")).toBeInTheDocument();
  });

  it.each([
    [{ kind: "observe" as const }, /Observe/],
    [{ kind: "ask_checkin" as const, is_retry: false }, /^Ask a check-in\.$/],
    [{ kind: "ask_checkin" as const, is_retry: true }, /retry/],
    [{ kind: "silent" as const, reason: { kind: "silent_mode" as const } }, /silence is on/],
    [
      { kind: "silent" as const, reason: { kind: "daily_ceiling" as const, limit: 4 } },
      /limit of 4/,
    ],
    [
      { kind: "silent" as const, reason: { kind: "on_fire" as const, until: "12:00" } },
      /on fire.* until 12:00/,
    ],
  ])("describes %j", (decision, expected) => {
    expect(describeDecision(decision)).toMatch(expected);
  });
});

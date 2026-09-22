import * as commands from "@/lib/tauri/commands";
import type { DataOverview, PrivacyToggles, RetentionPolicy } from "@/types";

export const defaultToggles: PrivacyToggles = {
  activeApplication: true,
  activeTime: true,
  idleDetection: true,
  windowTitle: false,
  keyboardMouseRhythm: false,
  calendar: false,
  cloudProcessing: false,
};

export const defaultRetention: RetentionPolicy = {
  activityEventsDays: 7,
  sessionsDays: 30,
  dailySummariesDays: null,
};

const empty = { count: 0, oldest: null, newest: null };
export const emptyOverview: DataOverview = {
  activityEvents: empty,
  sessions: empty,
  checkIns: empty,
  pauses: empty,
  dailySummaries: empty,
  interests: empty,
  appCategoryOverrides: 0,
  databaseBytes: 90_112,
};

/** Every command the Settings screen calls, with calm defaults. Call inside beforeEach. */
export function mockSettingsCommands() {
  vi.mocked(commands.getPrivacyToggles).mockResolvedValue(defaultToggles);
  vi.mocked(commands.setPrivacyToggles).mockImplementation(async (t) => t);
  vi.mocked(commands.getRetentionPolicy).mockResolvedValue(defaultRetention);
  vi.mocked(commands.setRetentionPolicy).mockResolvedValue({
    activityEventsDeleted: 0,
    sessionsDeleted: 0,
    interventionsDeleted: 0,
    pausesDeleted: 0,
    dailySummariesDeleted: 0,
  });
  vi.mocked(commands.getDataOverview).mockResolvedValue(emptyOverview);
  vi.mocked(commands.listRecentActivityEvents).mockResolvedValue([]);
  vi.mocked(commands.deleteRawData).mockResolvedValue(0);
  vi.mocked(commands.deleteAllLocalData).mockResolvedValue();
  vi.mocked(commands.listAppMappings).mockResolvedValue([]);
}

import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AppCategories } from "@/features/settings/AppCategories";
import * as commands from "@/lib/tauri/commands";
import type { AppMapping } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const apps: AppMapping[] = [
  {
    bundleId: "org.mozilla.firefox",
    applicationName: "Firefox",
    category: "Unknown",
    source: "unset",
  },
  {
    bundleId: "com.microsoft.VSCode",
    applicationName: "Code",
    category: "Create",
    source: "default",
  },
  { bundleId: "com.apple.Notes", applicationName: "Notes", category: "Think", source: "user" },
];

beforeEach(() => {
  vi.mocked(commands.listAppMappings).mockResolvedValue(apps);
  vi.mocked(commands.setAppCategory).mockResolvedValue();
  vi.mocked(commands.resetAppCategory).mockResolvedValue();
});

describe("AppCategories", () => {
  it("lists each app with its current category and where it came from", async () => {
    renderWithProviders(<AppCategories />);
    expect(await screen.findByLabelText("Category for Firefox")).toHaveValue("Unknown");
    expect(screen.getByLabelText("Category for Code")).toHaveValue("Create");
    expect(screen.getByLabelText("Category for Notes")).toHaveValue("Think");
    expect(screen.getByText(/org.mozilla.firefox · Not set/)).toBeInTheDocument();
    expect(screen.getByText(/com.microsoft.VSCode · Default/)).toBeInTheDocument();
    expect(screen.getByText(/com.apple.Notes · Your choice/)).toBeInTheDocument();
  });

  it("remaps an app when the user picks another category", async () => {
    renderWithProviders(<AppCategories />);
    await userEvent.selectOptions(await screen.findByLabelText("Category for Firefox"), "Explore");
    expect(commands.setAppCategory).toHaveBeenCalledWith("org.mozilla.firefox", "Explore");
  });

  it("refreshes the list after a change", async () => {
    renderWithProviders(<AppCategories />);
    await userEvent.selectOptions(await screen.findByLabelText("Category for Firefox"), "Explore");
    await vi.waitFor(() => expect(commands.listAppMappings).toHaveBeenCalledTimes(2));
  });

  it("offers Reset only for the user's own choices", async () => {
    renderWithProviders(<AppCategories />);
    await screen.findByLabelText("Category for Notes");
    expect(screen.getAllByRole("button", { name: /reset/i })).toHaveLength(1);

    await userEvent.click(screen.getByRole("button", { name: "Reset Notes to default" }));
    expect(commands.resetAppCategory).toHaveBeenCalledWith("com.apple.Notes");
  });

  it("is reachable and operable without a pointer", async () => {
    renderWithProviders(<AppCategories />);
    const firefox = await screen.findByLabelText("Category for Firefox");
    await userEvent.tab();
    expect(firefox).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByLabelText("Category for Code")).toHaveFocus();
    // Tab order continues through the Reset button of the user's own choice.
    await userEvent.tab();
    expect(screen.getByLabelText("Category for Notes")).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Reset Notes to default" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(commands.resetAppCategory).toHaveBeenCalledWith("com.apple.Notes");
  });

  it("explains the empty state", async () => {
    vi.mocked(commands.listAppMappings).mockResolvedValue([]);
    renderWithProviders(<AppCategories />);
    expect(await screen.findByText(/show up here once they've been seen/)).toBeInTheDocument();
  });
});

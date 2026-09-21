import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "@/app/App";
import * as commands from "@/lib/tauri/commands";
import { useNavigation } from "@/stores/navigation";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

beforeEach(() => {
  useNavigation.setState({ page: "my-day" });
  vi.mocked(commands.getMyDay).mockResolvedValue({
    trackingEnabled: true,
    state: "active",
    frontmostApplication: null,
    currentSession: null,
    activeMinutesToday: 0,
    contextSwitchesToday: 0,
    lastBreak: null,
  });
  vi.mocked(commands.listAppMappings).mockResolvedValue([]);
  vi.mocked(commands.getInterestSuggestion).mockResolvedValue(null);
});

describe("App", () => {
  it("opens on My Day", async () => {
    renderWithProviders(<App />);
    expect(await screen.findByRole("heading", { name: "My Day", level: 1 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "My Day" })).toHaveAttribute("aria-current", "page");
  });

  it("navigates with the keyboard-reachable menu", async () => {
    renderWithProviders(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByRole("heading", { name: "Settings", level: 1 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Settings" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByRole("button", { name: "My Day" })).not.toHaveAttribute("aria-current");
  });

  it("only offers pages that exist (plus the developer view in development)", () => {
    renderWithProviders(<App />);
    const nav = within(screen.getByRole("navigation", { name: "Main" }));
    const labels = nav.getAllByRole("button").map((b) => b.textContent);
    expect(labels).toEqual(["My Day", "Interest Inbox", "Settings", "Context (developer)"]);
  });
});

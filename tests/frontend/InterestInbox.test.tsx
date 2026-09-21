import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { InterestInbox } from "@/features/interest-inbox/InterestInbox";
import { InterestSuggestion } from "@/features/interest-inbox/InterestSuggestion";
import * as commands from "@/lib/tauri/commands";
import { useNavigation } from "@/stores/navigation";
import type { Interest } from "@/types";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

const webgpu: Interest = {
  id: 1,
  text: "Learn WebGPU",
  createdAt: "2026-03-02T09:00:00.000Z",
  archivedAt: null,
};
const bike: Interest = {
  id: 2,
  text: "Restore bicycle frame",
  createdAt: "2026-03-03T09:00:00.000Z",
  archivedAt: null,
};
const old: Interest = {
  id: 3,
  text: "Read about granular synthesis",
  createdAt: "2026-02-01T09:00:00.000Z",
  archivedAt: "2026-02-10T09:00:00.000Z",
};

beforeEach(() => {
  vi.mocked(commands.listInterests).mockImplementation(async (archived) =>
    archived ? [old] : [bike, webgpu],
  );
  vi.mocked(commands.addInterest).mockImplementation(async (text) => ({
    id: 99,
    text,
    createdAt: "2026-03-04T09:00:00.000Z",
    archivedAt: null,
  }));
  vi.mocked(commands.archiveInterest).mockResolvedValue(true);
  vi.mocked(commands.restoreInterest).mockResolvedValue(true);
  vi.mocked(commands.deleteInterest).mockResolvedValue(true);
});

describe("capturing", () => {
  it("is ready to type the moment the page opens", async () => {
    renderWithProviders(<InterestInbox />);
    expect(await screen.findByLabelText("Add an interest")).toHaveFocus();
  });

  it("takes a thought with typing and one Enter, nothing else", async () => {
    renderWithProviders(<InterestInbox />);
    await screen.findByRole("list", { name: "Interests" });
    await userEvent.keyboard("Learn granular synthesis{Enter}");
    expect(commands.addInterest).toHaveBeenCalledTimes(1);
    expect(commands.addInterest).toHaveBeenCalledWith("Learn granular synthesis");
  });

  it("clears the field and keeps the focus for the next thought", async () => {
    renderWithProviders(<InterestInbox />);
    const input = await screen.findByLabelText("Add an interest");
    await userEvent.type(input, "One{Enter}");
    await vi.waitFor(() => expect(input).toHaveValue(""));
    expect(input).toHaveFocus();
  });

  it("refreshes the list after adding", async () => {
    renderWithProviders(<InterestInbox />);
    await screen.findByRole("list", { name: "Interests" });
    await userEvent.keyboard("One{Enter}");
    await vi.waitFor(() =>
      expect(
        vi.mocked(commands.listInterests).mock.calls.filter(([a]) => a === false).length,
      ).toBeGreaterThan(1),
    );
  });

  it("has an Add button too", async () => {
    renderWithProviders(<InterestInbox />);
    await userEvent.type(await screen.findByLabelText("Add an interest"), "Two");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(commands.addInterest).toHaveBeenCalledWith("Two");
  });

  it("ignores an empty submission", async () => {
    renderWithProviders(<InterestInbox />);
    await screen.findByRole("list", { name: "Interests" });
    expect(screen.getByRole("button", { name: "Add" })).toBeDisabled();
    await userEvent.keyboard("   {Enter}");
    expect(commands.addInterest).not.toHaveBeenCalled();
  });

  it("limits the length of a note", async () => {
    renderWithProviders(<InterestInbox />);
    expect(await screen.findByLabelText("Add an interest")).toHaveAttribute("maxlength", "280");
  });

  it("keeps what was typed if saving fails, and says so plainly", async () => {
    vi.mocked(commands.addInterest).mockRejectedValue(new Error("disk full"));
    renderWithProviders(<InterestInbox />);
    const input = await screen.findByLabelText("Add an interest");
    await userEvent.type(input, "Keep me{Enter}");
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't save that");
    expect(input).toHaveValue("Keep me");
  });
});

describe("the list", () => {
  it("shows the newest first, with when each was added", async () => {
    renderWithProviders(<InterestInbox />);
    const list = await screen.findByRole("list", { name: "Interests" });
    const items = within(list).getAllByRole("listitem");
    expect(items[0]).toHaveTextContent("Restore bicycle frame");
    expect(items[1]).toHaveTextContent("Learn WebGPU");
    expect(within(items[0]).getByText(/Added/)).toHaveAttribute("datetime", bike.createdAt);
  });

  it("is calm when empty", async () => {
    vi.mocked(commands.listInterests).mockResolvedValue([]);
    renderWithProviders(<InterestInbox />);
    expect(await screen.findByText(/Nothing here yet/)).toBeInTheDocument();
  });

  it("archives with one click", async () => {
    renderWithProviders(<InterestInbox />);
    await userEvent.click(await screen.findByRole("button", { name: "Archive Learn WebGPU" }));
    expect(commands.archiveInterest).toHaveBeenCalledWith(1);
  });

  it("never scores, ranks or nags", async () => {
    renderWithProviders(<InterestInbox />);
    await screen.findByRole("list", { name: "Interests" });
    expect(document.body.textContent).not.toMatch(/score|streak|priority|overdue|goal|points/i);
  });
});

describe("deleting", () => {
  it("asks first, because it is permanent", async () => {
    renderWithProviders(<InterestInbox />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete Learn WebGPU" }));
    expect(commands.deleteInterest).not.toHaveBeenCalled();
    expect(screen.getByText("Delete permanently?")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Yes, delete" }));
    expect(commands.deleteInterest).toHaveBeenCalledWith(1);
  });

  it("can be cancelled", async () => {
    renderWithProviders(<InterestInbox />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete Learn WebGPU" }));
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(commands.deleteInterest).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Archive Learn WebGPU" })).toBeInTheDocument();
  });
});

describe("archived", () => {
  it("are hidden until asked for", async () => {
    renderWithProviders(<InterestInbox />);
    await screen.findByRole("list", { name: "Interests" });
    expect(screen.queryByText("Read about granular synthesis")).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("checkbox", { name: "Show archived" }));
    const archived = await screen.findByRole("list", { name: "Archived interests" });
    expect(within(archived).getByText("Read about granular synthesis")).toBeInTheDocument();
  });

  it("can be restored or deleted", async () => {
    renderWithProviders(<InterestInbox />);
    await userEvent.click(await screen.findByRole("checkbox", { name: "Show archived" }));
    await userEvent.click(
      await screen.findByRole("button", { name: "Restore Read about granular synthesis" }),
    );
    expect(commands.restoreInterest).toHaveBeenCalledWith(3);

    await userEvent.click(
      screen.getByRole("button", { name: "Delete Read about granular synthesis" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Yes, delete" }));
    expect(commands.deleteInterest).toHaveBeenCalledWith(3);
  });
});

describe("the day's suggestion", () => {
  beforeEach(() => useNavigation.setState({ page: "my-day" }));

  it("shows the interest of the day", async () => {
    vi.mocked(commands.getInterestSuggestion).mockResolvedValue(webgpu);
    renderWithProviders(<InterestSuggestion />);
    const card = await screen.findByRole("region", { name: "Something you wanted to explore" });
    expect(within(card).getByText("Learn WebGPU")).toBeInTheDocument();
  });

  it("shows nothing at all when there is nothing to suggest", async () => {
    vi.mocked(commands.getInterestSuggestion).mockResolvedValue(null);
    renderWithProviders(<InterestSuggestion />);
    await vi.waitFor(() => expect(commands.getInterestSuggestion).toHaveBeenCalled());
    expect(screen.queryByRole("region")).not.toBeInTheDocument();
  });

  it("can be archived from there", async () => {
    vi.mocked(commands.getInterestSuggestion).mockResolvedValue(webgpu);
    renderWithProviders(<InterestSuggestion />);
    await userEvent.click(await screen.findByRole("button", { name: "Archive Learn WebGPU" }));
    expect(commands.archiveInterest).toHaveBeenCalledWith(1);
  });

  it("leads to the inbox", async () => {
    vi.mocked(commands.getInterestSuggestion).mockResolvedValue(webgpu);
    renderWithProviders(<InterestSuggestion />);
    await userEvent.click(await screen.findByRole("button", { name: "Open Interest Inbox" }));
    expect(useNavigation.getState().page).toBe("interest-inbox");
  });
});

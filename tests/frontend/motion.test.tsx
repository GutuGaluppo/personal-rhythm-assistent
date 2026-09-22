import { act, renderHook, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Settings } from "@/features/settings/Settings";
import {
  applyMotionPreference,
  readMotionPreference,
  shouldReduceMotion,
  useReduceMotion,
  writeMotionPreference,
} from "@/lib/utils/motion";
import { mockSettingsCommands } from "./settingsMocks";
import { renderWithProviders } from "./utils";

vi.mock("@/lib/tauri/commands");

function mockSystemReducedMotion(reduced: boolean) {
  const listeners = new Set<() => void>();
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: reduced && query.includes("prefers-reduced-motion"),
    media: query,
    addEventListener: (_: string, l: () => void) => listeners.add(l),
    removeEventListener: (_: string, l: () => void) => listeners.delete(l),
  })) as unknown as typeof window.matchMedia;
  return listeners;
}

beforeEach(() => {
  window.localStorage.clear();
  delete document.documentElement.dataset.reduceMotion;
  mockSystemReducedMotion(false);
});

describe("motion preference", () => {
  it("defaults to following the system", () => {
    expect(readMotionPreference()).toBe("system");
    expect(shouldReduceMotion("system")).toBe(false);
  });

  it("the system setting alone is enough to reduce motion", () => {
    mockSystemReducedMotion(true);
    expect(shouldReduceMotion("system")).toBe(true);
  });

  it("the app setting reduces motion even when the system does not", () => {
    expect(shouldReduceMotion("reduce")).toBe(true);
  });

  it("the system setting cannot be switched off from inside the app", () => {
    mockSystemReducedMotion(true);
    writeMotionPreference("system");
    expect(shouldReduceMotion(readMotionPreference())).toBe(true);
  });

  it("is saved and shared through storage", () => {
    writeMotionPreference("reduce");
    expect(window.localStorage.getItem("reduceMotion")).toBe("reduce");
    expect(readMotionPreference()).toBe("reduce");
  });

  it("lets plain CSS follow the same decision", () => {
    applyMotionPreference();
    expect(document.documentElement.dataset.reduceMotion).toBe("false");
    writeMotionPreference("reduce");
    expect(document.documentElement.dataset.reduceMotion).toBe("true");
  });

  it("survives storage being unavailable", () => {
    const getItem = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(readMotionPreference()).toBe("system");
    expect(() => writeMotionPreference("reduce")).not.toThrow();
    getItem.mockRestore();
    setItem.mockRestore();
  });

  it("useReduceMotion follows changes without a reload", () => {
    const { result } = renderHook(() => useReduceMotion());
    expect(result.current).toBe(false);
    act(() => writeMotionPreference("reduce"));
    expect(result.current).toBe(true);
    act(() => writeMotionPreference("system"));
    expect(result.current).toBe(false);
  });

  it("useReduceMotion reacts when the system setting changes", () => {
    const listeners = mockSystemReducedMotion(false);
    const { result } = renderHook(() => useReduceMotion());
    expect(result.current).toBe(false);
    mockSystemReducedMotion(true);
    act(() => listeners.forEach((l) => l()));
    expect(result.current).toBe(true);
  });
});

describe("the Reduce motion setting", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("is a labelled switch, off by default", () => {
    renderWithProviders(<Settings />);
    expect(screen.getByRole("checkbox", { name: /reduce motion/i })).not.toBeChecked();
  });

  it("turns Reduce Motion on and off", async () => {
    renderWithProviders(<Settings />);
    const box = screen.getByRole("checkbox", { name: /reduce motion/i });

    await userEvent.click(box);
    expect(box).toBeChecked();
    expect(readMotionPreference()).toBe("reduce");
    expect(document.documentElement.dataset.reduceMotion).toBe("true");

    await userEvent.click(box);
    expect(readMotionPreference()).toBe("system");
  });

  it("reflects a saved preference", () => {
    writeMotionPreference("reduce");
    renderWithProviders(<Settings />);
    expect(screen.getByRole("checkbox", { name: /reduce motion/i })).toBeChecked();
  });

  it("says the system setting is always respected", () => {
    renderWithProviders(<Settings />);
    expect(
      screen.getByText(/system's Reduce Motion setting is always respected/i),
    ).toBeInTheDocument();
  });
});

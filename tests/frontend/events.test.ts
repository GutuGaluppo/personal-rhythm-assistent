import { listen } from "@tauri-apps/api/event";
import { onNavigate } from "@/lib/tauri/events";

vi.mock("@tauri-apps/api/event");

describe("onNavigate", () => {
  it("passes the requested page to the handler", async () => {
    vi.mocked(listen).mockImplementation(async (_event, cb) => {
      (cb as (e: { payload: string }) => void)({ payload: "interest-inbox" });
      return () => {};
    });
    const handler = vi.fn();
    await onNavigate(handler);
    expect(listen).toHaveBeenCalledWith("navigate", expect.any(Function));
    expect(handler).toHaveBeenCalledWith("interest-inbox");
  });

  it("is harmless outside the desktop shell", async () => {
    vi.mocked(listen).mockRejectedValue(new Error("no tauri here"));
    const stop = await onNavigate(vi.fn());
    expect(() => stop()).not.toThrow();
  });
});

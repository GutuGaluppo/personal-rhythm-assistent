import { formatAgo, formatDuration } from "@/lib/utils/format";

describe("formatDuration", () => {
  it.each([
    [0, "0m"],
    [0.4, "0m"],
    [45, "45m"],
    [59.6, "1h"],
    [60, "1h"],
    [92, "1h 32m"],
    [125, "2h 5m"],
    [-5, "0m"],
  ])("%s minutes -> %s", (minutes, expected) => {
    expect(formatDuration(minutes)).toBe(expected);
  });
});

describe("formatAgo", () => {
  it("says 'just now' under a minute", () => {
    expect(formatAgo(0.3)).toBe("just now");
  });
  it("appends 'ago'", () => {
    expect(formatAgo(42)).toBe("42m ago");
    expect(formatAgo(65)).toBe("1h 5m ago");
  });
});

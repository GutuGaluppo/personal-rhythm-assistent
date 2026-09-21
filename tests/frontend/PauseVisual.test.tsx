import { render, screen, waitFor } from "@testing-library/react";
import { PauseVisual } from "@/components/pause/PauseVisual";

function layer(container: HTMLElement, name: "bars" | "eyes") {
  return container.querySelector(`[data-layer="${name}"]`) as SVGGElement | null;
}
const opacity = (el: Element | null) => Number(getComputedStyle(el as Element).opacity || "1");

describe("PauseVisual: full motion", () => {
  it("morphs (the shapes change continuously)", () => {
    render(<PauseVisual settled={false} reduceMotion={false} />);
    expect(screen.getByRole("img")).toHaveAttribute("data-motion", "morph");
  });

  it("draws two shapes, the pause bars, before it settles", () => {
    const { container } = render(<PauseVisual settled={false} reduceMotion={false} />);
    expect(container.querySelectorAll("path")).toHaveLength(2);
    expect(layer(container, "bars")).toBeNull();
  });

  it("names its state in words", () => {
    const { rerender } = render(<PauseVisual settled={false} reduceMotion={false} />);
    expect(screen.getByRole("img", { name: "Pause" })).toBeInTheDocument();
    rerender(<PauseVisual settled reduceMotion={false} />);
    expect(screen.getByRole("img", { name: "Resting" })).toBeInTheDocument();
  });
});

describe("PauseVisual: Reduce Motion", () => {
  it("uses a crossfade, not a transformation", () => {
    render(<PauseVisual settled={false} reduceMotion />);
    expect(screen.getByRole("img")).toHaveAttribute("data-motion", "crossfade");
  });

  it("holds both pictures and fades between them", async () => {
    const { container, rerender } = render(<PauseVisual settled={false} reduceMotion />);
    expect(layer(container, "bars")).not.toBeNull();
    expect(layer(container, "eyes")).not.toBeNull();
    await waitFor(() => expect(opacity(layer(container, "bars"))).toBe(1));
    await waitFor(() => expect(opacity(layer(container, "eyes"))).toBe(0));

    rerender(<PauseVisual settled reduceMotion />);
    await waitFor(() => expect(opacity(layer(container, "bars"))).toBe(0));
    await waitFor(() => expect(opacity(layer(container, "eyes"))).toBe(1));
  });

  it("never moves, scales or loops anything", () => {
    const { container, rerender } = render(<PauseVisual settled={false} reduceMotion />);
    rerender(<PauseVisual settled reduceMotion />);
    const moving = Array.from(container.querySelectorAll<SVGElement>("*")).filter((el) =>
      /transform|scale|translate|rotate/.test(el.getAttribute("style") ?? ""),
    );
    expect(moving).toEqual([]);
    // The glow is a plain, still circle.
    expect(container.querySelector("circle")?.getAttribute("style") ?? "").not.toMatch(/transform/);
  });

  it("names its state in words", () => {
    const { rerender } = render(<PauseVisual settled={false} reduceMotion />);
    expect(screen.getByRole("img", { name: "Pause" })).toBeInTheDocument();
    rerender(<PauseVisual settled reduceMotion />);
    expect(screen.getByRole("img", { name: "Resting" })).toBeInTheDocument();
  });
});

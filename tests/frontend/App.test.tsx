import { render, screen } from "@testing-library/react";
import { App } from "@/app/App";

describe("App", () => {
  it("renders the main window heading", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: /personal rhythm assistant/i })).toBeInTheDocument();
  });
});

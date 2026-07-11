import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Button } from "./button";

describe("TR-07-001 ShadCN baseline component", () => {
  it("renders a ShadCN Button with variant/size classes", () => {
    render(<Button variant="destructive" size="lg">Delete</Button>);
    const btn = screen.getByRole("button", { name: "Delete" });
    expect(btn).toBeInTheDocument();
    expect(btn.className).toContain("bg-destructive");
    expect(btn.className).toContain("h-10");
  });

  it("supports asChild composition (Radix Slot)", () => {
    render(
      <Button asChild>
        <a href="/x">Link button</a>
      </Button>,
    );
    const link = screen.getByRole("link", { name: "Link button" });
    expect(link).toHaveClass("inline-flex");
  });
});

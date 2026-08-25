import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CompactTab } from "./CompactTab";

describe("Compact Tab workspace entry", () => {
  it("offers only Desktop and Reader transitions without dashboard content", () => {
    const onDesktop = vi.fn();
    const onOpenReader = vi.fn();
    render(<CompactTab onDesktop={onDesktop} onOpenReader={onOpenReader} />);

    expect(screen.getByRole("main", { name: "Agent Desk Compact Tab" })).toBeVisible();
    expect(screen.queryByText("Inbox")).not.toBeInTheDocument();
    expect(screen.queryByText("Todo")).not.toBeInTheDocument();
    expect(screen.queryByText("Reading")).not.toBeInTheDocument();

    const expand = screen.getByRole("button", { name: "展开 Compact Tab" });
    expect(expand).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(expand);
    expect(expand).toHaveAttribute("aria-expanded", "true");

    fireEvent.click(screen.getByRole("button", { name: "打开阅读器" }));
    expect(onOpenReader).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "缩小到桌面便利贴" }));
    expect(onDesktop).toHaveBeenCalledOnce();
  });
});

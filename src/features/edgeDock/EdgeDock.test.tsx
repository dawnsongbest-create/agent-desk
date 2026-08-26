import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DesktopStickyWindowPort } from "../../application/ports/desktopSticky";
import { defaultPreferences } from "../../domain/preferences";
import { EdgeDock } from "./EdgeDock";

function windowPort(): DesktopStickyWindowPort {
  return {
    enter: vi.fn().mockResolvedValue(defaultPreferences),
    open: vi.fn().mockResolvedValue(defaultPreferences),
    openReader: vi.fn().mockResolvedValue(defaultPreferences),
    setDesktopMode: vi.fn().mockResolvedValue(defaultPreferences),
    setReaderMode: vi.fn().mockResolvedValue(defaultPreferences),
    leaveReader: vi.fn().mockResolvedValue(defaultPreferences),
    startDragging: vi.fn().mockResolvedValue(undefined),
    dockIfNearRightEdge: vi.fn().mockResolvedValue(undefined),
    showContextMenu: vi.fn().mockResolvedValue(undefined),
    setEdgeDockExpanded: vi.fn().mockResolvedValue(undefined),
  };
}

describe("Edge Dock entry surface", () => {
  afterEach(() => vi.useRealTimers());

  it("expands on approach, retracts after leave, and exposes only Reader entry", () => {
    vi.useFakeTimers();
    const native = windowPort();
    render(<EdgeDock windowPort={native} />);
    const dock = screen.getByRole("main", { name: "Agent Desk Edge Dock" });

    fireEvent.mouseEnter(dock);
    expect(dock).toHaveAttribute("data-expanded", "true");
    expect(native.setEdgeDockExpanded).toHaveBeenCalledWith(true);
    expect(screen.getByRole("button", { name: "打开阅读器" })).toBeVisible();
    expect(screen.queryByText("Inbox")).not.toBeInTheDocument();
    expect(screen.queryByText("Todo")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "打开阅读器" }));
    expect(native.openReader).toHaveBeenCalledOnce();

    fireEvent.mouseLeave(dock);
    act(() => vi.advanceTimersByTime(260));
    expect(dock).toHaveAttribute("data-expanded", "false");
    expect(native.setEdgeDockExpanded).toHaveBeenLastCalledWith(false);
  });
});

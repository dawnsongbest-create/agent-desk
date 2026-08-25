import { createEvent, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DesktopStickyWindowPort } from "../../application/ports/desktopSticky";
import type { StickyCardsPort } from "../../application/ports/sticky";
import { defaultPreferences } from "../../domain/preferences";

const mocks = vi.hoisted(() => ({
  loadPreferences: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("../../infrastructure/tauri/preferences", () => ({
  tauriPreferences: {
    load: mocks.loadPreferences,
  },
}));

import { DesktopSticky } from "./DesktopSticky";

function dispatchPointer(
  element: Element,
  type: "pointerDown" | "pointerMove",
  values: { button: number; clientX: number; clientY: number },
) {
  const event = createEvent[type](element);
  for (const [key, value] of Object.entries(values))
    Object.defineProperty(event, key, { configurable: true, value });
  fireEvent(element, event);
}

function stickyPort(): StickyCardsPort {
  return {
    list: vi.fn().mockResolvedValue([
      {
        id: "task-1",
        kind: "task",
        text: "阅读论文",
        completed: false,
        dueDate: null,
        position: 0,
        createdAt: "2026-08-25T00:00:00Z",
        updatedAt: "2026-08-25T00:00:00Z",
      },
    ]),
    getProfile: vi.fn().mockResolvedValue({ quoteText: "保持好奇", updatedAt: "" }),
    create: vi.fn(),
    updateText: vi.fn(),
    setTaskCompleted: vi.fn(),
    setTaskDueDate: vi.fn(),
    delete: vi.fn(),
    reorder: vi.fn(),
    updateQuote: vi.fn(),
    exportRecord: vi.fn(),
  };
}

function windowPort(): DesktopStickyWindowPort {
  return {
    enter: vi.fn(),
    open: vi.fn().mockResolvedValue(defaultPreferences),
    openReader: vi.fn().mockResolvedValue(defaultPreferences),
    startDragging: vi.fn().mockResolvedValue(undefined),
    dockIfNearRightEdge: vi.fn().mockResolvedValue(undefined),
    showContextMenu: vi.fn().mockResolvedValue(undefined),
    setEdgeDockExpanded: vi.fn().mockResolvedValue(undefined),
  };
}

describe("Desktop Sticky native surface", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      stickyMode: "desktop",
      desktopStickyKind: "todo",
    });
    mocks.listen.mockResolvedValue(vi.fn());
  });

  it("renders the current durable Todo summary at the existing Mini paper size", async () => {
    render(<DesktopSticky stickyPort={stickyPort()} windowPort={windowPort()} />);
    const sticky = await screen.findByRole("button", { name: /1 件未完成待办：阅读论文/ });
    expect(sticky).toHaveTextContent("TODAY1阅读论文");
    expect(sticky).toHaveClass("mini-tab");
  });

  it("opens, edits, drags, and delegates the right-click menu without mutating data", async () => {
    const native = windowPort();
    render(<DesktopSticky stickyPort={stickyPort()} windowPort={native} />);
    const sticky = await screen.findByRole("button", { name: /桌面便利贴/ });

    fireEvent.click(sticky);
    await waitFor(() => expect(native.open).toHaveBeenCalledWith(undefined), { timeout: 700 });

    fireEvent.doubleClick(sticky);
    expect(native.open).toHaveBeenLastCalledWith("todo");

    dispatchPointer(sticky, "pointerDown", { button: 0, clientX: 10, clientY: 10 });
    dispatchPointer(sticky, "pointerMove", { button: 0, clientX: 20, clientY: 18 });
    expect(native.startDragging).toHaveBeenCalledOnce();
    await waitFor(() => expect(native.dockIfNearRightEdge).toHaveBeenCalledOnce());

    fireEvent.contextMenu(sticky);
    expect(native.showContextMenu).toHaveBeenCalledOnce();
  });
});
